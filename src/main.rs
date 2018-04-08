extern crate cast;
extern crate num_traits;
extern crate rand;

use std::cmp;
use std::collections::BinaryHeap;
use std::fmt;
use std::mem;
use std::ops;

use cast::usize;
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

#[derive(Copy, Clone, PartialEq, Eq)]
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

    fn remaining_colours(&self, mask: &Covered) -> usize {
        let mut seen = [false; COLOURS as usize];
        for (pos, &cell) in self.cells.into_iter().enumerate() {
            if mask.get_raw(pos) {
                continue;
            }
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

    fn score(&self) -> Score {
        self.inner.iter().map(|x| usize(x.count_ones())).sum()
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
        push_adjacents_raw(&mut todo, pos, |pos| {
            coverage.get_raw(pos) || board.get_raw(pos) != colour
        });
    }

    todo.sort_unstable();
    todo.dedup();

    while let Some(pos) = todo.pop() {
        {
            let skip = |pos| coverage.get_raw(pos) || board.get_raw(pos) != colour;
            if skip(pos) {
                continue;
            }

            push_adjacents_raw(&mut todo, pos, skip);
        }

        coverage.set_raw(pos);
    }
}

fn coord(x: usize, y: usize) -> usize {
    x + SIZE * y
}

fn push_adjacents_raw<F>(onto: &mut Vec<usize>, pos: usize, skip: F)
where
    F: Fn(usize) -> bool,
{
    let x = pos % SIZE;
    let y = pos / SIZE;

    if x > 0 {
        let pos = coord(x - 1, y);
        if !skip(pos) {
            onto.push(pos);
        }
    }

    if y > 0 {
        let pos = coord(x, y - 1);
        if !skip(pos) {
            onto.push(pos)
        }
    }

    if x < SIZE - 1 {
        let pos = coord(x + 1, y);
        if !skip(pos) {
            onto.push(pos);
        }
    }

    if y < SIZE - 1 {
        let pos = coord(x, y + 1);
        if !skip(pos) {
            onto.push(pos);
        }
    }
}

fn step<'b>(
    board: &'b Board,
    mask: &'b Covered,
    skip_colour: Colour,
) -> impl Iterator<Item = (Colour, Covered)> + 'b {
    (0..COLOURS)
        .filter(move |&colour| colour != skip_colour)
        .filter_map(move |colour| {
            let cand = expand_coverage(board, mask, colour);
            if cand != *mask {
                Some((colour, cand))
            } else {
                None
            }
        })
}

#[derive(Copy, Clone)]
struct State {
    score: Score,
    moves: TinyVec<Colour>,
    mask: Covered,
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

fn walk(board: &Board) {
    let mut best_moves = MAX_MOVES;
    let mut todo = BinaryHeap::with_capacity(10_000);

    let root_colour = board.get(0, 0);
    let mask = expand_coverage(&board, &Covered::new(), root_colour);
    let mut moves = TinyVec::new();
    moves.push(root_colour);

    todo.push(State {
        score: 0,
        moves,
        mask,
    });

    while let Some(State {
        score: _,
        moves,
        mask,
    }) = todo.pop()
    {
        if moves.len() + board.remaining_colours(&mask) >= best_moves {
            continue;
        }

        for (colour, mask) in step(&board, &mask, moves.get(moves.len() - 1)) {
            let mut solution = moves.clone();
            solution.push(colour);
            let score = mask.score();

            if score == SIZE * SIZE {
                best_moves = solution.len();
                complete(todo.len(), &solution);
                break;
            }

            todo.push(State {
                score,
                moves: solution,
                mask,
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

    fn get(&self, idx: usize) -> T {
        self.elements[idx]
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

    println!("{:?}", init.cells.iter().cloned().collect::<Vec<Colour>>());
    println!("{:?}", init);
    walk(&init);
}
