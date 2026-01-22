use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;

use srs_4l::{
    brokenboard::BrokenBoard,
    gameplay::{Board, Physics, Shape},
    vector::Placements,
};

use crate::queue::{Bag, QueueState};

type ScanStage = FxHashMap<Board, (SmallVec<[QueueState; 7]>, SmallVec<[Board; 6]>)>;

fn scan(
    legal_boards: &FxHashSet<Board>,
    start: Board,
    bags: &[Bag],
    _piece_count: usize,
    can_hold: bool,
    place_last: bool,
    physics: Physics,
    save: Option<Shape>,
) -> Vec<ScanStage> {
    let mut stages = Vec::new();

    let mut prev: ScanStage = FxHashMap::default();
    prev.insert(start, (bags.first().unwrap().init_hold(), SmallVec::new()));

    for (bag, i) in bags
        .iter()
        .flat_map(|b| (0..b.count).into_iter().map(move |i| (b, i)))
        .skip(1)
    {
        let mut next: ScanStage =
            FxHashMap::with_capacity_and_hasher(prev.len(), Default::default());

        for (&old_board, (old_queues, _preds)) in prev.iter() {
            for shape in Shape::ALL {
                let is_first = i == 0;
                let new_queues = bag.take(old_queues, shape, is_first, can_hold);

                if new_queues.is_empty() {
                    continue;
                }

                for (_, new_board) in Placements::place(old_board, shape, physics).canonical() {
                    if !legal_boards.is_empty() && !legal_boards.contains(&new_board) {
                        continue;
                    }

                    let (queues, preds) = next.entry(new_board).or_default();
                    if !preds.contains(&old_board) {
                        preds.push(old_board);
                    }
                    for &queue in &new_queues {
                        if !queues.contains(&queue) {
                            queues.push(queue);
                        }
                    }
                }
            }
        }

        stages.push(prev);
        prev = next;
    }

    if place_last {
        let mut next: ScanStage =
            FxHashMap::with_capacity_and_hasher(prev.len(), Default::default());

        for (&old_board, (old_queues, _preds)) in prev.iter() {
            for shape in Shape::ALL {
                if old_queues.iter().any(|queue| queue.hold() == Some(shape)) {
                    for (_, new_board) in Placements::place(old_board, shape, physics).canonical() {
                        if !legal_boards.is_empty() && !legal_boards.contains(&new_board) {
                            continue;
                        }

                        let (_queues, preds) = next.entry(new_board).or_default();
                        if !preds.contains(&old_board) {
                            preds.push(old_board);
                        }
                    }
                }
            }
        }

        stages.push(prev);
        prev = next;
    }

    if !place_last && let Some(s) = save {
        prev.retain(|_, (old_queues, _)| old_queues.iter().any(|q| q.hold() == Some(s)));
    }

    stages.push(prev);
    stages
}

fn cull(scanned: &[ScanStage]) -> FxHashSet<Board> {
    let mut culled = FxHashSet::with_capacity_and_hasher(scanned.len(), Default::default());

    let mut iter = scanned.iter().rev();

    if let Some(final_stage) = iter.next() {
        for (&board, (_queues, preds)) in final_stage.iter() {
            culled.insert(board);
            culled.extend(preds);
        }
    }

    for stage in iter {
        for (&board, (_queues, preds)) in stage.iter() {
            if culled.contains(&board) {
                culled.extend(preds);
            }
        }
    }

    culled
}

fn place(
    culled: &FxHashSet<Board>,
    start: BrokenBoard,
    bags: &[Bag],
    _piece_count: usize,
    can_hold: bool,
    place_last: bool,
    physics: Physics,
    save: Option<Shape>,
) -> FxHashMap<BrokenBoard, SmallVec<[QueueState; 7]>> {
    let mut prev = FxHashMap::default();
    prev.insert(start, bags.first().unwrap().init_hold());

    for (bag, i) in bags
        .iter()
        .flat_map(|b| (0..b.count).into_iter().map(move |i| (b, i)))
        .skip(1)
    {
        let mut next: FxHashMap<BrokenBoard, SmallVec<[QueueState; 7]>> =
            FxHashMap::with_capacity_and_hasher(prev.len(), Default::default());

        for (old_board, old_queues) in prev.iter() {
            for shape in Shape::ALL {
                let is_first = i == 0;
                let new_queues = bag.take(old_queues, shape, is_first, can_hold);

                if new_queues.is_empty() {
                    continue;
                }

                for (piece, new_board) in
                    Placements::place(old_board.board, shape, physics).canonical()
                {
                    if culled.contains(&new_board) {
                        let queues = next.entry(old_board.place(piece)).or_default();
                        for &queue in &new_queues {
                            if !queues.contains(&queue) {
                                queues.push(queue);
                            }
                        }
                    }
                }
            }
        }

        prev = next;
    }

    if place_last {
        let mut next: FxHashMap<BrokenBoard, SmallVec<[QueueState; 7]>> =
            FxHashMap::with_capacity_and_hasher(prev.len(), Default::default());

        for (old_board, old_queues) in prev.iter() {
            for shape in Shape::ALL {
                if old_queues.iter().any(|queue| queue.hold() == Some(shape)) {
                    for (piece, new_board) in
                        Placements::place(old_board.board, shape, physics).canonical()
                    {
                        if culled.contains(&new_board) {
                            next.insert(old_board.place(piece), SmallVec::new());
                        }
                    }
                }
            }
        }

        prev = next;
    }

    if let Some(s) = save {
        prev.retain(|_, queues| queues.iter().any(|q| q.hold() == Some(s)));
    }

    prev
}

pub fn compute(
    legal_boards: &FxHashSet<Board>,
    start: &BrokenBoard,
    bags: &[Bag],
    can_hold: bool,
    physics: Physics,
    save: Option<Shape>,
) -> Vec<BrokenBoard> {
    if bags.is_empty() {
        return vec![start.clone()];
    }

    let has_save = save.map_or(false, |s| bags.iter().any(|b| b.contains(s)));

    let piece_count = bags.iter().map(|b| b.count as usize).sum();
    let new_mino_count = piece_count as u32 * 4;
    let place_last = !has_save && start.board.0.count_ones() + new_mino_count <= 40;

    let scanned = scan(
        legal_boards,
        start.board,
        bags,
        piece_count,
        can_hold,
        place_last,
        physics,
        save,
    );
    let culled = cull(&scanned);
    let mut placed = place(
        &culled,
        start.clone(),
        bags,
        piece_count,
        can_hold,
        place_last,
        physics,
        save,
    );

    let mut solutions: Vec<BrokenBoard> =
        placed.drain().map(|(board, _queue_states)| board).collect();
    solutions.sort_unstable();

    solutions
}

pub fn print(board: &BrokenBoard, to: &mut String) {
    let pieces: Vec<(Shape, Board)> = board
        .pieces
        .iter()
        .map(|&piece| (piece.shape, piece.board()))
        .collect();
    let bits = board.to_broken_bitboard();

    for row in (0..4).rev() {
        'cell: for col in 0..10 {
            for &(shape, board) in &pieces {
                if board.get(row, col) {
                    to.push_str(shape.name());
                    continue 'cell;
                }
            }

            if bits.get(row, col) {
                to.push('G');
            } else {
                to.push('_');
            }
        }
    }
}
