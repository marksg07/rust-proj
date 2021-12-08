use minifb::{MouseMode, MouseButton, Window, WindowOptions};
extern crate cairo;
use cairo::{ ImageSurface, Format, Context };
use std::{thread, time};

const ONE_MILLI : time::Duration = time::Duration::from_millis(50);

use std::error::Error;
//use font_kit::loaders;
const WIDTH: usize = 400;
const HEIGHT: usize = 400;
#[path = "chess.rs"] mod chess;
use chess::Drawable;

fn click_to_board(pos: chess::ScreenPosition) -> Result<chess::BoardPosition, ()> {
    let bp = chess::BoardPosition((pos.0 * 8.0 / (WIDTH as f64)) as u8, (pos.1 * 8.0 / (HEIGHT as f64)) as u8);
    if bp.0 < 8 && bp.1 < 8 {
        Ok(bp)
    } else {
        Err(())
    }
}

fn get_next_legal_click(window: &mut Window, surface: &mut ImageSurface, board: &chess::Board) -> chess::BoardPosition {
    loop {
        while !window.get_mouse_down(MouseButton::Left) {draw(window, surface, board);}
        while window.get_mouse_down(MouseButton::Left) {draw(window, surface, board);}
        let opt_pos = window.get_mouse_pos(MouseMode::Clamp);
        if let Some(pos) = opt_pos {
            match click_to_board(chess::ScreenPosition(pos.0 as f64, pos.1 as f64)) {
                Ok(bp) => return bp,
                Err(()) => continue
            }
        }
    }
}

fn draw(window: &mut Window, surface: &mut ImageSurface, board: &chess::Board) -> Result<(), Box<dyn Error>> {
    let size = window.get_size();
    {
        let mut context = Context::new(surface)?;
        
        context.set_source_rgb(1.0, 1.0, 1.0);
        context.paint()?;
    
        board.draw(&mut context, chess::ScreenPosition(0.0, 0.0))?;
    }
    let data = surface.data();
    let uwdata = data.unwrap();
    let data = unsafe {
        let (_, d, _) = uwdata.align_to::<u32>();
        d
    };
    window.update_with_buffer(&data, size.0, size.1)?;
    Ok(())
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let mut window = Window::new("Raqote", WIDTH, HEIGHT, WindowOptions {
        ..WindowOptions::default()
    }).unwrap();
    let size = window.get_size();
    let mut surface = ImageSurface::create(Format::ARgb32, size.0 as i32, size.1 as i32)
        .expect("Couldn’t create surface");
    let mut board = chess::Board::new(WIDTH as f64, HEIGHT as f64);
    board.setup_new_game();
    draw(&mut window, &mut surface, &board)?;
    draw(&mut window, &mut surface, &board)?;
    loop {
        draw(&mut window, &mut surface, &board)?;
        draw(&mut window, &mut surface, &board)?;
        let bp1 = get_next_legal_click(&mut window, &mut surface, &board);
        board.set_highlight(Some(bp1));
        draw(&mut window, &mut surface, &board)?;
        let bp2 = get_next_legal_click(&mut window, &mut surface, &board);
        board.set_highlight(None);
        board.try_move(bp1, bp2);
        draw(&mut window, &mut surface, &board)?;
    }
    Ok(())
}