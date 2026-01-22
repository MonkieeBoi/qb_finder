use itertools::Itertools;
use js_sys::Uint8Array;
use std::{collections::HashSet, io::Cursor};

use qb_finder_core::{qb_finder::{QBFinder, expand_pattern, parse_shape}, solver};
use srs_4l::{board_list, gameplay::Board};
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub struct QBF {
    qbf: QBFinder,
}

#[wasm_bindgen]
impl QBF {
    #[wasm_bindgen(constructor)]
    pub fn init(legal_boards: Option<Uint8Array>) -> QBF {
        let boards: HashSet<Board> = match legal_boards {
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

    pub fn set_skip_4p (&mut self, skip_4p: bool) {
        self.qbf.skip_4p = skip_4p;
    }

    pub fn find(&self, build_queue: &str, solve_queue: &str, save: char) -> String {
        let setups = self.qbf.find(build_queue, &solve_queue, save);
        let solve_queues: HashSet<String> = expand_pattern(&solve_queue).into_iter().collect();
        let min_setups: Vec<_> = setups
            .iter()
            .map(|b| {
                (b, {
                    if b.pieces.len() == build_queue.replace(",", "").len() - 1 {
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
                        self.qbf.min_count(b, &solve_queue, &solve_queues, parse_shape(save))
                    }
                })
            })
            .sorted_by_key(|(_, count)| *count).collect();

        let mut str = String::new();

        for (board, min_count) in &min_setups {
            solver::print(&board, &mut str);
            str.push_str(&format!(",{}|", min_count));
        }

        str.pop();
        str
    }
}


