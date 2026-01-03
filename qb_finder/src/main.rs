use itertools::Itertools;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::collections::HashSet;
use std::fs::File;
use std::io::{Cursor, Read};
use std::time::Instant;

use srs_4l::{
    board_list,
    brokenboard::BrokenBoard,
    gameplay::{Board, Physics, Shape},
};

use crate::minimals::min_cover_size;
use crate::queue::Bag;

pub mod minimals;
pub mod queue;
pub mod solver;

fn pattern_bags(pattern: &str) -> Vec<Bag> {
    let mut bags = Vec::new();
    for bag in pattern.split(",") {
        let shapes = bag
            .chars()
            .map(parse_shape)
            .collect::<Option<Vec<Shape>>>()
            .unwrap();
        bags.push(Bag::new(&shapes, bag.len() as u8));
    }
    bags
}

fn expand_pattern(pattern: &str) -> Vec<String> {
    pattern
        .split(",")
        .map(|group| {
            let len = group.len();
            group
                .chars()
                .permutations(len)
                .unique()
                .map(|p| p.into_iter().collect::<String>())
                .collect_vec()
        })
        .multi_cartesian_product()
        .map(|prod| prod.join(""))
        .collect()
}

fn parse_shape(shape: char) -> Option<Shape> {
    match shape {
        'I' => Some(Shape::I),
        'J' => Some(Shape::J),
        'L' => Some(Shape::L),
        'O' => Some(Shape::O),
        'S' => Some(Shape::S),
        'T' => Some(Shape::T),
        'Z' => Some(Shape::Z),
        _ => None,
    }
}

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

fn is_hundred(
    legal_boards: &HashSet<Board>,
    setup: &BrokenBoard,
    solve_queues: &Vec<Vec<Bag>>,
    save: Option<Shape>,
    solve_save_count: usize,
) -> bool {
    solve_queues.iter().all(|q| {
        let solves = solver::compute(&legal_boards, &setup, &q, true, Physics::Jstris);

        if solves.is_empty() {
            return false;
        }

        let Some(save_shape) = save else {
            return true;
        };

        solves.iter().any(|solve| {
            let save_count = solve
                .pieces
                .iter()
                .filter(|piece| piece.shape == save_shape)
                .count();
            solve_save_count == 0 || solve_save_count - 1 == save_count
        })
    })
}

fn min_count(
    legal_boards: &HashSet<Board>,
    setup: &BrokenBoard,
    pattern: &str,
    universe: &HashSet<String>,
    save: Option<Shape>,
    solve_save_count: usize,
) -> usize {
    let board = &BrokenBoard::from_garbage(setup.to_broken_bitboard().0);
    let mut solves = solver::compute(
        legal_boards,
        board,
        &pattern_bags(pattern),
        true,
        Physics::Jstris,
    );

    if let (Some(s), true) = (save, solve_save_count > 0) {
        let target = solve_save_count - 1;
        solves.retain(|sol| sol.pieces.iter().filter(|p| p.shape == s).count() == target);
    }

    let pattern_bytes = pattern.replace(",", "").into_bytes();

    let covering_queues: Vec<Vec<String>> = solves
        .into_iter()
        .map(|solve| {
            solve
                .supporting_queues(Physics::Jstris)
                .iter()
                .flat_map(|&q| {
                    let saved_piece = q
                        .map(|s| s.name().as_bytes()[0])
                        .chain(pattern_bytes.iter().copied())
                        .fold(0, |acc, b| acc ^ b) as char;
                    parse_shape(saved_piece).map_or_else(|| q.unhold(), |s| q.push_last(s).unhold())
                })
                .unique_by(|q| q.0)
                .map(|q| q.to_string())
                .filter(|q| universe.contains(q))
                .collect()
        })
        .collect();
    min_cover_size(universe, covering_queues)
}

fn main() {
    let mut file = File::open("./legal-boards.leb128").expect("Failed to open legal_boards");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .expect("Failed to read legal_boards");

    let boards: HashSet<Board> = board_list::read(Cursor::new(buffer))
        .unwrap()
        .into_iter()
        .collect();

    let field_hash = 0;
    let start = BrokenBoard::from_garbage(field_hash);
    let buildq = "TLSZ";
    let solveq = "OLJ,TISZ";
    let save = 'T';

    let mut setups = solver::compute(
        &boards,
        &start,
        &pattern_bags(buildq),
        true,
        Physics::Jstris,
    );

    let start_load = Instant::now();
    let solve_queues = expand_pattern(solveq)
        .into_iter()
        .map(|q| {
            q.chars()
                .map(|c| {
                    let shape = parse_shape(c).expect("Invalid solve pattern");
                    Bag::new(&[shape], 1)
                })
                .collect()
        })
        .collect();

    let save_count = solveq.chars().filter(|x| *x == save).count();
    let parsed_save = parse_shape(save);
    setups = setups
        .into_par_iter()
        .filter(|setup| {
            is_hundred(
                &boards,
                &BrokenBoard::from_garbage(setup.to_broken_bitboard().0),
                &solve_queues,
                parsed_save,
                save_count,
            )
        })
        .collect();

    println!(
        "Found {:?} setups in {:?}",
        setups.len(),
        start_load.elapsed()
    );

    let solve_queues: HashSet<String> = expand_pattern(solveq).into_iter().collect();
    for (count, board) in setups
        .iter()
        .map(|b| {
            (
                min_count(&boards, b, solveq, &solve_queues, parsed_save, save_count),
                b,
            )
        })
        .sorted_by_key(|(count, _)| *count)
    {
        print_board(board);
        println!("Min count: {}\n", count);
    }
}
