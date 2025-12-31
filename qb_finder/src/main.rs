use std::collections::HashSet;
use std::fs::File;
use std::io::{Cursor, Read};

use srs_4l::{
    board_list,
    brokenboard::BrokenBoard,
    gameplay::{Board, Physics, Shape},
};

use crate::queue::Bag;

pub mod queue;
pub mod solver;

fn parse_pattern(pattern: &str) -> Vec<Bag> {
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

fn print_board(board: &BrokenBoard) {
    let mut table = [' '; 256];
    for i in 0..256 {
        table[i] = i as u8 as char;
    }
    table['I' as usize] = '📘';
    table['J' as usize] = '🟦';
    table['L' as usize] = '🟧';
    table['O' as usize] = '🟨';
    table['S' as usize] = '🟩';
    table['T' as usize] = '🟪';
    table['Z' as usize] = '🟥';
    table['G' as usize] = '⬜';
    table['_' as usize] = '⬛';

    let mut str = String::new();
    solver::print(&board, &mut str);
    str = str
        .chars()
        .map(|c| {
            if (c as usize) < 256 {
                table[c as usize]
            } else {
                c
            }
        })
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

    let boards: HashSet<Board> = board_list::read(Cursor::new(buffer))
        .unwrap()
        .into_iter()
        .collect();

    println!("Loaded legal_boards.");

    let field_hash = 0;
    let start = BrokenBoard::from_garbage(field_hash);

    let queue = parse_pattern("TLSZ");

    let solutions = solver::compute(&boards, &start, &queue, true, Physics::Jstris);

    for board in &solutions {
        print_board(&board);
        println!();
    }
}
