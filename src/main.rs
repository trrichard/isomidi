extern crate sdl2;
extern crate midir;
mod keyboard;

use sdl2::rect::Rect;
use sdl2::render::TextureQuery;
use std::{thread};
use std::time::{Instant,Duration};
use sdl2::pixels::Color;
use sdl2::ttf::Font;
use sdl2::event::Event;
use sdl2::event::WindowEvent;
use sdl2::keyboard::Keycode;
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::render::Renderer;
use std::f32;
use std::f32::consts::PI;
use std::collections::HashMap;
use keyboard::Keyboard;
use keyboard::HexAddr;
use keyboard::HexKey;
use keyboard::HarmonicKeyboard;
use keyboard::JammerKeyboard;
use midir::{MidiOutput, MidiOutputConnection};
use std::error::Error;
use sdl2::rwops::RWops;

const INCREMENT_ANGLE:f32 = 2.0*PI/6.0; // 60 degrees in radians
const MOUSE_OID:i64 = -1;
const NOTE_ON_MSG: u8 = 0x90;
const NOTE_OFF_MSG: u8 = 0x80;

/* TODO
 * Octave Shifing
 * Keyboard Rotation 
 * Multi Key Highlighting
 * Readme for github
 * Factor out midi
 * Better error handling/remove .unwraps
 * Add Guitar Layout
 * Consider changing draw_keyboard so that it draws only the key changes and not the whole board
 *      every time.
 * Correct velocity controls?
 */


fn get_hexagon(x:i16, y:i16, radius:i16) -> ([i16;6], [i16;6]) {
    // TODO this function needs to be broken up into a calculate and translate section, we don't
    // need to redo the sin math every time.
    let r:f32 = radius as f32;
    let mut angle:f32 = INCREMENT_ANGLE/2.0;

    let mut xs: [i16;6] = [0; 6];
    let mut ys: [i16;6] = [0; 6];
    for i in 0..6 {
        let xo = angle.sin()*r;
        let yo = angle.cos()*r;
        angle += INCREMENT_ANGLE;
        xs[i] = x + xo.round() as i16;
        ys[i] = y + yo.round() as i16;
    }
    return (xs, ys)
}

fn translate_hexagon(xlist:[i16;6], ylist:[i16;6], x:i16, y:i16) -> ([i16;6], [i16;6]) {
    let mut xs: [i16;6] = [0; 6];
    let mut ys: [i16;6] = [0; 6];
    for i in 0..6 {
        xs[i] = xlist[i] + x;
        ys[i] = ylist[i] + y;
    }
    return (xs, ys)
}


/// Given the x and y locations of a click, return the address of the hexagon
/// The logic I'm doing in here is a little crazy. 
/// By rotating the cordinate grid three times I can figure out the "index" in terms of number of
/// hexagons from a the starting point
/// This effectivly tesalates the hexagons into 6 triangles, this algorithm gets the location of
/// the triangle clicked, then figures out which hexagon that triangle belongs in.
fn get_hex_address(xo:f32, yo:f32, hexagon:&HexagonDescription) -> HexAddr {
    let hex_height = hexagon.half_height as f32;
    
    let plane1 = yo / hex_height;
    
    let incangle = INCREMENT_ANGLE * -2.0; // -120 degrees
    //let x = xo * incangle.cos() + yo * incangle.sin();
    let y = xo * incangle.sin() + yo * incangle.cos();
    let plane2 = -y / hex_height; // TODO why did I need to multiply this by two??
    
    let incangle = INCREMENT_ANGLE * -4.0; // -120 degrees
    //let x = xo * incangle.cos() + yo * incangle.sin();
    let y = xo * incangle.sin() + yo * incangle.cos();
    let plane3 = y / hex_height ;
   
    let cord1 = plane1.floor() as i16;
    let mut cord2 = plane2.floor() as i16;
    let mut cord3 = plane3.floor() as i16;

    // left justify the coordinate system for my own sanity while doing this modulo math
    cord2 -= cord1/2;
    cord3 += cord1/2 + 1;
    
    let mut y = cord1;
    let mut x = cord2/3;

    //println!("a: {} b:{} c:{}", cord1, cord2, cord3);
    if cord1 % 2 == 0 {
        // white down
        if cord2 % 3 == 0 {
            //println!("white");
            y+=1;
        } else if cord3 % 3 == 1 && cord2 % 3 == 1{
            //println!("white");
            y +=1;
        } else {
            //println!("purple");
            x+=1;
        }
    } else {
        // white up
        if cord2 % 3 == 1 {
            //println!("white");
        } else if cord3 % 3 == 0 && cord2 % 3 == 0 {
            //println!("white");
        } else {
            //println!("purple");
            y +=1;
            if cord2 %3 != 0 {
                x +=1;
            }
        }
    }
    //println!("x:{}, y:{}", x, y);
    HexAddr{x:x, y:y}
}

