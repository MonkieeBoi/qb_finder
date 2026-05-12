pub mod minimals;
pub mod queue;
pub mod solver;

use std::sync::atomic::{AtomicUsize, Ordering};

use itertools::Itertools;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};

use rustc_hash::{FxHashMap, FxHashSet};
use srs_4l::{
    brokenboard::BrokenBoard,
    gameplay::{Board, Physics, Shape},
};

use crate::minimals::{all_min_cover_sets, min_cover_size};
use crate::queue::Bag;

pub struct QBFinder {
    legal_boards: FxHashSet<Board>,
    start: BrokenBoard,
    hold: bool,
    physics: Physics,
    pub skip_4p: bool,
    pub full_cover: bool,
}

impl QBFinder {
    pub fn new(legal_boards: FxHashSet<Board>) -> QBFinder {
        QBFinder {
            legal_boards: legal_boards,
            start: BrokenBoard::from_garbage(0),
            hold: true,
            physics: Physics::Jstris,
            skip_4p: false,
            full_cover: false,
        }
    }

    fn good_save_count(
        &self,
        setup: &BrokenBoard,
        solve_queues: &[Vec<Bag>],
        saves: &[Shape],
        cur_best: usize,
    ) -> usize {
        let p_save = saves.first().copied();
        let s_saves = saves.get(1..).unwrap_or_default();

        let mut res = 0;

        for (i, q) in solve_queues.iter().enumerate() {
            if res + (solve_queues.len() - i) < cur_best {
                return 0;
            }
            if !solver::compute(
                &self.legal_boards,
                setup,
                q,
                self.hold,
                self.physics,
                p_save,
            )
            .is_empty()
            {
                res += 1;
                continue;
            }

            if s_saves.iter().all(|&s| {
                solver::compute(
                    &self.legal_boards,
                    setup,
                    q,
                    self.hold,
                    self.physics,
                    Some(s),
                )
                .is_empty()
            }) {
                return 0;
            }
        }
        return res;
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
            self.physics,
            save,
        )
    }

    pub fn find(
        &self,
        build_queue: &str,
        build_save: Option<Shape>,
        solve_queue: &str,
        saves: &str,
        min_saves: usize,
    ) -> (Vec<BrokenBoard>, usize) {
        let p_count = 11
            - (self.start.board.0.count_ones() / 4) as usize
            - build_queue.replace(",", "").len();
        let solve_queues: Vec<Vec<Bag>> = expand_pattern(solve_queue)
            .into_iter()
            .map(|q| {
                build_save
                    .map(|s| Bag::new(&[s], 1))
                    .into_iter()
                    .chain(q.chars().take(p_count as usize).map(|c| {
                        let shape = parse_shape(c).expect("Invalid solve pattern");
                        Bag::new(&[shape], 1)
                    }))
                    .collect()
            })
            .collect();

        let parsed_saves: Vec<Shape> = saves.chars().unique().filter_map(parse_shape).collect();

        let mut setups =
            if self.skip_4p && build_queue.replace(",", "").len() == 4 && build_save.is_none() {
                vec![]
            } else {
                self.compute(build_queue, &self.start, build_save)
            };

        if self.full_cover {
            let build_queues: Vec<_> = expand_pattern(build_queue)
                .into_iter()
                .map(|q| q.chars().filter_map(parse_shape).collect())
                .collect();
            setups = setups
                .into_par_iter()
                .filter(|setup| {
                    let scover: Vec<_> = setup
                        .supporting_queues(self.physics)
                        .iter()
                        .flat_map(|&q| match build_save {
                            Some(s) => q.push_last(s).unhold(),
                            None => q.unhold(),
                        })
                        .collect();
                    build_queues.iter().all(|q| scover.contains(q))
                })
                .collect();
        }

        let primary_save_count = AtomicUsize::new(min_saves);

        let setup_saves: Vec<(usize, BrokenBoard)> = setups
            .into_par_iter()
            .map(|setup| {
                let cur_best = primary_save_count.load(std::sync::atomic::Ordering::Relaxed);
                let save_count = self.good_save_count(
                    &BrokenBoard::from_garbage(setup.to_broken_bitboard().0),
                    &solve_queues,
                    &parsed_saves,
                    cur_best,
                );
                if save_count > cur_best {
                    primary_save_count.fetch_max(save_count, Ordering::Relaxed);
                }
                (save_count, setup)
            })
            .collect();

        let mut max_save = primary_save_count.load(Ordering::SeqCst);

        setups = setup_saves
            .into_iter()
            .filter(|(s, _)| *s == max_save)
            .map(|(_, s)| s)
            .collect();

        if setups.len() == 0 && build_queue.replace(",", "").len() == 4 && build_save.is_none() {
            for p in build_queue.replace(",", "").chars().unique() {
                let (subsetup, sub_save) =
                    self.find(build_queue, parse_shape(p), solve_queue, saves, max_save);
                if sub_save > max_save {
                    setups.clear();
                    max_save = sub_save
                }
                if sub_save == max_save {
                    setups.extend(subsetup);
                }
            }
        }
        (setups, max_save)
    }

    pub fn min_count(
        &self,
        setup: &BrokenBoard,
        pattern: &str,
        universe: &FxHashSet<String>,
        saves: &str,
    ) -> usize {
        let mut covering_queues = vec![];
        let mut primary_cover = FxHashSet::default();
        let pattern_xor = pattern.replace(',', "").bytes().fold(0, |acc, b| acc ^ b);
        let parsed_saves: Vec<Option<Shape>> = saves.chars().unique().map(parse_shape).collect();

        let saves_to_check = if parsed_saves.is_empty() {
            vec![None]
        } else {
            parsed_saves
        };

        for (i, &save) in saves_to_check.iter().enumerate() {
            if primary_cover.len() == universe.len() {
                break;
            }

            let solves = solver::compute(
                &self.legal_boards,
                &BrokenBoard::from_garbage(setup.to_broken_bitboard().0),
                &pattern_bags(pattern),
                true,
                self.physics,
                save,
            );

            for solve in solves {
                let cover: Vec<String> = solve
                    .supporting_queues(self.physics)
                    .iter()
                    .flat_map(|&q| match save {
                        Some(s) => q.push_last(s).unhold(),
                        None => {
                            let saved = (q
                                .map(|s| s.name().as_bytes()[0])
                                .fold(pattern_xor, |a, b| a ^ b))
                                as char;
                            parse_shape(saved)
                                .map_or_else(|| q.unhold(), |s| q.push_last(s).unhold())
                        }
                    })
                    .map(|q| q.to_string())
                    .filter(|q| universe.contains(q) && (i == 0 || !primary_cover.contains(q)))
                    .collect();

                if i == 0 {
                    primary_cover.extend(cover.clone());
                }
                covering_queues.push(cover);
            }
        }
        min_cover_size(universe, &covering_queues)
    }

    pub fn all_min_sets(
        &self,
        setup: &BrokenBoard,
        pattern: &str,
        universe: &FxHashSet<String>,
        saves: &str,
    ) -> (
        Vec<BrokenBoard>,
        Vec<Vec<usize>>,
        FxHashMap<usize, Vec<usize>>,
    ) {
        let mut covering_queues = vec![];
        let mut primary_cover = FxHashSet::default();
        let pattern_xor = pattern.replace(',', "").bytes().fold(0, |acc, b| acc ^ b);
        let parsed_saves: Vec<Option<Shape>> = saves.chars().unique().map(parse_shape).collect();

        let saves_to_check = if parsed_saves.is_empty() {
            vec![None]
        } else {
            parsed_saves
        };

        let mut all_solves = vec![];
        let mut equivalent_map: FxHashMap<usize, Vec<usize>> = FxHashMap::default();

        for (i, &save) in saves_to_check.iter().enumerate() {
            let solves = solver::compute(
                &self.legal_boards,
                &BrokenBoard::from_garbage(setup.to_broken_bitboard().0),
                &pattern_bags(pattern),
                true,
                Physics::Jstris,
                save,
            );

            let mut solve_cover_hashsets: Vec<FxHashSet<String>> = vec![];

            for solve in &solves {
                let mut cover: Vec<String> = solve
                    .supporting_queues(Physics::Jstris)
                    .iter()
                    .flat_map(|&q| match save {
                        Some(s) => q.push_last(s).unhold(),
                        None => {
                            let saved = (q
                                .map(|s| s.name().as_bytes()[0])
                                .fold(pattern_xor, |a, b| a ^ b))
                                as char;
                            parse_shape(saved)
                                .map_or_else(|| q.unhold(), |s| q.push_last(s).unhold())
                        }
                    })
                    .map(|q| q.to_string())
                    .filter(|q| universe.contains(q) && (i == 0 || !primary_cover.contains(q)))
                    .collect();

                let mut cover_set: FxHashSet<String> = cover.iter().cloned().collect();

                for (j, previous_solve) in solve_cover_hashsets.iter().enumerate() {
                    if &cover_set == previous_solve {
                        cover.clear();
                        cover_set.clear();
                        equivalent_map
                            .entry(j + all_solves.len())
                            .or_default()
                            .push(covering_queues.len());
                        break;
                    }
                }

                solve_cover_hashsets.push(cover_set);

                if i == 0 {
                    primary_cover.extend(cover.clone());
                }
                covering_queues.push(cover);
            }
            all_solves.extend(solves);
        }
        let all_sets = all_min_cover_sets(universe, &covering_queues);
        let used_solves: FxHashSet<usize> = all_sets.iter().cloned().flatten().collect();
        equivalent_map.retain(|k, _| used_solves.contains(k));
        (all_solves, all_sets, equivalent_map)
    }
}

fn pattern_bags(pattern: &str) -> Vec<Bag> {
    let mut bags = Vec::new();
    for line in pattern.lines() {
        for bag in line.trim().split(",") {
            let shapes = bag
                .chars()
                .map(parse_shape)
                .collect::<Option<Vec<Shape>>>()
                .unwrap();
            bags.push(Bag::new(&shapes, bag.len() as u8));
        }
    }
    bags
}

pub fn expand_pattern(pattern: &str) -> Vec<String> {
    pattern
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .flat_map(|line| {
            line.split(",")
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
        })
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
