#[path = "chess.rs"] pub mod chess;
use std::str::from_utf8;
use minifb::{MouseMode, MouseButton, Window, WindowOptions};
extern crate cairo;
use cairo::{ ImageSurface, Format, Context };
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::error::Error;

pub trait Networkable where Self: Sized {
    fn serialize(&self, stream: &mut TcpStream) -> Result<(), Box<dyn Error>>;
    fn deserialize(stream: &mut TcpStream) -> Result<Self, Box<dyn Error>>;
}

impl Networkable for chess::BoardPosition {
    fn serialize(&self, stream: &mut TcpStream) -> Result<(), Box<dyn Error>> {
        assert!(self.0 < 256 && self.1 < 256);
        stream.write_all(&[self.0 as u8, self.1 as u8])?;
        Ok(())
    }

    fn deserialize(stream: &mut TcpStream) -> Result<Self, Box<dyn Error>> {
        let mut buf = [0; 2];
        stream.read_exact(&mut buf)?;
        Ok(chess::BoardPosition(buf[0] as usize, buf[1] as usize))
    }
}

impl Networkable for () {
    fn serialize(&self, _: &mut TcpStream) -> Result<(), Box<dyn Error>> {Ok(())}
    fn deserialize(_: &mut TcpStream) -> Result<Self, Box<dyn Error>> {Ok(())}
}

impl<T: Networkable> Networkable for (T, T) {
    fn serialize(&self, stream: &mut TcpStream) -> Result<(), Box<dyn Error>> {
        self.0.serialize(stream)?;
        self.1.serialize(stream)
    }
    fn deserialize(stream: &mut TcpStream) -> Result<Self, Box<dyn Error>> {
        let e1 = T::deserialize(stream)?;
        let e2 = T::deserialize(stream)?;
        Ok((e1, e2))
    }
}

impl Networkable for usize {
    fn serialize(&self, stream: &mut TcpStream) -> Result<(), Box<dyn Error>> {
        stream.write_all(&self.to_be_bytes())?;
        Ok(())
    }
    fn deserialize(stream: &mut TcpStream) -> Result<Self, Box<dyn Error>> {
        let mut bytes = [0; 8];
        stream.read_exact(&mut bytes)?;
        Ok(usize::from_be_bytes(bytes))
    }
}

/*impl Networkable for String {
    fn serialize(&self, ster) -> Vec<u8> {
        let mut vec = self.len().serialize();
        vec.extend_from_slice(self.as_bytes());
        vec
    }
    fn deserialize(bytes: &[u8]) -> (Self, &[u8]) {
        let (len, bytes) = usize::deserialize(bytes);
        (from_utf8(&bytes[..len]).unwrap().to_string(), &bytes[len..])
    }
}*/

#[derive(Debug)]
enum Packet {
    Move(chess::BoardPosition, chess::BoardPosition),
    AckMove,
    RejMove
}

#[derive(Debug, Clone)]
struct BadPacketError {}
impl std::fmt::Display for BadPacketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unrecognized packet received")
    }
}
impl std::error::Error for BadPacketError {}

impl Networkable for Packet {
    fn serialize(&self, stream: &mut TcpStream) -> Result<(), Box<dyn Error>> {
        println!("Serializing packet: {:?}", self);
        match self {
            Packet::Move(bp1, bp2) => {
                stream.write_all(&mut [0])?;
                bp1.serialize(stream)?;
                bp2.serialize(stream)?;
            },
            Packet::AckMove => {
                stream.write_all(&mut [1])?;
            },
            Packet::RejMove => {
                stream.write_all(&mut [2])?;
            }
        }
        Ok(())
    }
    fn deserialize(stream: &mut TcpStream) -> Result<Self, Box<dyn Error>> {
        println!("Attempting to deserialize packet");
        let mut switch_byte = [0; 1];
        stream.read_exact(&mut switch_byte)?;
        match switch_byte[0] {
            0 => {
                let bp1 = chess::BoardPosition::deserialize(stream)?;
                let bp2 = chess::BoardPosition::deserialize(stream)?;
                println!("Deserialized packet: {:?}", Packet::Move(bp1, bp2));
                Ok(Packet::Move(bp1, bp2))
            },
            1 => {
                println!("Deserialized packet: {:?}", Packet::AckMove);
                Ok(Packet::AckMove)
            },
            2 => {
                println!("Deserialized packet: {:?}", Packet::RejMove);
                Ok(Packet::RejMove)
            },
            _ => {
                println!("Deserialized error packet!!!");
                Err(Box::new(BadPacketError {}))
            }
        }
    }
}

type StateResult = Result<Box<dyn ChessState>, Box<dyn Error>>;

pub trait ChessState {
    fn next(&mut self) -> StateResult;
}

pub struct GlobalState {
    board: chess::Board,
    window: Window, 
    surface: ImageSurface,
    stream: TcpStream,
    width: usize,
    height: usize
}

use std::time;
const ONE_MILLI : time::Duration = time::Duration::from_millis(20);

impl GlobalState {
    fn click_to_board(&self, pos: chess::ScreenPosition) -> Result<chess::BoardPosition, ()> {
        let bp = chess::BoardPosition((pos.0 * 8.0 / (self.width as f64)) as usize, (pos.1 * 8.0 / (self.height as f64)) as usize);
        if bp.0 < 8 && bp.1 < 8 {
            Ok(bp)
        } else {
            Err(())
        }
    }
    