fn note_to_color(note: u8, config: &Config) -> Color {
     // 0 for root, 1 for in key, 2 for sharp or flat
     let colors = &config.colors;
     let key = config.root_note;

     //C    C#  D   D#  E   F   F#  G   G#  A   A#  B
     let major_color_mask = [ 
         colors.root, // c
         colors.out_of_key, // c#
         colors.in_key_and_penta, // d
         colors.out_of_key, // d#
         colors.in_key_and_penta, // e
         colors.in_key, // f
         colors.out_of_key, // f#
         colors.in_key_and_penta, // g
         colors.out_of_key, // g#
         colors.in_key_and_penta, // a
         colors.out_of_key, // a#
         colors.in_key  // b
     ];
     // computed c relative
     let minor_color_mask = [ 
         colors.in_key_and_penta, // c
         colors.out_of_key, // c#
         colors.in_key_and_penta, // d
         colors.out_of_key, // d#
         colors.in_key_and_penta, // e
         colors.in_key, // f
         colors.out_of_key, // f#
         colors.in_key_and_penta, // g
         colors.out_of_key, // g#
         colors.root, // a
         colors.out_of_key, // a#
         colors.in_key  // b
     ];
     let index = (note + key ) % 12;
     if config.is_major {
         major_color_mask[index as usize]
     } else {
         minor_color_mask[index as usize]
     }
}

struct Config {
    colors: ColorProfile,
    root_note: u8, // 0 for c
    is_major: bool, 
    hexagon: HexagonDescription,
    width: u32,
    height: u32,
    rows: i16,
    cols: i16,
    velocity: u8,
}

struct ColorProfile {
    line_color: Color,
    root: Color,
    out_of_key: Color,
    in_key_and_penta: Color,
    in_key: Color,
    white: Color,
}

#[derive(Debug)]
struct HexagonDescription {
    width:i16,
    height:i16,
    half_height:i16,
    radius:i16,
    x_vec:[i16;6],
    y_vec:[i16;6],
}


fn draw_keyboard(
        renderer:&mut Renderer, 
        font: &Font,
        config: &Config,
        keyboard: &Keyboard,
        pressed_keys: Vec<HexAddr>) -> Result<(),String> {
    let rows = config.rows;
    let cols = config.cols;

    for row in 0..rows {
        for col in 0..cols {
            let addr = HexAddr{x:col, y:row};

            let is_even = row % 2 == 0;
            let (mut x_offset, y_offset) = match is_even {
                true => ((config.hexagon.width + config.hexagon.radius) * col, row * config.hexagon.half_height),
                false => ((config.hexagon.width + config.hexagon.radius) * col + config.hexagon.radius + config.hexagon.radius/2, row * config.hexagon.half_height),
            };
            x_offset -= config.hexagon.width/2;

            let (xs, ys) = translate_hexagon(config.hexagon.x_vec, config.hexagon.y_vec, x_offset, y_offset);
            let key_info = keyboard.get_key_info(addr);
            let (color, label) = if let Some(key_info) = key_info {
                (note_to_color(key_info.note, config), key_info.label)
            } else {
                (config.colors.line_color, " ".to_string())
            };

            let polygon_color = match pressed_keys.contains(&addr) {
                true => config.colors.white,
                false => color,
            };

            try!(renderer.filled_polygon(&xs, &ys, polygon_color));
            try!(renderer.polygon(&xs, &ys, config.colors.line_color));

            // TODO cache textures for the hex labels
            // if we don't have a keyboard then just print the row and column numbers
            let surface = font.render(label.as_str()).blended(config.colors.line_color).unwrap();
            let mut texture = renderer.create_texture_from_surface(&surface).unwrap();

            let TextureQuery { width, height, .. } = texture.query();
            let label_x = (x_offset as i32 - width as i32/2) as i32;
            let label_y = (y_offset as i32 - height as i32/2) as i32;
            let target = Rect::new(label_x, label_y, width, height);
            try!(renderer.copy(&mut texture, None, Some(target)));
        }
    }
    Ok(())
}

