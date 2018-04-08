extern crate num_traits;
extern crate rand;

use std::cmp;
use std::collections::BinaryHeap;
use std::fmt;
use std::mem;
use std::ops;

use num_traits::Zero;
use rand::Rng;

const MAX_MOVES: usize = 23;
const SIZE: usize = 12;
const COLOURS: Colour = 6;
const MARKER: Colour = Colour::max_value();

type Score = usize;
type Colour = u8;
type Cells = [Colour; SIZE * SIZE];

#[derive(Copy, Clone)]
struct Board {
    cells: Cells,
}

#[derive(Copy, Clone)]
struct TinyVec<T> {
    elements: [T; MAX_MOVES],
    len: usize,
}

type Block = usize;

const COVERED_BLOCK_BITS: usize = mem::size_of::<Block>() * 8;

// TODO: lazy maths, may overshoot
const COVERED_STORAGE: usize = (SIZE * SIZE) / COVERED_BLOCK_BITS + 1;
#[derive(Copy, Clone)]
struct Covered {
    inner: [Block; COVERED_STORAGE],
}

impl Board {
    fn random() -> Board {
        let mut rand = rand::thread_rng();
        let mut cells = [0; SIZE * SIZE];
        for cell in cells.iter_mut() {
            *cell = rand.gen_range(0, COLOURS)
        }

        let mut board = Board { cells };

        let start = board.get(0, 0);
        if rand.gen() {
            board.set(0, 1, start);
        } else {
            board.set(1, 0, start)
        }

        board
    }

    fn get(&self, x: usize, y: usize) -> Colour {
        self.cells[coord(x, y)]
    }

    fn get_raw(&self, pos: usize) -> Colour {
        self.cells[pos]
    }

    fn set(&mut self, x: usize, y: usize, val: Colour) {
        self.cells[coord(x, y)] = val;
    }

    fn mark(mut self) -> Board {
        let src = self.get(0, 0);

        let mut todo = Vec::with_capacity(80);
        push_adjacents(&mut todo, 0, 0);

        while let Some((x, y)) = todo.pop() {
            if self.get(x, y) != src {
                continue;
            }
            push_adjacents(&mut todo, x, y);
            self.set(x, y, MARKER);
        }

        self
    }

    fn marked_replace(mut self, target: Colour) -> Board {
        self.cells
            .iter_mut()
            .filter(|&&mut cell| MARKER == cell)
            .for_each(|cell| *cell = target);
        self
    }

    fn marked_score(&self) -> usize {
        self.cells
            .into_iter()
            .filter(|&&cell| MARKER == cell)
            .count()
    }

    fn remaining_colours(&self) -> usize {
        let mut seen = [false; COLOURS as usize];
        for &cell in self.cells.into_iter() {
            seen[usize::from(cell)] = true;
        }

        let mut count = 0;

        for &colour in &seen {
            if colour {
                count += 1;
            }
        }

        count
    }
}

impl Covered {
    fn new() -> Covered {
        let inner = [0; COVERED_STORAGE];
        let mut covered = Covered { inner };
        covered.set(0, 0);
        covered
    }

    fn get(&self, x: usize, y: usize) -> bool {
        self.get_raw(coord(x, y))
    }

    fn get_raw(&self, pos: usize) -> bool {
        let block = pos / COVERED_BLOCK_BITS;
        let bit = pos % COVERED_BLOCK_BITS;
        let mask = 1 << bit;
        self.inner[block] & mask == mask
    }

    fn set(&mut self, x: usize, y: usize) {
        self.set_raw(coord(x, y));
    }

    fn set_raw(&mut self, pos: usize) {
        let block = pos / COVERED_BLOCK_BITS;
        let bit = pos % COVERED_BLOCK_BITS;
        let mask = 1 << bit;
        self.inner[block] |= mask;
    }
}

fn expand_coverage(board: &Board, coverage: &Covered, colour: Colour) -> Covered {
    let mut new = *coverage;
    fill2(board, &mut new, colour);
    new
}

fn fill2(board: &Board, coverage: &mut Covered, colour: Colour) {
    let mut todo = Vec::with_capacity(40);
    for pos in 0..(SIZE * SIZE) {
        if !coverage.get_raw(pos) {
            continue;
        }
        push_adjacents_raw(&mut todo, pos);
    }

    todo.sort_unstable();
    todo.dedup();

    while let Some(pos) = todo.pop() {
        if coverage.get_raw(pos) || board.get_raw(pos) != colour {
            continue;
        }

        coverage.set_raw(pos);

        push_adjacents_raw(&mut todo, pos);
    }
}

fn coord(x: usize, y: usize) -> usize {
    x + SIZE * y
}

fn push_adjacents(onto: &mut Vec<(usize, usize)>, x: usize, y: usize) {
    if x > 0 {
        onto.push((x - 1, y));
    }

    if y > 0 {
        onto.push((x, y - 1))
    }

    if x < SIZE - 1 {
        onto.push((x + 1, y));
    }

    if y < SIZE - 1 {
        onto.push((x, y + 1));
    }
}

