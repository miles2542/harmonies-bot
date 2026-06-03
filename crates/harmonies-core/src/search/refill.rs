use std::collections::HashSet;

use crate::model::{BagCounts, Color};

pub(super) fn candidate_refills(
    bag_counts: &BagCounts,
    samples: usize,
    seed: u64,
) -> Vec<Vec<Color>> {
    let mut colors = expanded_bag(bag_counts);
    if colors.len() < 3 {
        return Vec::new();
    }
    let mut refills = Vec::new();
    colors.sort_by_key(|color| std::cmp::Reverse(bag_counts.get(*color)));
    refills.push(sorted_refill(colors.iter().copied().take(3).collect()));
    let mut rng = seed.max(1);
    for _ in 0..samples {
        let mut pool = colors.clone();
        let mut refill = Vec::new();
        for _ in 0..3 {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let index = (rng as usize) % pool.len();
            refill.push(pool.remove(index));
        }
        refills.push(sorted_refill(refill));
    }
    let mut seen = HashSet::new();
    refills.retain(|refill| seen.insert(refill.clone()));
    refills
}

fn expanded_bag(bag_counts: &BagCounts) -> Vec<Color> {
    [
        (Color::Water, bag_counts.water),
        (Color::Mountain, bag_counts.mountain),
        (Color::Trunk, bag_counts.trunk),
        (Color::Foliage, bag_counts.foliage),
        (Color::Field, bag_counts.field),
        (Color::Building, bag_counts.building),
    ]
    .into_iter()
    .flat_map(|(color, count)| std::iter::repeat(color).take(count as usize))
    .collect()
}

fn sorted_refill(mut refill: Vec<Color>) -> Vec<Color> {
    refill.sort_by_key(color_sort_key);
    refill
}

fn color_sort_key(color: &Color) -> u8 {
    match color {
        Color::Water => 1,
        Color::Mountain => 2,
        Color::Trunk => 3,
        Color::Foliage => 4,
        Color::Field => 5,
        Color::Building => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refill_sampling_respects_available_counts() {
        let refills = candidate_refills(
            &BagCounts {
                water: 2,
                mountain: 1,
                ..BagCounts::default()
            },
            4,
            7,
        );
        assert_eq!(
            refills,
            vec![vec![Color::Water, Color::Water, Color::Mountain]]
        );
    }
}
