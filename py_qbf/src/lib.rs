use pyo3::prelude::*;
use qb_finder_core::{QBFinder, parse_shape, solver};
use rustc_hash::FxHashSet;
use srs_4l::{board_list, brokenboard::BrokenBoard, gameplay::Board};
use std::io::Cursor;

#[pyclass]
struct QBSolver {
    qbf: QBFinder,
}

#[pymethods]
impl QBSolver {
    #[new]
    fn new() -> PyResult<Self> {
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

    #[pyo3(signature = (build_queue, solve_queue, saves=""))]
    fn find_qb(
        &self,
        py: Python,
        build_queue: &str,
        solve_queue: &str,
        saves: &str,
    ) -> PyResult<(Vec<String>, usize)> {
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
}

#[pymodule]
fn py_qbf(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<QBSolver>()?;
    Ok(())
}