struct KeyboardState<'a> {
    active_presses_map : HashMap<i64, HexAddr>,
    config: &'a Config,
    connection_out: &'a mut MidiOutputConnection,
}

impl<'a> KeyboardState<'a> {
    fn start_note(&mut self, addr: HexAddr, keyboard: &mut Keyboard) {
        let key = keyboard.get_key_info(addr);
        if let Some(x) = key {
            let res = self.connection_out.send(&[NOTE_ON_MSG, x.note, self.config.velocity]);
            if let Err(err) = res {
                println!("Error Sending Midi Note {}", err);
            };
        };
    }
    fn end_note(&mut self, addr: HexAddr, keyboard: &mut Keyboard) {
        let key = keyboard.get_key_info(addr);
        if let Some(x) = key {
            let res = self.connection_out.send(&[NOTE_OFF_MSG, x.note, self.config.velocity]);
            if let Err(err) = res {
                println!("Error Sending Midi Note {}", err);
            };
        };
    }
    fn on_press(&mut self, oid: i64, x:f32, y:f32, keyboard: &mut Keyboard) {
        let addr = get_hex_address(x, y, &self.config.hexagon);
        self.active_presses_map.insert(oid, addr);
        self.start_note(addr, keyboard);
    }
    fn on_release(&mut self, oid: i64, keyboard: &mut Keyboard) {
        match self.active_presses_map.remove(&oid) {
            Some(addr) => self.end_note(addr, keyboard),
            None => (),
        }
    }
    fn on_move(&mut self, oid: i64, x:f32, y:f32, keyboard: &mut Keyboard) {
        let addr = get_hex_address(x, y, &self.config.hexagon);
        match self.active_presses_map.get(&oid) {
            None => self.start_note(addr, keyboard),
            Some(&old_addr) => {
                if addr != old_addr {
                    self.start_note(addr, keyboard);
                    self.end_note(old_addr, keyboard);
                }
            }
        };
        self.active_presses_map.insert(oid, addr);
    }
    fn get_pressed(&self) -> Vec<HexAddr> {
        // TODO this iteration is SLOW and this function is called once per hexagon
        // TODO make this function FAST!
        let mut vec = Vec::new();
        for (_, &value) in &self.active_presses_map {
            vec.push(value);
        }
        vec
    }
}

fn get_midi_connection() -> Result<MidiOutputConnection,Box<Error>> {
    // TODO improve midi selection criteria, maybe pick off of command line.
    let midi_out = try!(MidiOutput::new("Isomidi"));
    let out_port: u32 = match midi_out.port_count() {
        0 => return Err("no output port found".into()),
        _ => {
            println!("Choosing the last available output port: {}", midi_out.port_name(0).unwrap());
            midi_out.port_count() -1
        }
    };
    println!("\nOpening connection");
    Ok(try!(midi_out.connect(out_port, "isomidi").map_err(|e| e.kind())))
}

