use chess::{Board, ChessMove, MoveGen, Piece, BoardStatus, Square};
use shakmaty::{Chess, Position, uci::Uci, san::San};
use rayon::prelude::*;
use std::sync::Mutex;
use std::env;
use std::io;
use std::io::Write;


fn parse_steno_string(steno: &str) -> Result<Vec<char>, String> {
    let valid_chars = [
        '~', '1', '2', '3', '4', '5', '6', '7', '8', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'x',
        '+', '#', 'L', 'N', 'R', 'Q', 'K', 'P', '%', '=', 'o', '0', 'r', 'n', 'l', 'q'
    ];
    let mut parsed_chars = Vec::new();

    for ch in steno.chars() {
        if valid_chars.contains(&ch) {
            parsed_chars.push(ch);
        } else {
            return Err(format!("Invalid character in steno string: {}", ch));
        }
    }

    Ok(parsed_chars)
}

fn check_steno_constraints(board: &Board, last_move: Option<ChessMove>, last_piece_moved: Option<Piece>, piece_on_dest: Option<Piece>, depth: u8, steno_constraints: &[char]) -> bool {
    if last_move.is_none() {
        return true;
    }

    let constraint = steno_constraints[(depth - 1) as usize];
    let last_move_unwrapped = last_move.unwrap();
    let dest_square = last_move_unwrapped.get_dest();
    let source_square = last_move_unwrapped.get_source();
    match constraint {
        '~' => true,
        '1'..='8' => dest_square.get_rank().to_index() == constraint.to_digit(10).unwrap() as usize - 1,
        'a'..='h' => dest_square.get_file().to_index() == constraint as usize - 'a' as usize,
        '+' => board.checkers().count() > 0,
        '#' => matches!(board.status(), BoardStatus::Checkmate),
        'L' => last_piece_moved.unwrap() == Piece::Bishop,
        'N' => last_piece_moved.unwrap() == Piece::Knight,
        'R' => last_piece_moved.unwrap() == Piece::Rook,
        'Q' => last_piece_moved.unwrap() == Piece::Queen,
        'K' => last_piece_moved.unwrap() == Piece::King,
        'P' => last_piece_moved.unwrap() == Piece::Pawn,
        'x' => {
            if piece_on_dest.is_some() {
                return true;
            }
            if let Some(last_piece) = last_piece_moved {
                // Check en passant
                if last_piece == Piece::Pawn {
                    let source_square = last_move_unwrapped.get_source();
                    let dest_square = last_move_unwrapped.get_dest();
                    let is_diagonal_move = source_square.get_file() != dest_square.get_file();

                    return is_diagonal_move && piece_on_dest.is_none();
                }
            }
            return false;
        }
        '%' => {
            if let Some(last_piece) = last_piece_moved {
                // Check en passant
                if last_piece == Piece::Pawn {
                    let source_square = last_move_unwrapped.get_source();
                    let dest_square = last_move_unwrapped.get_dest();
                    let is_diagonal_move = source_square.get_file() != dest_square.get_file();

                    return is_diagonal_move && piece_on_dest.is_none();
                }
            }
            return false;
        }
        '=' => matches!(board.status(), BoardStatus::Stalemate),
        'o' => {
            (last_piece_moved.unwrap() == Piece::King) &&
                ((source_square == Square::E1 && dest_square == Square::G1) || // White castling kingside
                    (source_square == Square::E8 && dest_square == Square::G8)) // Black castling kingside
        }
        '0' => {
            (last_piece_moved.unwrap() == Piece::King) &&
                ((source_square == Square::E1 && dest_square == Square::C1) || // White castling queenside
                    (source_square == Square::E8 && dest_square == Square::C8)) // Black castling queenside
        }
        'r' => {
            let promotion = last_move_unwrapped.get_promotion();
            promotion == Some(Piece::Rook)
        }
        'n' => {
            let promotion = last_move_unwrapped.get_promotion();
            promotion == Some(Piece::Knight)
        }
        'l' => {
            let promotion = last_move_unwrapped.get_promotion();
            promotion == Some(Piece::Bishop)
        }
        'q' => {
            let promotion = last_move_unwrapped.get_promotion();
            promotion == Some(Piece::Queen)
        }
        _ => false,
    }
}

fn enumerate_positions(board: Board, depth: u8, path: Vec<ChessMove>, last_move: Option<ChessMove>, last_piece_moved: Option<Piece>, piece_on_dest: Option<Piece>, results: &Mutex<u32>, steno_constraints: &[char]) {
    if !check_steno_constraints(&board, last_move, last_piece_moved, piece_on_dest, depth, steno_constraints) {
        return;
    }

    if depth as usize == steno_constraints.len() {
        let mut num_results = results.lock().unwrap();
        *num_results += 1;

        let mut moves = Vec::new();
        let mut position = Chess::default();
        for mov in &path {
            let uci: Uci = mov.to_string().parse().unwrap();
            let uci_move = uci.to_move(&position).unwrap();
            let san_move = San::from_move(&position, &uci_move);
            moves.push(san_move.to_string());
            position = position.clone().play(&uci_move).unwrap();
        }

        let lichess_url = format!("https://lichess.org/analysis/pgn/{}", moves.join("_"));

        let stdout = io::stdout();
        let mut handle = stdout.lock();
        writeln!(handle, "{}", lichess_url).unwrap();
        return;
    }

    let moves: Vec<ChessMove> = MoveGen::new_legal(&board).collect();

    moves.par_iter().for_each(|&mov| {
        let mut new_board = board.clone();
        let piece_moved = board.piece_on(mov.get_source());
        let piece_on_dest = board.piece_on(mov.get_dest());
        board.make_move(mov, &mut new_board);
        let mut new_path = path.clone();
        new_path.push(mov);

        enumerate_positions(new_board, depth + 1, new_path, Some(mov), piece_moved, piece_on_dest, results, steno_constraints);
    });
}

fn solve(steno_constraints: &[char]) {
    let board = Board::default();
    let results = Mutex::new(0);
    enumerate_positions(board, 0, Vec::new(), None, None, None, &results, &steno_constraints);

    let solutions_count = results.lock().unwrap();
    println!("Number of solutions found: {}", solutions_count);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: steno_solver <steno_string>");
        return;
    }
    let steno_string = &args[1];

    match parse_steno_string(steno_string) {
        Ok(steno_constraints) => solve(&steno_constraints),
        Err(err) => eprintln!("{}", err),
    }
}
