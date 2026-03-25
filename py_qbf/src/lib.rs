use dashmap::DashMap;
use itertools::Itertools;
use pyo3::prelude::*;
use qb_finder_core::{QBFinder, parse_shape, solver};
use rayon::prelude::*;
use rustc_hash::FxHashSet;
use srs_4l::{
    board_list,
    brokenboard::BrokenBoard,
    gameplay::{Board, Shape},
    queue::Queue,
};
use std::{
    collections::{HashMap, HashSet},
    io::Cursor,
};

#[pyclass]
struct QBSolver {
    qbf: QBFinder,
}

#[pymethods]
impl QBSolver {
    #[new]
    #[pyo3(signature = (threads=1))]
    fn new(threads: usize) -> PyResult<Self> {
        let bytes = include_bytes!("../../legal-boards.leb128");

        let legal_boards: FxHashSet<Board> = board_list::read(Cursor::new(bytes))
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to read boards: {}",
                    e
                ))
            })?
            .into_iter()
            .collect();
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .unwrap_or(());
        Ok(QBSolver {
            qbf: QBFinder::new(legal_boards),
        })
    }

    #[pyo3(signature = (queue, save=None, garb=None))]
    fn compute(
        &self,
        py: Python,
        queue: &str,
        save: Option<char>,
        garb: Option<u64>,
    ) -> PyResult<Vec<String>> {
        let solves = py.detach(|| {
            self.qbf.compute(
                queue,
                &BrokenBoard::from_garbage(garb.unwrap_or(0)),
                save.and_then(parse_shape),
            )
        });

        let res: Vec<String> = solves
            .iter()
            .map(|solve| {
                let mut board_str = String::with_capacity(40);
                solver::print(solve, &mut board_str);
                board_str
            })
            .collect();

        Ok(res)
    }

    #[pyo3(signature = (build_queue, solve_queue, saves="", skip_4p=false))]
    fn find_qb(
        &mut self,
        py: Python,
        build_queue: &str,
        solve_queue: &str,
        saves: &str,
        skip_4p: bool,
    ) -> PyResult<(Vec<String>, usize)> {
        self.qbf.skip_4p = skip_4p;
        let (setups, save_count) =
            py.detach(|| self.qbf.find(build_queue, None, solve_queue, saves, 1));

        let res: Vec<String> = setups
            .iter()
            .map(|solve| {
                let mut board_str = String::with_capacity(40);
                solver::print(solve, &mut board_str);
                board_str
            })
            .collect();

        Ok((res, save_count))
    }

    #[pyo3(signature = (fifth))]
    fn bestsaves(&mut self, py: Python, fifth: &str) -> PyResult<HashMap<String, Vec<String>>> {
        if fifth.len() != 2 {
            return Ok(HashMap::new());
        }
        fn bestsaves_queues(setup: &BrokenBoard, queue: &str, qbf: &QBFinder) -> Vec<String> {
            let pieces = "IJLOSZ";
            let mut res = HashSet::new();
            let save = Some(Shape::T);
            for (i, piece) in pieces.chars().enumerate() {
                let q = format!("{},T,{}", queue, piece);
                let solves = qbf.compute(
                    &q,
                    &BrokenBoard::from_garbage(setup.to_broken_bitboard().0),
                    save,
                );
                if solves.is_empty() {
                    return vec![];
                }
                let cover: HashSet<String> = solves
                    .iter()
                    .flat_map(|solve| {
                        solve
                            .supporting_queues(srs_4l::gameplay::Physics::Jstris)
                            .iter()
                            .filter_map(|q| {
                                let mut shapes: Vec<Shape> = q.collect();
                                if shapes.last().copied() == parse_shape(piece) {
                                    shapes.pop();
                                    Some(shapes.into_iter().collect::<Queue>())
                                } else {
                                    None
                                }
                            })
                            .flat_map(|q| q.unhold())
                            .map(|q| q.to_string())
                            .collect::<Vec<_>>()
                    })
                    .collect();
                if i == 0 {
                    res = cover;
                } else {
                    res = res.intersection(&cover).cloned().collect();
                }
            }
            res.iter().cloned().collect::<Vec<String>>()
        }
        let pieces = "TIJLOSZ";

        let perms: Vec<_> = pieces.chars().permutations(3).collect();

        let res: DashMap<String, Vec<String>> = DashMap::new();

        py.detach(|| {
            perms.into_par_iter().for_each(|p3| {
                let p3_str: String = p3.iter().collect();
                let q = format!("{},{}", fifth, p3_str);

                for save in fifth.chars().chain(p3.iter().copied()).unique() {
                    let setups =
                        self.qbf
                            .compute(&q, &BrokenBoard::from_garbage(0), parse_shape(save));
                    let remaining: String = pieces.chars().filter(|c| !p3.contains(c)).collect();
                    let qqq = format!("{},{}", save, remaining);

                    for setup in setups {
                        let queues = bestsaves_queues(&setup, &qqq, &self.qbf);
                        if queues.is_empty() {
                            continue;
                        }

                        let mut board_str = String::with_capacity(40);
                        solver::print(&setup, &mut board_str);

                        for queue in queues {
                            if !queue.starts_with(save) {
                                continue;
                            }

                            let fullq = format!("{}{}", p3_str, queue.replacen(save, "", 1));

                            res.entry(fullq).or_default().push(board_str.clone());
                        }
                    }
                }
            });
        });
        let res: HashMap<String, Vec<String>> = res.into_iter().collect();

        Ok(res)
    }
}

#[pymodule]
fn py_qbf(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<QBSolver>()?;
    Ok(())
}
