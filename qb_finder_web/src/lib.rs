use js_sys::Uint8Array;
use std::{collections::HashSet, io::Cursor};

use qb_finder_core::{qb_finder::QBFinder, solver};
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

        let mut str = String::new();

        for board in &setups {
            solver::print(&board, &mut str);
            str.push('|');
        }

        str.pop();
        str
    }
}
