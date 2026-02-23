use std::{
    fs::File,
    io::{self, Cursor, Read, Write},
    time::Instant,
};

use itertools::Itertools;
use qb_finder_core::{QBFinder, expand_pattern, parse_shape, solver};
use rustc_hash::FxHashSet;
use srs_4l::{board_list, brokenboard::BrokenBoard, gameplay::Board};

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
    let mut str = String::with_capacity(40);
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
    let mut file = File::open("./legal-boards.leb128").expect("Failed to open legal_boards");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .expect("Failed to read legal_boards");

    let legal_boards: FxHashSet<Board> = board_list::read(Cursor::new(buffer))
        .unwrap()
        .into_iter()
        .collect();

    let qbf = QBFinder::new(legal_boards);
    loop {
        print!("Build Queue: ");
        let _ = io::stdout().flush();
        let mut input = String::new();

        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        let buildq = input.trim();
        if buildq.len() < 1 {
            break;
        }

        print!("Solve Queue: ");
        let _ = io::stdout().flush();
        let mut input = String::new();

        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");
        let mut solveq = input.trim().to_owned();

        let save = 'T';
        let pieces = "TIOLJSZ";
        let remaining = pieces
            .chars()
            .filter(|&c| !solveq.contains(c))
            .collect::<String>();
        if !remaining.is_empty() {
            solveq = format!("{solveq},{remaining}");
        }

        let start = Instant::now();
        let setups = qbf.find(buildq, None, &solveq, save);

        println!("Found {:?} setups in {:?}", setups.len(), start.elapsed());
        // for board in &setups {
        //     print_board(&board);
        // }

        let solve_queues: FxHashSet<String> = expand_pattern(&solveq).into_iter().collect();
        for (board, count) in setups
            .iter()
            .map(|b| {
                (b, {
                    if b.pieces.len() < 3 {
                        0
                    } else if b.pieces.len() == buildq.replace(",", "").len() - 1 {
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
            if count > 0 {
                println!("Min count: {}\n", count);
            } else {
                println!("");
            }
        }
    }
}
