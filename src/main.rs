use my_project;
mod raqote_example;
mod net_chess;
//mod chess;
use std::env;
use minifb::{MouseMode, MouseButton, Window, WindowOptions};
extern crate cairo;
use cairo::{ ImageSurface, Format, Context };
use std::error::Error;

const WIDTH: usize = 400;
const HEIGHT: usize = 400;
//static global_state: Option<net_chess::GlobalState> = None;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Hello, world!");
    let args: Vec<String> = env::args().collect();
    let s_or_c = &args[1];
    let is_client = match s_or_c.as_ref() {
        "s" => false,
        "c" => true,
        _ => panic!("Invalid c or s string!")
    };
 
    let sport = &args[2];
    let port = sport.parse::<usize>()?;

    let window = Window::new("Raqote", WIDTH, HEIGHT, WindowOptions {
        ..WindowOptions::default()
    }).unwrap();
    let size = window.get_size();
    let surface = ImageSurface::create(Format::ARgb32, size.0 as i32, size.1 as i32)
        .expect("Couldn’t create surface");
    let mut board = net_chess::chess::Board::new(WIDTH as f64, HEIGHT as f64);
    board.setup_new_game();

    if is_client {
        net_chess::run_client(board, window, surface, port, WIDTH, HEIGHT)?;
    } else {
        net_chess::run_server(board, window, surface, port, WIDTH, HEIGHT)?;
    }
    Ok(())
//    // let gs = Rc::new(net_chess::GlobalState);
//     //raqote_example::main();

//     //my_project::run(1500)?;

//     Ok(())
}
