use crate::{
    cards::CardCatalog,
    model::{ActiveCard, BagCounts, Color, GameSnapshotV1},
};

pub(super) const RIVER_SIZE: usize = 5;

#[derive(Clone)]
pub(super) struct SimulationDeck {
    unseen_cards: Vec<u8>,
    next_card_id: u32,
}

impl SimulationDeck {
    pub(super) fn from_snapshot(snapshot: &GameSnapshotV1, catalog: &CardCatalog) -> Self {
        let visible = snapshot
            .river_cards
            .iter()
            .chain(
                snapshot
                    .players
                    .iter()
                    .flat_map(|player| player.active_cards.iter().chain(&player.completed_cards)),
            )
            .filter(|card| !card.is_spirit)
            .map(|card| card.type_arg)
            .collect::<Vec<_>>();
        let mut unseen_cards = catalog
            .cards
            .values()
            .filter(|card| !card.is_spirit && !visible.contains(&card.type_arg))
            .map(|card| card.type_arg)
            .collect::<Vec<_>>();
        unseen_cards.sort_unstable();
        Self {
            unseen_cards,
            next_card_id: 20_000,
        }
    }

    pub(super) fn draw_card(&mut self, rng: &mut u64) -> Option<u8> {
        if self.unseen_cards.is_empty() {
            return None;
        }
        *rng = next_rng(*rng);
        Some(
            self.unseen_cards
                .remove((*rng as usize) % self.unseen_cards.len()),
        )
    }

    pub(super) fn next_card_id(&mut self) -> u32 {
        let card_id = self.next_card_id;
        self.next_card_id += 1;
        card_id
    }
}

pub(super) fn draw_color(bag_counts: &mut BagCounts, rng: &mut u64) -> Option<Color> {
    let total = bag_counts.total_known();
    if total == 0 {
        return None;
    }
    *rng = next_rng(*rng);
    let mut index = (*rng % total as u64) as u16;
    for color in [
        Color::Water,
        Color::Mountain,
        Color::Trunk,
        Color::Foliage,
        Color::Field,
        Color::Building,
    ] {
        let count = bag_counts.get(color);
        if index < count {
            bag_counts.saturating_sub_color(color);
            return Some(color);
        }
        index -= count;
    }
    None
}

pub(super) fn synthetic_card(card_id: u32, type_arg: u8, catalog: &CardCatalog) -> ActiveCard {
    ActiveCard {
        card_id,
        type_arg,
        remaining_cubes: catalog
            .get(type_arg)
            .map(|card| card.point_locations.len() as u8)
            .unwrap_or(0),
        is_spirit: type_arg >= 33,
    }
}

fn next_rng(value: u64) -> u64 {
    value.wrapping_mul(6364136223846793005).wrapping_add(1)
}
