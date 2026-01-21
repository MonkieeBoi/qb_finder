use std::{collections::HashSet, time::Instant};

use itertools::Itertools;
use srs_4l::brokenboard::BrokenBoard;

use qb_finder::QBFinder;

use crate::qb_finder::{expand_pattern, parse_shape};

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
    let mut solveq = "OIL".to_owned();
    let save = 'T';
    let pieces = "TIOLJSZ";
    let remaining = pieces
        .chars()
        .filter(|&c| !solveq.contains(c))
        .collect::<String>();
    if !remaining.is_empty() {
        solveq = format!("{solveq},{remaining}");
    }

    let qbf = QBFinder::new();
    let start = Instant::now();
    let setups = qbf.find(buildq, &solveq, save);

    println!("Found {:?} setups in {:?}", setups.len(), start.elapsed());
    // for board in &setups {
    //     print_board(&board);
    // }

    let solve_queues: HashSet<String> = expand_pattern(&solveq).into_iter().collect();
    for (board, count) in setups
        .iter()
        .map(|b| {
            (b, {
                if b.pieces.len() == buildq.replace(",", "").len() - 1 {
                    let xor = buildq
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
                    qbf.min_count(
                        b,
                        &(r.clone() + &solveq),
                        &solve_queues.clone().iter().map(|q| r.clone() + q).collect(),
                        parse_shape(save),
                    )
                } else {
                    qbf.min_count(b, &solveq, &solve_queues, parse_shape(save))
                }
            })
        })
        .sorted_by_key(|(_, count)| *count)
    {
        print_board(board);
        println!("Min count: {}\n", count);
    }
}
