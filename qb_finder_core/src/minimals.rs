use good_lp::{Expression, Solution, SolverModel, microlp, variables};
use rustc_hash::FxHashSet;

pub fn min_cover_size<T: PartialEq>(universe: &FxHashSet<T>, sets: &Vec<Vec<T>>) -> usize {
    let mut vars = variables!();

    let set_vars: Vec<_> = sets
        .iter()
        .map(|_| vars.add(good_lp::variable().binary()))
        .collect();

    let objective: Expression = set_vars.iter().sum();
    let mut problem = vars.minimise(objective).using(microlp);

    for element in universe {
        let mut constraint_expr: Expression = 0.into();

        for (i, set) in sets.iter().enumerate() {
            if set.contains(element) {
                constraint_expr += set_vars[i];
            }
        }
        problem.add_constraint(constraint_expr.geq(1));
    }

    let solution = problem.solve();

    match solution {
        Ok(s) => set_vars
            .into_iter()
            .filter(|var| s.value(*var) > 0.5)
            .count(),
        Err(_) => 0,
    }
}

pub fn all_min_cover_sets<T: PartialEq>(
    universe: &FxHashSet<T>,
    sets: &Vec<Vec<T>>,
) -> Vec<Vec<usize>> {
    let min_size = min_cover_size(universe, &sets);
    let mut res = Vec::new();
    if min_size == 0 {
        return res;
    }

    let mut found_sets: Vec<Vec<usize>> = Vec::new();

    loop {
        let mut vars = variables!();
        let set_vars: Vec<_> = sets
            .iter()
            .map(|_| vars.add(good_lp::variable().binary()))
            .collect();

        let objective: Expression = set_vars.iter().sum();
        let mut problem = vars.minimise(objective.clone()).using(microlp);

        problem.add_constraint(objective.eq(min_size as f64));

        for element in universe {
            let mut constraint_expr = Expression::from(0.0);
            for (i, set) in sets.iter().enumerate() {
                if set.contains(element) {
                    constraint_expr = constraint_expr + set_vars[i];
                }
            }
            problem.add_constraint(constraint_expr.geq(1.0));
        }

        for found in &found_sets {
            let mut current_cut_expr = Expression::from(0.0);
            for &idx in found {
                current_cut_expr = current_cut_expr + set_vars[idx];
            }

            let rhs = (found.len() as f64) - 1.0;
            let cut_constraint = Expression::from(rhs) - current_cut_expr;
            problem.add_constraint(cut_constraint.geq(0.0));
        }

        match problem.solve() {
            Ok(solution) => {
                let mut current_indices = Vec::new();
                for (i, &var) in set_vars.iter().enumerate() {
                    if solution.value(var) > 0.5 {
                        current_indices.push(i);
                    }
                }

                res.push(current_indices.clone());
                found_sets.push(current_indices);
            }
            Err(_) => break,
        }
    }

    res
}
