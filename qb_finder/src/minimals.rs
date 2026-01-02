use good_lp::{Expression, Solution, SolverModel, microlp, variables};
use std::collections::BTreeSet;

pub fn min_cover_size<T: PartialEq>(universe: &BTreeSet<T>, sets: Vec<Vec<T>>) -> usize {
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