fn main() {
    /////////////////////////
    ///// CONSTANTS
    /////////////////////////
    
    // https://coolors.co/f4d06f-ff8811-9dd9d2-fff8f0-392f5a

    let radius = 75;

    let screen_height = 1200;
    let screen_width = 1800;
    let ttf_font_bytes = include_bytes!("FantasqueSansMono-Regular.ttf");
    
    let mut connection_out = get_midi_connection().unwrap();


    /////////////////////////
    //// SDL Setup
    /////////////////////////
    let sdl_context = sdl2::init().unwrap(); 
    let video_subsystem = sdl_context.video().unwrap();
    video_subsystem.gl_attr().set_multisample_samples(8);
    let ttf_context = sdl2::ttf::init().unwrap();

    let window = video_subsystem.window("Isomidi", screen_width, screen_height)
        .position_centered()
        .opengl()
        .resizable()
        .build()
        .unwrap();
    
    let mut renderer = window.renderer().build().unwrap();
    
    let font_rwop = RWops::from_bytes(ttf_font_bytes).unwrap();
    let keyboard_font = ttf_context.load_font_from_rwops(font_rwop, 20).unwrap();
    // be bold
    // keyboard_font.set_style(sdl2::ttf::STYLE_BOLD);

    // Draw a black screen
    renderer.set_draw_color(Color::RGB(0, 0, 0));
    renderer.clear();
    renderer.present();
    
    let mut event_pump = sdl_context.event_pump().unwrap();

    /////////////////////////
    //// Load the keyboard
    /////////////////////////
    //let mut keyboard = JammerKeyboard {};
    'config: loop {
        /////////////////////////
        ///// Derived Constants
        /////////////////////////
        let colors = ColorProfile {
            line_color: Color::RGB(0, 0, 0),
            root : Color::RGB(0xf4,0xD0,0x6F), // root
            out_of_key : Color::RGB(0xff,0x88,0x11), // sharp/flat
            in_key_and_penta : Color::RGB(0x9D,0x9D,0xD2), // in key & pentatonic
            in_key : Color::RGB(0xba, 0xba, 0xdf), // in key
            white : Color::RGB(0x39,0x2F,0x5A),
        };

        let (hexagon_x, hexagon_y) = get_hexagon(0,0,radius);
        let half_height = ((INCREMENT_ANGLE).sin() * radius as f32).round() as i16;
        let hexagon = HexagonDescription {
            width : (radius * 2 ) as i16,
            half_height: half_height,
            height :  half_height * 2,
            radius: radius,
            x_vec: hexagon_x,
            y_vec: hexagon_y,
        };
        println!("hexagon: {:?}", hexagon);

        let size = {
            let mut window = renderer.window_mut().unwrap();
            window.size()
        };
        let rows = (size.1 as i16/hexagon.height as i16) * 2 + 3;
        let cols = (size.0 as i16/(hexagon.width + hexagon.radius)) as i16 + 2;

        let mut config = Config {
            colors: colors,
            root_note: 0, // C
            is_major: true,
            hexagon: hexagon,
            width: size.0,
            height: size.1,
            rows: rows,
            cols: cols,
            velocity: 70,
        };
        let mut keyboard = HarmonicKeyboard {};
        
        let mut keyboard_state = KeyboardState { 
            config: &config, 
            active_presses_map: HashMap::new(),
            connection_out: &mut connection_out,
        };

        /////////////////////////
        //// Main loop
        /////////////////////////
        let mut frame_count = 0;
        let mut last_time = Instant::now();
        let mut first_run = true;
        'render: loop {
            // TODO sleep till next event?
            let sleep_time = Duration::from_millis(10);
            thread::sleep(sleep_time);

            // TODO: How are we going to do multi finger tracking and mouse tracking?
            // list of active fingerids / mouse id plus the current hex addr.
            // on hex addr change fire on_key_press
            let mut trigger_draw = false;
            if first_run { trigger_draw = true; first_run = false }
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        println!("Exiting");
                        break 'config
                    },
                    Event::MouseButtonDown {x, y, ..} => {
                        keyboard_state.on_press(MOUSE_OID, x as f32, y as f32, &mut keyboard);
                        trigger_draw = true;
                    },
                    Event::MouseButtonUp {..} => {
                        keyboard_state.on_release(MOUSE_OID, &mut keyboard);
                        trigger_draw = true;
                    },
                    Event::MouseMotion {x, y, mousestate, ..} => {
                        // track only if left mouse button is down
                        if mousestate.left() {
                            keyboard_state.on_move(MOUSE_OID, x as f32, y as f32, &mut keyboard);
                            trigger_draw = true;
                        }
                    },
                    Event::FingerDown {x, y, finger_id, ..} => {
                        keyboard_state.on_press(finger_id, x as f32, y as f32, &mut keyboard);
                        trigger_draw = true;
                    },
                    Event::FingerMotion {x, y, finger_id, ..} => {
                        keyboard_state.on_move(finger_id, x as f32, y as f32, &mut keyboard);
                        trigger_draw = true;
                    },
                    Event::FingerUp {finger_id, ..} => {
                        keyboard_state.on_release(finger_id, &mut keyboard);
                        trigger_draw = true;
                    },
                    Event::Window {win_event, ..} => {
                        match win_event {
                            WindowEvent::SizeChanged (width, height) => {
                                // breaks out of the render loop and reconfigures the application
                                break 'render
                            }
                            _ => {}
                        }
                    },
                    _ => {}
                }
            }
            if trigger_draw {
                renderer.set_draw_color(config.colors.line_color);
                renderer.clear();
                draw_keyboard(&mut renderer, &keyboard_font, &config, &keyboard, keyboard_state.get_pressed()).unwrap();
            }

            renderer.present();

            // TODO render a stats section in app
            frame_count += 1;
            if last_time.elapsed() > Duration::from_secs(1) {
                println!("fps {}", frame_count);
                frame_count = 0;
                last_time = Instant::now();
            }
        }
    }
}
