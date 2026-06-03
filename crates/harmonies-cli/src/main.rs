use std::{fs, io::Read, path::PathBuf};

use anyhow::Context;
use harmonies_core::{advise, bga::normalize_gamedatas, AdvisorRequestV1, CardCatalog};
use serde_json::Value;

fn main() -> anyhow::Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if matches!(args.first().map(String::as_str), Some("normalize")) {
        args.remove(0);
        return normalize_command(args);
    }

    let mut args = args.into_iter();
    let request_path = args.next().map(PathBuf::from);
    let catalog_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("docs/cards_database.json"));

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
