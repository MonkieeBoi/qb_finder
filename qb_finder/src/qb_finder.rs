use itertools::Itertools;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::collections::HashSet;
use std::fs::File;
use std::io::{Cursor, Read};

use srs_4l::{
    board_list,
    brokenboard::BrokenBoard,
    gameplay::{Board, Physics, Shape},
};

use crate::minimals::min_cover_size;
use crate::queue::Bag;
use crate::solver;

pub struct QBFinder {
    legal_boards: HashSet<Board>,
    start: BrokenBoard,
    hold: bool,
    physics: Physics,
    skip_4p: bool,
}

impl QBFinder {
    pub fn new() -> QBFinder {
        let mut file = File::open("./legal-boards.leb128").expect("Failed to open legal_boards");
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .expect("Failed to read legal_boards");

        let boards: HashSet<Board> = board_list::read(Cursor::new(buffer))
            .unwrap()
            .into_iter()
            .collect();

        QBFinder {
            legal_boards: boards,
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
        solve_save_count: usize,
    ) -> bool {
        solve_queues.iter().all(|q| {
            let solves = solver::compute(&self.legal_boards, &setup, &q, self.hold, self.physics);

            if solves.is_empty() {
                return false;
            }

            let Some(save_shape) = save else {
                return true;
            };

            solves.iter().any(|solve| {
                let save_count = solve
                    .pieces
                    .iter()
                    .filter(|piece| piece.shape == save_shape)
                    .count();
                solve_save_count == 0 || solve_save_count - 1 == save_count
            })
        })
    }

    pub fn compute(&self, queue: &str, setup: &BrokenBoard) -> Vec<BrokenBoard> {
        solver::compute(
            &self.legal_boards,
            setup,
            &pattern_bags(queue),
            true,
            Physics::Jstris,
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

        let save_count = solve_queue.chars().filter(|x| *x == save).count();
        let parsed_save = parse_shape(save);

        let mut setups = if self.skip_4p && build_queue.replace(",", "").len() == 4 {
            vec![]
        } else {
            self.compute(build_queue, &self.start)
        };

        setups = setups
            .into_par_iter()
            .filter(|setup| {
                self.is_hundred(
                    &BrokenBoard::from_garbage(setup.to_broken_bitboard().0),
                    &solve_queues,
                    parsed_save,
                    save_count,
                )
            })
            .collect();
        if setups.len() == 0 && build_queue.replace(",", "").len() == 4 {
            for p3 in build_queue.chars().combinations(3) {
                let b: String = p3.iter().collect();
                let r: String = build_queue.chars().filter(|c| !p3.contains(c)).collect();
                setups.extend(self.find(&b, &(r + "," + &solve_queue), save));
            }
        }
        setups
    }

    pub fn min_count(
        &self,
        setup: &BrokenBoard,
        pattern: &str,
        universe: &HashSet<String>,
        save: Option<Shape>,
        solve_save_count: usize,
    ) -> usize {
        let mut solves = solver::compute(
            &self.legal_boards,
            &BrokenBoard::from_garbage(setup.to_broken_bitboard().0),
            &pattern_bags(pattern),
            true,
            Physics::Jstris,
        );

        if let (Some(s), true) = (save, solve_save_count > 0) {
            let target = solve_save_count - 1;
            solves.retain(|sol| sol.pieces.iter().filter(|p| p.shape == s).count() == target);
        }

        let pattern_xor = pattern.replace(",", "").bytes().fold(0, |acc, b| acc ^ b);

        let covering_queues: Vec<Vec<String>> = solves
            .into_iter()
            .map(|solve| {
                solve
                    .supporting_queues(Physics::Jstris)
                    .iter()
                    .flat_map(|&q| {
                        let saved_piece = q
                            .map(|s| s.name().as_bytes()[0])
                            .fold(pattern_xor, |acc, b| acc ^ b)
                            as char;
                        parse_shape(saved_piece)
                            .map_or_else(|| q.unhold(), |s| q.push_last(s).unhold())
                    })
                    .unique_by(|q| q.0)
                    .map(|q| q.to_string())
                    .filter(|q| universe.contains(q))
                    .collect()
            })
            .collect();
        min_cover_size(universe, covering_queues)
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


pub fn min_count(
    legal_boards: &HashSet<Board>,
    setup: &BrokenBoard,
    pattern: &str,
    universe: &HashSet<String>,
    save: Option<Shape>,
    solve_save_count: usize,
) -> usize {
    let board = &BrokenBoard::from_garbage(setup.to_broken_bitboard().0);
    let mut solves = solver::compute(
        legal_boards,
        board,
        &pattern_bags(pattern),
        true,
        Physics::Jstris,
    );

    if let (Some(s), true) = (save, solve_save_count > 0) {
        let target = solve_save_count - 1;
        solves.retain(|sol| sol.pieces.iter().filter(|p| p.shape == s).count() == target);
    }

    let pattern_bytes = pattern.replace(",", "").into_bytes();

    let covering_queues: Vec<Vec<String>> = solves
        .into_iter()
        .map(|solve| {
            solve
                .supporting_queues(Physics::Jstris)
                .iter()
                .flat_map(|&q| {
                    let saved_piece = q
                        .map(|s| s.name().as_bytes()[0])
                        .chain(pattern_bytes.iter().copied())
                        .fold(0, |acc, b| acc ^ b) as char;
                    parse_shape(saved_piece).map_or_else(|| q.unhold(), |s| q.push_last(s).unhold())
                })
                .unique_by(|q| q.0)
                .map(|q| q.to_string())
                .filter(|q| universe.contains(q))
                .collect()
        })
        .collect();
    min_cover_size(universe, covering_queues)
}
