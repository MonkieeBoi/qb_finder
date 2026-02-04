use itertools::Itertools;
use js_sys::Uint8Array;
use qb_finder_core::{QBFinder, expand_pattern, parse_shape, solver};
use rustc_hash::FxHashSet;
use std::io::Cursor;

use srs_4l::{
    base64::{base64_decode, base64_encode},
    board_list,
    brokenboard::BrokenBoard,
    gameplay::Board,
};
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub struct QBF {
    qbf: QBFinder,
}

#[wasm_bindgen]
impl QBF {
    #[wasm_bindgen(constructor)]
    pub fn init(legal_boards: Option<Uint8Array>) -> QBF {
        let boards: FxHashSet<Board> = match legal_boards {
            Some(arr) => board_list::read(Cursor::new(&arr.to_vec()))
                .unwrap()
                .drain(..)
                .collect(),
            None => Default::default(),
        };

        QBF {
            qbf: QBFinder::new(boards),
        }
    }

    pub fn set_skip_4p(&mut self, skip_4p: bool) {
        self.qbf.skip_4p = skip_4p;
    }

    pub fn find(&self, build_queue: &str, solve_queue: &str, save: char) -> String {
        let setups = self.qbf.find(build_queue, None, &solve_queue, save);
        let solve_queues: FxHashSet<String> = expand_pattern(&solve_queue).into_iter().collect();
        let min_setups: Vec<_> = setups
            .iter()
            .map(|b| {
                (b, {
                    if b.pieces.len() < 3 {
                        0
                    } else if b.pieces.len() == build_queue.replace(",", "").len() - 1 {
                        let xor = build_queue
                            .replace(",", "")
                            .chars()
                            .fold(0, |a, c| a ^ (c as u8));
                        let r: String = ((xor
                            ^ b.pieces
                                .iter()
                                .map(|p| p.shape.name().chars().nth(0).unwrap_or_default())
                                .fold(0, |a, c| a ^ (c as u8)))
                            as char)
                            .into();
                        self.qbf.min_count(
                            b,
                            &(r.clone() + &solve_queue),
                            &solve_queues.clone().iter().map(|q| r.clone() + q).collect(),
                            parse_shape(save),
                        )
                    } else {
                        self.qbf
                            .min_count(b, &solve_queue, &solve_queues, parse_shape(save))
                    }
                })
            })
            .sorted_by_key(|(_, count)| *count)
            .collect();

        let mut res = String::new();

        for (board, min_count) in &min_setups {
            solver::print(&board, &mut res);
            res.push_str(&format!(",{},", min_count));
            base64_encode(&board.encode(), &mut res);
            res.push('|');
        }

        res.pop();
        res
    }

    pub fn find_min_sets(
        &self,
        setup: &str,
        build_queue: &str,
        solve_queue: &str,
        save: char,
    ) -> String {
        let mut res = String::new();

        let bits = match base64_decode(setup) {
            Some(b) => b,
            None => return res,
        };

        let board = match BrokenBoard::decode(&bits) {
            Some(b) => b,
            None => return res,
        };

        solver::print(&board, &mut res);
        res.push('|');

        let solve_queues: FxHashSet<String> = expand_pattern(&solve_queue).into_iter().collect();

        let (solves, covers) = if board.pieces.len() == build_queue.replace(",", "").len() - 1 {
            let xor = build_queue
                .replace(",", "")
                .chars()
                .fold(0, |a, c| a ^ (c as u8));

            let r: String = ((xor
                ^ board
                    .pieces
                    .iter()
                    .map(|p| p.shape.name().chars().nth(0).unwrap_or_default())
                    .fold(0, |a, c| a ^ (c as u8))) as char)
                .into();

            self.qbf.all_min_sets(
                &board,
                &(r.clone() + &solve_queue),
                &solve_queues.clone().iter().map(|q| r.clone() + q).collect(),
                parse_shape(save),
            )
        } else {
            self.qbf
                .all_min_sets(&board, &solve_queue, &solve_queues, parse_shape(save))
        };

        let mut common: FxHashSet<usize> = covers[0].iter().cloned().collect();

        for set in covers.iter().skip(1) {
            let current_set: FxHashSet<usize> = set.iter().cloned().collect();
            common.retain(|idx| current_set.contains(idx));
        }

        for &idx in &common {
            solver::print(&solves[idx], &mut res);
            res.push(',');
        }
        res.push('|');

        for set in covers {
            let unique: Vec<_> = set.iter().filter(|idx| !common.contains(idx)).collect();
            for &idx in &unique {
                solver::print(&solves[*idx], &mut res);
                res.push(',');
            }
            res.push('|');
        }

        res.pop();
        res
    }
}
