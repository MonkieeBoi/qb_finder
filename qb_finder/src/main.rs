use std::time::Instant;

use srs_4l::brokenboard::BrokenBoard;

use qb_finder::QBFinder;

pub mod minimals;
pub mod qb_finder;
pub mod queue;
pub mod solver;

fn emoji_map(c: char) -> char {
    match c {
        'I' => '📘',
        'J' => '🟦',
        'L' => '🟧',
        'O' => '🟨',
        'S' => '🟩',
        'T' => '🟪',
        'Z' => '🟥',
        'G' => '⬜',
        '_' => '⬛',
        _ => c,
    }
}

fn print_board(board: &BrokenBoard) {
    let mut str = String::new();
    solver::print(&board, &mut str);
    str = str
        .chars()
        .map(emoji_map)
        .collect::<Vec<char>>()
        .chunks(10)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<String>>()
        .join("\n");
    println!("{str}")
}

fn main() {
    let buildq = "TLSZ";
    let solveq = "OIL,TJSZ";
    let save = 'T';

    let start_load = Instant::now();

    let qbf = QBFinder::new();
    let setups = qbf.find(buildq, solveq, save);

    println!(
        "Found {:?} setups in {:?}",
        setups.len(),
        start_load.elapsed()
    );
    for board in &setups {
        print_board(&board);
    }

    // FIX FOR 3p LATER
    // let solve_queues: HashSet<String> = expand_pattern(solveq).into_iter().collect();
    // for (count, board) in setups
    //     .iter()
    //     .map(|b| {
    //         (
    //             qbf.min_count(
    //                 b,
    //                 solveq,
    //                 &solve_queues,
    //                 parse_shape(save),
    //                 solveq.chars().filter(|c| *c == save).count(),
    //             ),
    //             b,
    //         )
    //     })
    //     .sorted_by_key(|(count, _)| *count)
    // {
    //     print_board(board);
    //     println!("Min count: {}\n", count);
    // }
}
