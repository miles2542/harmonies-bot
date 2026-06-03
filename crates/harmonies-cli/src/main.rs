use std::{fs, io::Read, path::PathBuf};

use anyhow::Context;
use harmonies_core::{
    advise, bga::normalize_gamedatas, scoring::score_player, AdvisorRequestV1, CardCatalog,
    EvalWeights, GameSnapshotV1,
};
use serde::Serialize;
use serde_json::Value;

fn main() -> anyhow::Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if matches!(args.first().map(String::as_str), Some("normalize")) {
        args.remove(0);
        return normalize_command(args);
    }
    if matches!(args.first().map(String::as_str), Some("score")) {
        args.remove(0);
        return score_command(args);
    }

    let mut args = args.into_iter();
    let request_path = args.next().map(PathBuf::from);
    let catalog_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("docs/cards_database.json"));
    let weights_path = args.next().map(PathBuf::from);

    let mut input = String::new();
    if let Some(path) = request_path {
        input = fs::read_to_string(&path)
            .with_context(|| format!("failed to read request file {}", path.display()))?;
    } else {
        std::io::stdin()
            .read_to_string(&mut input)
            .context("failed to read stdin")?;
    }

    let mut request: AdvisorRequestV1 =
        serde_json::from_str(&input).context("failed to parse AdvisorRequestV1")?;
    let catalog_json = fs::read_to_string(&catalog_path)
        .with_context(|| format!("failed to read catalog {}", catalog_path.display()))?;
    request.catalog =
        CardCatalog::from_cards_database_json(&catalog_json).context("failed to parse catalog")?;
    if let Some(path) = weights_path {
        request.weights = load_weights(&path)?;
    }

    let response = advise(request);
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}

fn normalize_command(args: Vec<String>) -> anyhow::Result<()> {
    let path = args
        .first()
        .map(PathBuf::from)
        .context("usage: harmonies-cli normalize <snapshot.json> [perspectivePlayerId]")?;
    let perspective = args.get(1).map(String::as_str);
    let input =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let raw: Value = serde_json::from_str(&input).context("failed to parse raw JSON")?;
    let gamedatas = raw.get("gamedatas").unwrap_or(&raw);
    let snapshot =
        normalize_gamedatas(gamedatas, perspective).context("failed to normalize BGA")?;
    println!("{}", serde_json::to_string_pretty(&snapshot)?);
    Ok(())
}

fn score_command(args: Vec<String>) -> anyhow::Result<()> {
    let path = args.first().map(PathBuf::from).context(
        "usage: harmonies-cli score <snapshot.json> [--perspective ID] [--catalog PATH]",
    )?;
    let options = ScoreCommandOptions::parse(&args[1..])?;
    let input =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let raw: Value = serde_json::from_str(&input).context("failed to parse snapshot JSON")?;
    let snapshot = snapshot_from_value(raw, options.perspective.as_deref())?;
    let catalog = load_catalog(&options.catalog_path)?;
    let players = snapshot
        .players
        .iter()
        .map(|player| {
            let breakdown = score_player(player, snapshot.board_side, &catalog);
            PlayerScoreReport {
                player_id: player.player_id.clone(),
                total: breakdown.total(),
                breakdown,
            }
        })
        .collect();
    let report = ScoreReport {
        schema_version: snapshot.schema_version,
        perspective_player_id: snapshot.perspective_player_id,
        active_player_id: snapshot.active_player_id,
        board_side: snapshot.board_side,
        players,
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

#[derive(Debug)]
struct ScoreCommandOptions {
    perspective: Option<String>,
    catalog_path: PathBuf,
}

impl ScoreCommandOptions {
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut perspective = None;
        let mut catalog_path = PathBuf::from("docs/cards_database.json");
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--perspective" => {
                    perspective = Some(
                        args.get(index + 1)
                            .cloned()
                            .context("--perspective requires an id")?,
                    );
                    index += 2;
                }
                "--catalog" => {
                    catalog_path = args
                        .get(index + 1)
                        .map(PathBuf::from)
                        .context("--catalog requires a path")?;
                    index += 2;
                }
                other => anyhow::bail!("unknown score option: {other}"),
            }
        }
        Ok(Self {
            perspective,
            catalog_path,
        })
    }
}

fn snapshot_from_value(raw: Value, perspective: Option<&str>) -> anyhow::Result<GameSnapshotV1> {
    if raw.get("schemaVersion").is_some() {
        return serde_json::from_value(raw).context("failed to parse normalized GameSnapshotV1");
    }
    let gamedatas = raw.get("gamedatas").unwrap_or(&raw);
    normalize_gamedatas(gamedatas, perspective).context("failed to normalize BGA")
}

fn load_catalog(path: &PathBuf) -> anyhow::Result<CardCatalog> {
    let input =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    CardCatalog::from_cards_database_json(&input).context("failed to parse catalog")
}

fn load_weights(path: &PathBuf) -> anyhow::Result<EvalWeights> {
    let input =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&input).context("failed to parse weights")
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScoreReport {
    schema_version: u8,
    perspective_player_id: String,
    active_player_id: String,
    board_side: harmonies_core::BoardSide,
    players: Vec<PlayerScoreReport>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlayerScoreReport {
    player_id: String,
    total: i32,
    breakdown: harmonies_core::scoring::ScoreBreakdown,
}
