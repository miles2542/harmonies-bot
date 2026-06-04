use std::collections::HashSet;

use crate::model::Coord;

pub const DIRECTIONS: usize = 6;

pub fn neighbor(coord: Coord, direction: usize) -> Coord {
    let even = coord.col % 2 == 0;
    let (dc, dr) = match (even, direction) {
        (true, 0) => (1, 0),
        (true, 1) => (1, -1),
        (true, 2) => (0, -1),
        (true, 3) => (-1, -1),
        (true, 4) => (-1, 0),
        (true, 5) => (0, 1),
        (false, 0) => (1, 1),
        (false, 1) => (1, 0),
        (false, 2) => (0, -1),
        (false, 3) => (-1, 0),
        (false, 4) => (-1, 1),
        (false, 5) => (0, 1),
        _ => unreachable!("direction bounded by caller"),
    };
    Coord {
        col: coord.col + dc,
        row: coord.row + dr,
    }
}

pub fn neighbors(coord: Coord) -> [Coord; DIRECTIONS] {
    [
        neighbor(coord, 0),
        neighbor(coord, 1),
        neighbor(coord, 2),
        neighbor(coord, 3),
        neighbor(coord, 4),
        neighbor(coord, 5),
    ]
}

pub fn rotate_chain(positions: &[usize], rotation: usize) -> Vec<usize> {
    positions
        .iter()
        .map(|position| (position + rotation) % DIRECTIONS)
        .collect()
}

pub fn connected_components(coords: &HashSet<Coord>) -> Vec<Vec<Coord>> {
    let mut unseen = coords.clone();
    let mut groups = Vec::new();

    while let Some(start) = unseen.iter().next().copied() {
        unseen.remove(&start);
        let mut stack = vec![start];
        let mut group = Vec::new();

        while let Some(coord) = stack.pop() {
            group.push(coord);
            for next in neighbors(coord) {
                if unseen.remove(&next) {
                    stack.push(next);
                }
            }
        }
        groups.push(group);
    }

    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn odd_q_neighbors_match_bga_dom_layout() {
        assert_eq!(
            neighbors(Coord { col: 2, row: 2 }),
            [
                Coord { col: 3, row: 2 },
                Coord { col: 3, row: 1 },
                Coord { col: 2, row: 1 },
                Coord { col: 1, row: 1 },
                Coord { col: 1, row: 2 },
                Coord { col: 2, row: 3 },
            ]
        );
        assert_eq!(
            neighbors(Coord { col: 3, row: 2 }),
            [
                Coord { col: 4, row: 3 },
                Coord { col: 4, row: 2 },
                Coord { col: 3, row: 1 },
                Coord { col: 2, row: 2 },
                Coord { col: 2, row: 3 },
                Coord { col: 3, row: 3 },
            ]
        );
    }
}