fn push_adjacents_raw(onto: &mut Vec<usize>, pos: usize) {
    let x = pos % SIZE;
    let y = pos / SIZE;

    if x > 0 {
        onto.push(coord(x - 1, y));
    }

    if y > 0 {
        onto.push(coord(x, y - 1))
    }

    if x < SIZE - 1 {
        onto.push(coord(x + 1, y));
    }

    if y < SIZE - 1 {
        onto.push(coord(x, y + 1));
    }
}

fn step(board: Board) -> impl Iterator<Item = (Score, Board)> {
    let marked = board.mark();
    let init_score = marked.marked_score();
    (0..COLOURS)
        .filter(move |&colour| colour != board.get(0, 0))
        .filter_map(move |colour| {
            let cand = marked.marked_replace(colour);
            let new_score = cand.mark().marked_score();
            if new_score > init_score {
                Some((new_score, cand))
            } else {
                None
            }
        })
}

#[derive(Copy, Clone)]
struct State {
    score: Score,
    moves: TinyVec<Colour>,
    board: Board,
}

impl cmp::Eq for State {}

impl cmp::PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl cmp::PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.score.partial_cmp(&other.score)
    }
}

impl cmp::Ord for State {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.score.cmp(&other.score)
    }
}

fn walk(init: Board) {
    let mut best_moves = MAX_MOVES;
    let mut todo = BinaryHeap::with_capacity(10_000);

    todo.push(State {
        score: 0,
        moves: TinyVec::new(),
        board: init,
    });

    while let Some(State {
        score: _,
        moves,
        board,
    }) = todo.pop()
    {
        if moves.len() + board.remaining_colours() > best_moves {
            continue;
        }

        for (score, item) in step(board) {
            let mut solution = moves.clone();
            solution.push(item.get(0, 0));

            if score == SIZE * SIZE {
                best_moves = solution.len();
                complete(todo.len(), &solution);
                break;
            }

            todo.push(State {
                score,
                moves: solution,
                board: item,
            })
        }
    }
}

fn complete(remaining: usize, solution: &[Colour]) {
    println!(
        "{}: {} ({})",
        solution.len(),
        solution
            .into_iter()
            .cloned()
            .map(symbol)
            .collect::<String>(),
        remaining
    );
}

fn symbol(colour: Colour) -> char {
    match colour {
        0 => '-',
        1 => '#',
        2 => 'N',
        3 => 'o',
        4 => 'T',
        5 => 'v',
        MARKER => ' ',
        _ => unimplemented!(),
    }
}

impl fmt::Debug for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for y in 0..SIZE {
            for x in 0..SIZE {
                write!(f, "{}", symbol(self.get(x, y)))?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl fmt::Debug for Covered {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for y in 0..SIZE {
            for x in 0..SIZE {
                write!(f, "{}", if self.get(x, y) { 'X' } else { '.' })?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl<T: Copy + Zero> TinyVec<T> {
    fn new() -> TinyVec<T> {
        TinyVec {
            elements: [T::zero(); MAX_MOVES],
            len: 0,
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    fn push(&mut self, element: T) {
        self.elements[self.len] = element;
        self.len += 1;
    }
}

impl<T> ops::Deref for TinyVec<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        &self.elements[0..self.len]
    }
}

fn main() {
    #[cfg(never)]
    let init = Board::random();
    let init = Board {
        cells: [
            0, 0, 1, 1, 1, 0, 2, 5, 0, 2, 2, 4, 1, 5, 1, 1, 4, 1, 1, 5, 5, 5, 5, 5, 5, 3, 3, 1, 0,
            3, 0, 1, 4, 5, 1, 0, 2, 1, 1, 0, 2, 2, 5, 0, 0, 4, 4, 4, 1, 0, 3, 5, 4, 4, 1, 3, 0, 4,
            2, 1, 5, 0, 1, 2, 3, 2, 3, 2, 2, 3, 2, 3, 5, 2, 4, 0, 4, 4, 2, 1, 4, 0, 4, 1, 5, 5, 0,
            4, 3, 5, 5, 0, 5, 5, 2, 0, 0, 2, 4, 5, 0, 5, 5, 4, 4, 3, 3, 5, 0, 5, 4, 0, 4, 3, 4, 2,
            3, 0, 4, 2, 2, 5, 5, 1, 4, 2, 4, 1, 0, 1, 0, 4, 2, 1, 1, 2, 0, 1, 4, 5, 1, 0, 4, 2,
        ],
    };

    for colour in 0..COLOURS {
        println!("{}: {}", colour, symbol(colour));
    }

    let coverage = Covered::new();
    let ex = expand_coverage(&init, &coverage, init.get(0, 0));
    println!("{:?}", ex);
    let ex = expand_coverage(&init, &ex, 1);
    println!("{:?}", ex);
    let ex = expand_coverage(&init, &ex, 0);
    println!("{:?}", ex);
    let ex = expand_coverage(&init, &ex, 1);
    println!("{:?}", ex);

    println!("{:?}", init.cells.iter().cloned().collect::<Vec<Colour>>());
    println!("{:?}", init);
    walk(init);
}
