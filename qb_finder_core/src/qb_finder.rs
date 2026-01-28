use itertools::Itertools;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};

use rustc_hash::FxHashSet;
use srs_4l::{
    brokenboard::BrokenBoard,
    gameplay::{Board, Physics, Shape},
};

use crate::minimals::{all_min_cover_sets, min_cover_size};
use crate::queue::Bag;
use crate::solver;

pub struct QBFinder {
    legal_boards: FxHashSet<Board>,
    start: BrokenBoard,
    hold: bool,
    physics: Physics,
    pub skip_4p: bool,
}

impl QBFinder {
    pub fn new(legal_boards: FxHashSet<Board>) -> QBFinder {
        QBFinder {
            legal_boards: legal_boards,
            start: BrokenBoard::from_garbage(0),
            hold: true,
            physics: Physics::Jstris,
            skip_4p: false,
        }
    }

    fn is_hundred(
        &self,
        setup: &BrokenBoard,
        solve_queues: &Vec<Vec<Bag>>,
        save: Option<Shape>,
    ) -> bool {
        solve_queues.iter().all(|q| {
            !solver::compute(
                &self.legal_boards,
                &setup,
                &q,
                self.hold,
                self.physics,
                save,
            )
            .is_empty()
        })
    }

    pub fn compute(
        &self,
        queue: &str,
        setup: &BrokenBoard,
        save: Option<Shape>,
    ) -> Vec<BrokenBoard> {
        solver::compute(
            &self.legal_boards,
            setup,
            &pattern_bags(queue),
            true,
            Physics::Jstris,
            save,
        )
    }

    pub fn find(&self, build_queue: &str, solve_queue: &str, save: char) -> Vec<BrokenBoard> {
        let solve_queues = expand_pattern(solve_queue)
            .into_iter()
            .map(|q| {
                q.chars()
                    .map(|c| {
                        let shape = parse_shape(c).expect("Invalid solve pattern");
                        Bag::new(&[shape], 1)
                    })
                    .collect()
            })
            .collect();

        let parsed_save = parse_shape(save);

        let mut setups = if self.skip_4p && build_queue.replace(",", "").len() == 4 {
            vec![]
        } else {
            self.compute(build_queue, &self.start, None)
        };

        setups = setups
            .into_par_iter()
            .filter(|setup| {
                self.is_hundred(
                    &BrokenBoard::from_garbage(setup.to_broken_bitboard().0),
                    &solve_queues,
                    parsed_save,
                )
            })
            .collect();
        // Maybe switch to using save in build
        if setups.len() == 0 && build_queue.replace(",", "").len() == 4 {
            let build_pieces = build_queue.replace(",", "");
            let xor = build_pieces.chars().fold(0, |a, c| a ^ (c as u8));

            for p3 in build_pieces.chars().combinations(3).unique() {
                let b: String = p3.iter().collect();
                let r: String = ((xor ^ b.chars().fold(0, |a, c| a ^ (c as u8))) as char).into();

                setups.extend(self.find(&b, &(r + "," + &solve_queue), save));
            }
        }
        setups
    }

    pub fn min_count(
        &self,
        setup: &BrokenBoard,
        pattern: &str,
        universe: &FxHashSet<String>,
        save: Option<Shape>,
    ) -> usize {
        let solves = solver::compute(
            &self.legal_boards,
            &BrokenBoard::from_garbage(setup.to_broken_bitboard().0),
            &pattern_bags(pattern),
            true,
            Physics::Jstris,
            save,
        );

        let pattern_xor = pattern.replace(",", "").bytes().fold(0, |acc, b| acc ^ b);

        let covering_queues: Vec<Vec<String>> = solves
            .into_iter()
            .map(|solve| {
                solve
                    .supporting_queues(Physics::Jstris)
                    .iter()
                    .flat_map(|&q| match save {
                        Some(s) => q.push_last(s).unhold(),
                        None => {
                            let saved_piece = q
                                .map(|s| s.name().as_bytes()[0])
                                .fold(pattern_xor, |acc, b| acc ^ b)
                                as char;
                            parse_shape(saved_piece)
                                .map_or_else(|| q.unhold(), |s| q.push_last(s).unhold())
                        }
                    })
                    .unique_by(|q| q.0)
                    .map(|q| q.to_string())
                    .filter(|q| universe.contains(q))
                    .collect()
            })
            .collect();
        min_cover_size(universe, &covering_queues)
    }

    pub fn all_min_sets(
        &self,
        setup: &BrokenBoard,
        pattern: &str,
        universe: &FxHashSet<String>,
        save: Option<Shape>,
    ) -> (Vec<BrokenBoard>, Vec<Vec<usize>>) {
        let solves = solver::compute(
            &self.legal_boards,
            &BrokenBoard::from_garbage(setup.to_broken_bitboard().0),
            &pattern_bags(pattern),
            true,
            Physics::Jstris,
            save,
        );

        let pattern_xor = pattern.replace(",", "").bytes().fold(0, |acc, b| acc ^ b);

        let covering_queues: Vec<Vec<String>> = solves
            .iter()
            .map(|solve| {
                solve
                    .supporting_queues(Physics::Jstris)
                    .iter()
                    .flat_map(|&q| match save {
                        Some(s) => q.push_last(s).unhold(),
                        None => {
                            let saved_piece = q
                                .map(|s| s.name().as_bytes()[0])
                                .fold(pattern_xor, |acc, b| acc ^ b)
                                as char;
                            parse_shape(saved_piece)
                                .map_or_else(|| q.unhold(), |s| q.push_last(s).unhold())
                        }
                    })
                    .unique_by(|q| q.0)
                    .map(|q| q.to_string())
                    .filter(|q| universe.contains(q))
                    .collect()
            })
            .collect();

        (
            solves,
            all_min_cover_sets(universe, &covering_queues)
        )
    }
}

fn pattern_bags(pattern: &str) -> Vec<Bag> {
    let mut bags = Vec::new();
    for bag in pattern.split(",") {
        let shapes = bag
            .chars()
            .map(parse_shape)
            .collect::<Option<Vec<Shape>>>()
            .unwrap();
        bags.push(Bag::new(&shapes, bag.len() as u8));
    }
    bags
}

pub fn expand_pattern(pattern: &str) -> Vec<String> {
    pattern
        .split(",")
        .map(|group| {
            let len = group.len();
            group
                .chars()
                .permutations(len)
                .unique()
                .map(|p| p.into_iter().collect::<String>())
                .collect_vec()
        })
        .multi_cartesian_product()
        .map(|prod| prod.join(""))
        .collect()
}

pub fn parse_shape(shape: char) -> Option<Shape> {
    match shape {
        'I' => Some(Shape::I),
        'J' => Some(Shape::J),
        'L' => Some(Shape::L),
        'O' => Some(Shape::O),
        'S' => Some(Shape::S),
        'T' => Some(Shape::T),
        'Z' => Some(Shape::Z),
        _ => None,
    }
}