    fn get_next_legal_click(&mut self) -> chess::BoardPosition {
        self.window.limit_update_rate(Some(ONE_MILLI));
        loop {
            while !self.window.get_mouse_down(MouseButton::Left) {self.window.update();/*draw(window, surface, board);*/}
            println!("Got down event");
            while self.window.get_mouse_down(MouseButton::Left) {self.window.update();/*draw(window, surface, board);*/}
            println!("Got up event");
            let opt_pos = self.window.get_mouse_pos(MouseMode::Clamp);
            if let Some(pos) = opt_pos {
                match self.click_to_board(chess::ScreenPosition(pos.0 as f64, pos.1 as f64)) {
                    Ok(bp) => return bp,
                    Err(()) => continue
                }
            }
        }
    }
    
    fn draw(&mut self) -> Result<(), Box<dyn Error>> {
        let size = self.window.get_size();
        {
            let mut context = Context::new(&self.surface)?;
            
            context.set_source_rgb(1.0, 1.0, 1.0);
            context.paint()?;
        
            self.board.draw(&mut context, chess::ScreenPosition(0.0, 0.0))?;
        }
        let data = self.surface.data();
        let uwdata = data.unwrap();
        let data = unsafe {
            let (_, d, _) = uwdata.align_to::<u32>();
            d
        };
        self.window.update_with_buffer(&data, size.0, size.1)?;
        Ok(())
    }
}

use std::rc::Rc;
use std::cell::RefCell;

struct MyMove {
    global_state: Rc<RefCell<GlobalState>>,
}
struct OtherMove {
    global_state: Rc<RefCell<GlobalState>>,
}
struct AwaitAck {
    global_state: Rc<RefCell<GlobalState>>,
    next_move: (chess::BoardPosition, chess::BoardPosition)
}

use chess::Drawable;

impl ChessState for MyMove {
    fn next(&mut self) -> StateResult {
        let mut gs = self.global_state.borrow_mut();
        gs.draw()?;
        let mut bp1;
        let mut bp2;
        loop {
            // Get a click on board and assure we are clicking the correct color
            bp1 = gs.get_next_legal_click();
            if !gs.board.game_state.is_legal_start(bp1) {
                continue;
            }
            // Highlight clicked square and draw in w/ highlight
            gs.board.highlight = Some(bp1);
            gs.draw()?;
            // Get next click and delete highlight + draw (whether or not move is allowed)
            bp2 = gs.get_next_legal_click();
            gs.board.highlight = None;
            gs.draw()?;
            // If move is legal, break, otherwise keep looping
            if gs.board.game_state.is_legal(bp1, bp2) {
                break;
            }
        }
        // We have a legal move bp1, bp2 -- Transition to the AwaitAck state
        Ok(Box::new(AwaitAck{global_state: self.global_state.clone(), next_move: (bp1, bp2)}))
    }
}

impl ChessState for AwaitAck {
    fn next(&mut self) -> StateResult {
        let mut gs = self.global_state.borrow_mut();        
        Packet::Move(self.next_move.0, self.next_move.1).serialize(&mut gs.stream)?;
        loop {
            gs.draw()?;
            let next_packet = Packet::deserialize(&mut gs.stream)?;
            match next_packet {
                Packet::AckMove => {
                    // Success case -- they acknowledged our move, so we can do the move and move into new state
                    gs.board.game_state.do_move(self.next_move.0, self.next_move.1);
                    gs.draw()?;
                    return Ok(Box::new(OtherMove{global_state: self.global_state.clone()}));
                },
                Packet::RejMove => {
                    // Failure case -- Move was rejected. :( lets just die because this shouldn't happen
                    panic!("Move was rejected!!!!");
                },
                _ => {}
            }
        }
    }
}

impl ChessState for OtherMove {
    fn next(&mut self) -> StateResult {
        let mut gs = self.global_state.borrow_mut();
        // Wait to receive other's move.
        loop {
            gs.draw()?;
            let next_packet = Packet::deserialize(&mut gs.stream)?;
            match next_packet {
                Packet::Move(bp1, bp2) => {
                    // Check legality of move.
                    if gs.board.game_state.is_legal(bp1, bp2) {
                        // Accept move, draw board, go to MyMove state
                        Packet::AckMove.serialize(&mut gs.stream)?;
                        gs.board.game_state.do_move(bp1, bp2);
                        gs.draw()?;
                        return Ok(Box::new(MyMove{global_state: self.global_state.clone()}))
                    } else {
                        // Reject move and keep waiting
                        Packet::RejMove.serialize(&mut gs.stream)?;
                    }
                },
                _ => {}
            }
        }
    }
}

pub fn run_server(board: chess::Board,
    window: Window, 
    surface: ImageSurface,
    port: usize,
    width: usize,
    height: usize) -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;

    let stream = listener.incoming().next().unwrap()?;

    let mut global_state = GlobalState {board, window, surface, stream, width, height};
    global_state.draw()?;
    
    let mut run_state : Box<dyn ChessState> = Box::new(MyMove{global_state: Rc::new(RefCell::new(global_state))});
    loop {
        run_state = run_state.next()?;
    }
    Ok(())
}

pub fn run_client(board: chess::Board,
    window: Window, 
    surface: ImageSurface,
    port: usize,
    width: usize,
    height: usize) -> Result<(), Box<dyn Error>> {
    let stream = TcpStream::connect(format!("127.0.0.1:{}", port))?;

    let mut global_state = GlobalState {board, window, surface, stream, width, height};
    global_state.draw()?;
    
    let mut run_state : Box<dyn ChessState> = Box::new(OtherMove{global_state: Rc::new(RefCell::new(global_state))});
    loop {
        run_state = run_state.next()?;
    }
    Ok(())
}