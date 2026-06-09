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

/// Contains (**All Solves**, **All Minimal Sets**, **Solve -> Equivalent Cover Map**).
type SetupMinimals = (
    Vec<BrokenBoard>,
    Vec<Vec<usize>>,
    FxHashMap<usize, Vec<usize>>,
);

pub struct QBFinder {
    legal_boards: FxHashSet<Board>,
    start: BrokenBoard,
    physics: Physics,
    pub hold: bool,
    pub skip_4p: bool,
    pub full_cover: bool,
}

impl QBFinder {
    pub fn new(legal_boards: FxHashSet<Board>) -> QBFinder {
        QBFinder {
            legal_boards,
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
        res
    }

    pub fn compute(
        &self,
        queue: &str,
        setup: &BrokenBoard,
        save: Option<Shape>,
    ) -> Vec<BrokenBoard> {
        queue
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .flat_map(|pattern| {
                solver::compute(
                    &self.legal_boards,
                    setup,
                    &pattern_bags(pattern),
                    self.hold,
                    self.physics,
                    save,
                )
            })
            .unique()
            .collect()
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
                    .chain(q.chars().take(p_count).map(|c| {
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

        if setups.is_empty() && build_queue.replace(",", "").len() == 4 && build_save.is_none() {
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
        let mut save_groups: Vec<Vec<Shape>> = saves
            .split(",")
            .map(|g| g.chars().unique().flat_map(parse_shape).collect())
            .filter(|g: &Vec<_>| !g.is_empty())
            .collect();

        if save_groups.is_empty() {
            use Shape::*;
            save_groups.push(vec![]);
            save_groups.push(vec![I, J, L, O, S, T, Z]);
        }

        let mut equivalent_map: FxHashMap<BrokenBoard, Vec<BrokenBoard>> = FxHashMap::default();
        let mut setup_cover_map: FxHashMap<BrokenBoard, FxHashSet<String>> = FxHashMap::default();
        let mut already_covered = FxHashSet::default();

        for group in save_groups {
            let mut new_cover = FxHashSet::default();
            let group_saves = if group.is_empty() {
                vec![None]
            } else {
                group.into_iter().map(|s| Some(s)).collect()
            };
            for save in group_saves {
                let solves = self.compute(
                    pattern,
                    &BrokenBoard::from_garbage(setup.to_broken_bitboard().0),
                    save,
                );

                let mut prev_solves: Vec<BrokenBoard> = vec![];

                for solve in solves {
                    let mut cover: Vec<String> = solve
                        .supporting_queues(Physics::Jstris)
                        .iter()
                        .flat_map(|&q| match save {
                            Some(s) => q.push_last(s).unhold(),
                            None => q.unhold(),
                        })
                        .map(|q| q.to_string())
                        .filter(|q| universe.contains(q) && !already_covered.contains(q))
                        .collect();

                    let mut cover_set: FxHashSet<String> = cover.iter().cloned().collect();

                    for psolve in &prev_solves {
                        if cover_set == setup_cover_map[&psolve] {
                            cover.clear();
                            cover_set.clear();
                            equivalent_map
                                .entry(psolve.clone())
                                .or_default()
                                .push(solve.clone());
                            break;
                        }
                    }

                    prev_solves.push(solve.clone());
                    setup_cover_map
                        .entry(solve)
                        .or_default()
                        .extend(cover.clone());
                    new_cover.extend(cover);
                }
            }
            already_covered.extend(new_cover);
        }
        let covering_queues: Vec<Vec<String>> = setup_cover_map
            .values()
            .map(|c| c.iter().cloned().collect())
            .collect();
        min_cover_size(universe, &covering_queues)
    }

    pub fn all_min_sets(
        &self,
        setup: &BrokenBoard,
        pattern: &str,
        universe: &FxHashSet<String>,
        saves: &str,
    ) -> SetupMinimals {
        let mut save_groups: Vec<Vec<Shape>> = saves
            .split(",")
            .map(|g| g.chars().unique().flat_map(parse_shape).collect())
            .filter(|g: &Vec<_>| !g.is_empty())
            .collect();

        if save_groups.is_empty() {
            use Shape::*;
            save_groups.push(vec![]);
            save_groups.push(vec![I, J, L, O, S, T, Z]);
        }

        let mut equivalent_map: FxHashMap<BrokenBoard, Vec<BrokenBoard>> = FxHashMap::default();
        let mut setup_cover_map: FxHashMap<BrokenBoard, FxHashSet<String>> = FxHashMap::default();
        let mut already_covered = FxHashSet::default();

        for group in save_groups {
            let mut new_cover = FxHashSet::default();
            let group_saves = if group.is_empty() {
                vec![None]
            } else {
                group.into_iter().map(|s| Some(s)).collect()
            };
            for save in group_saves {
                let solves = self.compute(
                    pattern,
                    &BrokenBoard::from_garbage(setup.to_broken_bitboard().0),
                    save,
                );

                let mut prev_solves: Vec<BrokenBoard> = vec![];

                for solve in solves {
                    let mut cover: Vec<String> = solve
                        .supporting_queues(Physics::Jstris)
                        .iter()
                        .flat_map(|&q| match save {
                            Some(s) => q.push_last(s).unhold(),
                            None => q.unhold(),
                        })
                        .map(|q| q.to_string())
                        .filter(|q| universe.contains(q) && !already_covered.contains(q))
                        .collect();

                    let mut cover_set: FxHashSet<String> = cover.iter().cloned().collect();

                    for psolve in &prev_solves {
                        if cover_set == setup_cover_map[&psolve] {
                            cover.clear();
                            cover_set.clear();
                            equivalent_map
                                .entry(psolve.clone())
                                .or_default()
                                .push(solve.clone());
                            break;
                        }
                    }

                    prev_solves.push(solve.clone());
                    setup_cover_map
                        .entry(solve)
                        .or_default()
                        .extend(cover.clone());
                    new_cover.extend(cover);
                }
            }
            already_covered.extend(new_cover);
        }
        let all_solves: Vec<BrokenBoard> = setup_cover_map.keys().cloned().collect();
        let covering_queues: Vec<Vec<String>> = all_solves
            .iter()
            .map(|s| setup_cover_map[s].iter().cloned().collect())
            .collect();
        let solve_index_map: FxHashMap<_, usize> = all_solves
            .iter()
            .enumerate()
            .map(|(i, solve)| (solve, i))
            .collect();
        let all_sets = all_min_cover_sets(universe, &covering_queues);
        let used_solves: FxHashSet<usize> = all_sets.iter().flatten().cloned().collect();
        let mut equivalent_map: FxHashMap<usize, Vec<usize>> = equivalent_map
            .into_iter()
            .map(|(key, vector)| {
                let new_key = solve_index_map[&key];
                let new_vector = vector.into_iter().map(|b| solve_index_map[&b]).collect();
                (new_key, new_vector)
            })
            .collect();
        equivalent_map.retain(|k, _| used_solves.contains(k));
        (all_solves, all_sets, equivalent_map)
    }
}
