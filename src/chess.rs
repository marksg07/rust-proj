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

#[derive(Clone, Copy)]
pub struct ScreenPosition(
    pub f64,
    pub f64
);

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BoardPosition(
    pub u8,
    pub u8
);

pub trait Drawable {
    fn draw(&self, dt: &mut Context, position: ScreenPosition) -> Result<(), Box<dyn Error>> {Ok(())}
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
enum Color {
    White,
    Black
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
enum Piece {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
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

struct GameState {
	board_state: [[BoardSquare; 8]; 8],
	turn: Color,
    black_can_castle: bool,
    white_can_castle: bool,
    // If en passant is legal, the square of the pawn which can be captured via en passant.
    en_passant_square: Option<BoardPosition>,
}
impl GameState {
    fn new() -> Self {
        GameState {
            board_state: [[BoardSquare::Empty; 8]; 8],
            turn: Color::White,
            black_can_castle: true,
            white_can_castle: true,
            en_passant_square: None,
        }
    }
	fn is_legal(&self, from_pos: BoardPosition, to_pos: BoardPosition) -> bool {
        true
    }
    fn is_legal_start(&self, pos: BoardPosition) -> bool {
        true
    }
    // Assumes legal move.
	fn do_move(&mut self, from_pos: BoardPosition, to_pos: BoardPosition) -> () {
        self.board_state[to_pos.1 as usize][to_pos.0 as usize] = self.board_state[from_pos.1 as usize][from_pos.0 as usize];
        self.board_state[from_pos.1 as usize][from_pos.0 as usize] = BoardSquare::Empty;
    }
}

fn c_to_sq(c: char) -> BoardSquare {
    let color = if c.is_ascii_lowercase() { Color::Black } else { Color::White };
    match c.to_ascii_lowercase() {
        '.' => BoardSquare::Empty,
        'p' => BoardSquare::Occupied(Piece::Pawn, color),
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
	game_state: GameState,
	pieces: HashMap<BoardSquare, Box<dyn Drawable>>,
    highlight: Option<BoardPosition>
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
        println!("DS len {}", default_state.len());
        let mut default_state_it = default_state.chars();
        for i in 0..8 {
            for c in 0..8 {
                self.game_state.board_state[i][c] = c_to_sq(default_state_it.next().unwrap());
            }    
        }
        self.game_state.turn = Color::White;
        self.game_state.black_can_castle = true;
        self.game_state.white_can_castle = true;
        self.game_state.en_passant_square = Option::None;
    }

	pub fn try_move(&mut self, from_pos: BoardPosition, to_pos: BoardPosition) -> () {
		if self.game_state.is_legal(from_pos, to_pos) {
		    self.game_state.do_move(from_pos, to_pos);
        }
    }

    pub fn set_highlight(&mut self, opt_pos: Option<BoardPosition>) {
        self.highlight = opt_pos;
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
                self.pieces.get(&self.game_state.board_state[i][j]).unwrap().draw(ctx, ScreenPosition(j as f64 * tile_w, i as f64 * tile_h))?;
            }
        }
        ctx.restore()?;
        Ok(())
    }
}
		
/*
main() {
	board = board::new(size_x, size_y)
	loop {
		(click to move semantics)
		until legal:
			px, py = wait_for_user_click()
			bx, by = convert_to_boardpos(size_x, size_y, px, py)
		board.highlight(bx, by, true)
		Until legal:
			px2, py2 = wait_for_user_click()
			bx2, by2 = convert_to_boardpos(size_x, size_y, px, py)
		board.highlight(bx, by, false)
		board.move((bx, by), (bx2, by2))
	}
}
*/		

	
	

