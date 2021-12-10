use minifb::{MouseMode, Window, WindowOptions, ScaleMode, Scale};
extern crate cairo;
use cairo::{ ImageSurface, Format, Context };
use font_kit::family_name::FamilyName;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;
use image::io::Reader as ImageReader;
use image::imageops::FilterType;
use std::error::Error;
use std::fs::File;
use std::collections::HashMap;
use std::cmp;
use std::ops::{Not};

#[derive(Clone, Copy, Debug)]
pub struct ScreenPosition(
    pub f64,
    pub f64
);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BoardPosition(
    pub usize,
    pub usize
);

pub trait Drawable {
    fn draw(&self, dt: &mut Context, position: ScreenPosition) -> Result<(), Box<dyn Error>> {Ok(())}
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum Color {
    White,
    Black
}

impl Not for Color {
    type Output = Self;
    fn not(self) -> Self {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
enum Piece {
    Pawn(bool),
    Knight,
    Bishop,
    Rook,
    Queen,
    King
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
enum BoardSquare {
    Empty,
    Occupied(Piece, Color)
}

struct EmptyDrawable {}
impl Drawable for EmptyDrawable {}

pub struct PngDrawable {
	//pngImage: Source<'a>,
    surface: ImageSurface,
    data: Vec<u32>,
    width_ratio: f64, 
    height_ratio: f64,
}

impl PngDrawable {
    pub fn new(path: &str, width: f64, height: f64) -> Result<Self, Box<dyn Error>> {
        let mut openf = File::open(path)?;
        let imsurf = ImageSurface::create_from_png(&mut openf)?;

        let img_height = imsurf.height() as f64;
        let img_width = imsurf.width() as f64;
        let width_ratio = width / img_width;
        let height_ratio = height / img_height;

        Ok(PngDrawable {surface: imsurf, data: vec![], width_ratio, height_ratio})
    }
}

impl Drawable for PngDrawable {
    fn draw(&self, ctx: &mut Context, position: ScreenPosition) -> Result<(), Box<dyn Error>> {
        ctx.save()?;
        ctx.translate(position.0, position.1);
        ctx.scale(self.width_ratio, self.height_ratio);
        ctx.set_source_surface(&self.surface, 0.0, 0.0)?;
        ctx.paint()?;
        ctx.restore()?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct GameState {
	board_state: [[BoardSquare; 8]; 8],
	pub turn: Color,
    black_can_castle_left: bool,
    white_can_castle_left: bool,
    black_can_castle_right: bool,
    white_can_castle_right: bool,
    // If en passant is legal, the square of the pawn which can be captured via en passant.
    en_passant_square: Option<BoardPosition>,
}
impl GameState {
    fn new() -> Self {
        GameState {
            board_state: [[BoardSquare::Empty; 8]; 8],
            turn: Color::White,
            black_can_castle_left: true,
            white_can_castle_left: true,
            black_can_castle_right: true,
            white_can_castle_right: true,
            en_passant_square: None,
        }
    }
    fn piece_iterator<'a>(&'a self) -> Box<dyn Iterator<Item = (BoardPosition, BoardSquare)> + 'a> {
        Box::new((0..64).map(|x| {
            let pos = BoardPosition(x / 8, x % 8);
            (pos, self.board_state[pos.1][pos.0])
        }).filter(|&(_, state)| state != BoardSquare::Empty))
    }
	pub fn is_legal(&self, from_pos: BoardPosition, to_pos: BoardPosition) -> bool {
        if !self.is_legal_start(from_pos) {
            return false;
        }
        // Check square is not occupied by friendly pieces
        if self.is_legal_start(to_pos) {
            return false;
        }
        // Check basic legality of piece movement
        if !self.is_attacking_movement_ok(from_pos, to_pos, false) {
            let pos_diff = ((to_pos.0 as i64 - from_pos.0 as i64), (to_pos.1 as i64 - from_pos.1 as i64));
            // Check special rules (non-attacking pawn move, en passant, castling)
            let could_be_legal = match self.board_state[from_pos.1][from_pos.0] {
                BoardSquare::Occupied(Piece::Pawn(has_moved), _) => {
                    let pawn_dir = if self.turn == Color::White {-1} else {1};
                    if let BoardSquare::Occupied(_, _) = self.board_state[to_pos.1][to_pos.0] {
                        // Direct attack case already handled
                        false
                    } else {
                        if pos_diff.0 == 0 {
                            // (!has_moved && (0, 2) movement) || (0, 1) movement
                            pos_diff.1 == pawn_dir || (pos_diff.1 == pawn_dir * 2 && !has_moved)
                        } else {
                            // En passant case
                            if let Some(pass_pos) = self.en_passant_square {
                                pos_diff.0.abs() == 1 && pos_diff.1 == pawn_dir && to_pos.0 == pass_pos.0 && to_pos.1 == (pass_pos.1 as i64 + pawn_dir) as usize
                            } else {
                                false
                            }
                        }
                    }
                },
                BoardSquare::Occupied(Piece::King, _)  => {
                    let is_left = pos_diff.0 < 0;
                    if pos_diff.1 != 0 || pos_diff.0.abs() != 2 {
                        false
                    } else if (self.turn == Color::Black &&
                        ((is_left && self.black_can_castle_left) || (!is_left && self.black_can_castle_right))) || 
                        (self.turn == Color::White &&
                            ((is_left && self.white_can_castle_left) || (!is_left && self.white_can_castle_right))) {
                        let x = if is_left {0} else {7};
                        // valid rook move really means that there's nothing between rook and king here.
                        if self.is_valid_rook_move(from_pos, BoardPosition(x, from_pos.1)) {
                            // Check for castling through or out of check
                            !(self.is_square_attacked(from_pos, !self.turn) || self.is_square_attacked(BoardPosition((from_pos.0 as i64 + (pos_diff.0 / 2)) as usize, from_pos.1), !self.turn))
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                },
                _ => false
            };
            if !could_be_legal {
                return false;
            }
        }
        // Check legality of board state post-movement -- i.e. actually do the move, then see if our king becomes in check
        let mut test_state = (*self).clone();
        test_state.do_move(from_pos, to_pos);
        let (pos, _) = test_state.piece_iterator().filter(|&(_, sq)| sq == BoardSquare::Occupied(Piece::King, self.turn)).next().unwrap();
        if test_state.is_square_attacked(pos, !self.turn) {
            false
        } else {
            true
        }
    }
    fn is_square_attacked(&self, pos: BoardPosition, attacker: Color) -> bool {
        if let BoardSquare::Occupied(_, color) = self.board_state[pos.1][pos.0] {
            assert_ne!(color, attacker, "Color is same as attacker checking!");
        }
        for (piece_pos, boardsquare) in self.piece_iterator() {
            if let BoardSquare::Occupied(_, color) = boardsquare {
                if color != attacker {
                    continue;
                }
                if self.is_attacking_movement_ok(piece_pos, pos, true) {
                    return true;
                }
            } else {
                panic!("AAAAAAH");
            }
        }
        false
    }
    fn is_valid_rook_move(&self, from_pos: BoardPosition, to_pos: BoardPosition) -> bool {
        let pos_diff = ((to_pos.0 as i64 - from_pos.0 as i64), (to_pos.1 as i64 - from_pos.1 as i64));
        if pos_diff.0 != 0 && pos_diff.1 != 0 {
            false
        } else {
            if pos_diff.0 == 0 {
                let start = cmp::min(to_pos.1, from_pos.1);
                for i in start + 1..start + pos_diff.1.abs() as usize {
                    if let BoardSquare::Occupied(_, _) = self.board_state[i][from_pos.0] {
                        return false;
                    }
                }
            } else {
                let start = cmp::min(to_pos.0, from_pos.0);

                for i in start + 1..start + pos_diff.0.abs() as usize {
                    if let BoardSquare::Occupied(_, _) = self.board_state[from_pos.1][i] {
                        return false;
                    }
                }
            }
            true
        }
    }
    fn is_valid_bishop_move(&self, from_pos: BoardPosition, to_pos: BoardPosition) -> bool {
        let pos_diff = ((to_pos.0 as i64 - from_pos.0 as i64), (to_pos.1 as i64 - from_pos.1 as i64));
        if pos_diff.0.abs() != pos_diff.1.abs() {
            false
        } else {
            let i_dir : i64 = if pos_diff.0 > 0 {1} else {-1};
            let j_dir : i64 = if pos_diff.1 > 0 {1} else {-1};
            let mut pos = (from_pos.0 as i64 + i_dir, from_pos.1 as i64 + j_dir);
            while pos.0 != to_pos.0 as i64 {
                if let BoardSquare::Occupied(_, _) = self.board_state[pos.1 as usize][pos.0 as usize] {
                    return false;
                }
                pos.0 += i_dir;
                pos.1 += j_dir;
            }
            true
        }
    }

    // Assumes from_pos is occupied, and to_pos is occupied by opposite color or nothing
    fn is_attacking_movement_ok(&self, from_pos: BoardPosition, to_pos: BoardPosition, pretend_occupied: bool) -> bool {
        if let BoardSquare::Occupied(_, color) = self.board_state[from_pos.1][from_pos.0] {
            if let BoardSquare::Occupied(_, color2) = self.board_state[to_pos.1][to_pos.0] {
                if color == color2 {
                    panic!("Attacking wrong color");
                }
            }
        
            let pos_diff = ((to_pos.0 as i64 - from_pos.0 as i64), (to_pos.1 as i64 - from_pos.1 as i64));
            match self.board_state[from_pos.1][from_pos.0] {
                BoardSquare::Occupied(Piece::Knight, _) => (pos_diff.0.abs() == 2 && pos_diff.1.abs() == 1) || (pos_diff.0.abs() == 1 && pos_diff.1.abs() == 2),
                BoardSquare::Occupied(Piece::King, _) => pos_diff.0.abs() <= 1 && pos_diff.1.abs() <= 1,
                BoardSquare::Occupied(Piece::Rook, _) => self.is_valid_rook_move(from_pos, to_pos),
                BoardSquare::Occupied(Piece::Bishop, _) => self.is_valid_bishop_move(from_pos, to_pos),
                BoardSquare::Occupied(Piece::Queen, _) => self.is_valid_bishop_move(from_pos, to_pos) || self.is_valid_rook_move(from_pos, to_pos),
                // En passant is handled separately since it cannot attack a targeted square. Otherwise, pawns can only attack a real piece
                BoardSquare::Occupied(Piece::Pawn(_), _) => {
                    (pretend_occupied || matches!(self.board_state[to_pos.1][to_pos.0], BoardSquare::Occupied(_, _))) && pos_diff.0.abs() == 1 && ((pos_diff.1 == 1 && color == Color::Black) || (pos_diff.1 == -1 && color == Color::White))
                },
                _ => panic!("Invalid from square")
            }
        } else {
            panic!("From empty square");
        }
    }
    pub fn is_legal_start(&self, pos: BoardPosition) -> bool {
        if let BoardSquare::Occupied(_, color) = self.board_state[pos.1][pos.0] {
            color == self.turn
        } else { 
            false
        }
    }
    // Assumes legal move.
	pub fn do_move(&mut self, from_pos: BoardPosition, to_pos: BoardPosition) -> () {
        // reset this here so we can set it correctly if needed
        let old_en_passant_square = self.en_passant_square;
        self.en_passant_square = None;
        // Special cases where weird stuff happens: Castle, en passant. Also, keep track of state
        let pos_diff = ((to_pos.0 as i64 - from_pos.0 as i64), (to_pos.1 as i64 - from_pos.1 as i64));
        match self.board_state[from_pos.1][from_pos.0] {
            BoardSquare::Occupied(Piece::King, _) => {
                if pos_diff.0.abs() == 2 {
                    let rook_x = if pos_diff.0 < 0 {0} else {7};
                    assert!(self.board_state[from_pos.1][rook_x] == BoardSquare::Occupied(Piece::Rook, self.turn));
                    // move rook
                    self.board_state[to_pos.1][(to_pos.0 as i64 - (pos_diff.0 / 2)) as usize] = BoardSquare::Occupied(Piece::Rook, self.turn);
                    self.board_state[from_pos.1][rook_x] = BoardSquare::Empty;
                }
                // update castling vars
                if self.turn == Color::White {
                    self.white_can_castle_left = false;
                    self.white_can_castle_right = false;
                } else {
                    self.black_can_castle_left = false;
                    self.black_can_castle_right = false;
                }
                self.board_state[to_pos.1][to_pos.0] = self.board_state[from_pos.1][from_pos.0];
                self.board_state[from_pos.1][from_pos.0] = BoardSquare::Empty;
            },
            BoardSquare::Occupied(Piece::Pawn(_), _) => {
                // Make sure we set has_moved so we can't double move, plus set en passant, plus handle en passant
                // Double move should set en_passant_square to this
                if pos_diff.1.abs() == 2 {
                    self.en_passant_square = Some(to_pos);
                } else if pos_diff.0.abs() == 1 && self.board_state[to_pos.1][to_pos.0] == BoardSquare::Empty {
                    // en passanting
                    let pass_pos = old_en_passant_square.unwrap();
                    self.board_state[pass_pos.1][pass_pos.0] = BoardSquare::Empty;
                }
                self.board_state[to_pos.1][to_pos.0] = BoardSquare::Occupied(Piece::Pawn(true), self.turn);
                self.board_state[from_pos.1][from_pos.0] = BoardSquare::Empty;
            },
            BoardSquare::Occupied(Piece::Rook, _) => {
                // update castling vars
                let back_rank = if self.turn == Color::White {7} else {0};
                if from_pos.1 == back_rank {
                    if from_pos.0 == 0 {
                        if self.turn == Color::White {
                            self.white_can_castle_left = false;
                        } else {
                            self.black_can_castle_left = false;
                        }
                    } else if from_pos.0 == 7 {
                        if self.turn == Color::White {
                            self.white_can_castle_right = false;
                        } else {
                            self.black_can_castle_right = false;
                        }
                    }
                }
                self.board_state[to_pos.1][to_pos.0] = self.board_state[from_pos.1][from_pos.0];
                self.board_state[from_pos.1][from_pos.0] = BoardSquare::Empty;
            }, 
            _ => {
                self.board_state[to_pos.1][to_pos.0] = self.board_state[from_pos.1][from_pos.0];
                self.board_state[from_pos.1][from_pos.0] = BoardSquare::Empty;
            }  
        }
        // Update turn
        self.turn = !self.turn;
    }

    pub fn is_checkmate(&mut self, attacker: Color) -> bool {
        let (pos, _) = self.piece_iterator().filter(|&(_, piece)| piece == BoardSquare::Occupied(Piece::King, !attacker)).next().unwrap();
        if !self.is_square_attacked(pos, attacker) {
            return false;
        }
        let old_turn = self.turn.clone();
        self.turn = !attacker;
        let mut is_mate = true;
        'outer: for (defender_pos, defender_sq) in self.piece_iterator() {
            if let BoardSquare::Occupied(_, color) = defender_sq {
                if color == attacker {
                    continue;
                }
                for target_sq in (0..64).map(|x| BoardPosition(x / 8, x % 8)) {
                    if self.is_legal(defender_pos, target_sq) {
                        is_mate = false;
                        break 'outer;
                    }
                }
            }
        }
        self.turn = old_turn;
        is_mate
    }
}

fn c_to_sq(c: char) -> BoardSquare {
    let color = if c.is_ascii_lowercase() { Color::Black } else { Color::White };
    match c.to_ascii_lowercase() {
        '.' => BoardSquare::Empty,
        'p' => BoardSquare::Occupied(Piece::Pawn(false), color),
        'r' => BoardSquare::Occupied(Piece::Rook, color),
        'b' => BoardSquare::Occupied(Piece::Bishop, color),
        'n' => BoardSquare::Occupied(Piece::Knight, color),
        'q' => BoardSquare::Occupied(Piece::Queen, color),
        'k' => BoardSquare::Occupied(Piece::King, color),
        _ => panic!("Nonexistent c")
    }
}

const default_state: &str = "\
rnbqkbnr\
pppppppp\
........\
........\
........\
........\
PPPPPPPP\
RNBQKBNR";

const base_dir: &str = "/Users/gabriel.marks/my-project/images/";

const piece_imagepaths: [(char, &str); 12] = [
    ('p', "blackpawn.png"),
    ('r', "blackrook.png"),
    ('b', "blackbishop.png"),
    ('n', "blackknight.png"),
    ('q', "blackqueen.png"),
    ('k', "blackking.png"),
    ('P', "whitepawn.png"),
    ('R', "whiterook.png"),
    ('B', "whitebishop.png"),
    ('N', "whiteknight.png"),
    ('Q', "whitequeen.png"),
    ('K', "whiteking.png"),
];

pub struct Board {
	width: f64,
    height: f64,
	pub game_state: GameState,
	pieces: HashMap<BoardSquare, Box<dyn Drawable>>,
    pub highlight: Option<BoardPosition>
}

impl Board {
    pub fn new(width: f64, height: f64) -> Self {
        let game_state = GameState::new();
        let mut pieces : HashMap<BoardSquare, Box<dyn Drawable>> = HashMap::new();
        for (c, path) in piece_imagepaths {
            pieces.insert(c_to_sq(c), Box::new(PngDrawable::new(&format!("{}{}", base_dir, path), width / 8.0, height / 8.0).unwrap()));
        }
        pieces.insert(BoardSquare::Empty, Box::new(EmptyDrawable{}));
        Board {width, height, game_state, pieces, highlight: None}
    }
    
    pub fn setup_new_game(&mut self) -> () {
        self.setup_set_game(default_state, Color::White, true, true, true, true, None);
    }

    pub fn setup_set_game(&mut self, state: &str, turn: Color, bcl: bool, bcr: bool, wcl: bool, wcr: bool, eps: Option<BoardPosition>) -> () {
        assert_eq!(state.len(), 64);
        let mut state_it = state.chars();
        for i in 0..8 {
            for c in 0..8 {
                self.game_state.board_state[i][c] = c_to_sq(state_it.next().unwrap());
            }    
        }
        self.game_state.turn = turn;
        self.game_state.black_can_castle_left = bcl;
        self.game_state.black_can_castle_right = bcr;
        self.game_state.white_can_castle_left = wcl;
        self.game_state.white_can_castle_right = wcr;
        self.game_state.en_passant_square = eps;
    }

    pub fn is_checkmated(&mut self) -> bool {
        self.game_state.is_checkmate(!self.game_state.turn)
    }
}
const light_color: (f64, f64, f64) = (180.0 / 255.0, 175.0 / 255.0, 165.0 / 255.0);
const dark_color: (f64, f64, f64) = (145.0 / 255.0, 140.0 / 255.0, 125.0 / 255.0);
const highlight_color: (f64, f64, f64) = (180.0 / 255.0, 80.0 / 255.0, 80.0 / 255.0);
impl Drawable for Board {
    fn draw(&self, ctx: &mut Context, position: ScreenPosition) -> Result<(), Box<dyn Error>> {
        ctx.save()?;
        ctx.translate(position.0, position.0);
        ctx.set_source_rgb(light_color.0, light_color.1, light_color.2);
        ctx.rectangle(0.0, 0.0, self.width, self.height);
        ctx.fill()?;
        let mut first_light = true;
        let tile_w = self.width / 8.0;
        let tile_h = self.height / 8.0;
        ctx.set_source_rgb(dark_color.0, dark_color.1, dark_color.2);
        for i in 0..8 {
            let start = if first_light {1} else {0};
            for j in (start..8).step_by(2) {
                ctx.rectangle(j as f64 * tile_w, i as f64 * tile_h, tile_w.ceil(), tile_h.ceil());
            }
            first_light = !first_light;
        }
        ctx.fill()?;
        if let Some(hl_pos) = self.highlight {
            ctx.set_source_rgb(highlight_color.0, highlight_color.1, highlight_color.2);
            ctx.rectangle(hl_pos.0 as f64 * tile_w, hl_pos.1 as f64 * tile_h, tile_w.ceil(), tile_h.ceil());
            ctx.fill()?;
        }
        for i in 0..8 {
            for j in 0..8 {
                let bs = &self.game_state.board_state[i][j];
                let piece_draw;
                if let BoardSquare::Occupied(Piece::Pawn(true), color) = *bs {
                    piece_draw = self.pieces.get(&BoardSquare::Occupied(Piece::Pawn(false), color)).unwrap();
                } else {
                    piece_draw = self.pieces.get(bs).unwrap();
                }
                piece_draw.draw(ctx, ScreenPosition(j as f64 * tile_w, i as f64 * tile_h))?;
            }
        }
        ctx.restore()?;
        Ok(())
    }
}
	
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_checkmate() -> Result<(), &'static str> {
        let mut board = Board::new(0.0, 0.0);
        board.setup_set_game(&format!("R...k...R.......K{}", ".".repeat(64-17)), Color::Black, false, false, false, false, None);
        assert!(board.is_checkmated());
        board.setup_set_game(&format!("R...k...B.......K{}", ".".repeat(64-17)), Color::Black, false, false, false, false, None);
        assert!(!board.is_checkmated());
        board.setup_set_game(&format!("R...k...R.......K{}", ".".repeat(64-17)), Color::White, false, false, false, false, None);
        assert!(!board.is_checkmated());
        Ok(())
    }
}