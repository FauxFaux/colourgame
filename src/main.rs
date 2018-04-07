extern crate cast;
extern crate rand;

use std::cmp;
use std::collections::BinaryHeap;
use std::fmt;

use cast::f32;
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
        self.cells[y + SIZE * x]
    }

    fn set(&mut self, x: usize, y: usize, val: Colour) {
        self.cells[y + SIZE * x] = val;
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

struct State {
    score: f32,
    moves: Vec<Colour>,
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
        self.score.partial_cmp(&other.score).unwrap()
    }
}

fn walk(init: Board) {
    let mut best_moves = MAX_MOVES;
    let mut todo = BinaryHeap::with_capacity(10_000);

    todo.push(State {
        score: 0.,
        moves: Vec::new(),
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
                score: f32(score) / f32(solution.len()),
                moves: solution,
                board: item,
            })
        }
    }
}

fn complete(remaining: usize, solution: &[Colour]) {
    println!("{}: {:?} ({})", solution.len(), solution, remaining);
}

impl fmt::Debug for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for y in 0..SIZE {
            for x in 0..SIZE {
                write!(
                    f,
                    "{}",
                    match self.get(x, y) {
                        0 => '-',
                        1 => '#',
                        2 => 'N',
                        3 => 'o',
                        4 => 'T',
                        5 => 'v',
                        MARKER => ' ',
                        _ => unimplemented!(),
                    }
                )?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

fn main() {
    //    let init = Board::random();
    let init = Board {
        cells: [
            0, 0, 1, 1, 1, 0, 2, 5, 0, 2, 2, 4, 1, 5, 1, 1, 4, 1, 1, 5, 5, 5, 5, 5, 5, 3, 3, 1, 0,
            3, 0, 1, 4, 5, 1, 0, 2, 1, 1, 0, 2, 2, 5, 0, 0, 4, 4, 4, 1, 0, 3, 5, 4, 4, 1, 3, 0, 4,
            2, 1, 5, 0, 1, 2, 3, 2, 3, 2, 2, 3, 2, 3, 5, 2, 4, 0, 4, 4, 2, 1, 4, 0, 4, 1, 5, 5, 0,
            4, 3, 5, 5, 0, 5, 5, 2, 0, 0, 2, 4, 5, 0, 5, 5, 4, 4, 3, 3, 5, 0, 5, 4, 0, 4, 3, 4, 2,
            3, 0, 4, 2, 2, 5, 5, 1, 4, 2, 4, 1, 0, 1, 0, 4, 2, 1, 1, 2, 0, 1, 4, 5, 1, 0, 4, 2,
        ],
    };

    println!("{:?}", init.cells.iter().cloned().collect::<Vec<Colour>>());
    println!("{:?}", init);
    walk(init);
}
