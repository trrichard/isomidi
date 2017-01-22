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
use sdl2::keyboard::Keycode;
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::render::Renderer;
use std::f32;
use std::f32::consts::PI;
use std::path::Path;
use sdl2::render::Texture;
use std::collections::HashMap;
use keyboard::Keyboard;
use keyboard::HexAddr;
use keyboard::HexKey;
use midir::{MidiOutput, Ignore, MidiOutputConnection};

const INCREMENT_ANGLE:f32 = 2.0*PI/6.0; // 60 degrees in radians
const MOUSE_OID:i64 = -1;

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


/// A Keyboard is responsible for labeling it's keys and knowing what to do when a given key is
/// pressed.
/// Keyboards must map from a given hexagon address to an action
struct JammerKeyboard<'a> {
    font : Font<'a>,
    colors: &'a ColorProfile,
    connection_out: MidiOutputConnection,
}

impl<'a> Keyboard for JammerKeyboard<'a> {
    fn on_key_press(&mut self, addr: HexAddr) {
        let (note, note_num) = {
            self.get_note(addr)
        };
        self.connection_out.send(&[144, note_num, 70]);
        println!("Note Down: {:?} -> {}", note, note_num);
    }
    fn on_key_release(&mut self, addr: HexAddr) {
        let (note, note_num) = {
            self.get_note(addr)
        };
        self.connection_out.send(&[144, note_num, 0]);
        println!("Note Up: {}", note);
    }
    fn get_key_info(&self, addr: HexAddr, renderer: &mut Renderer) -> Result<HexKey, &'static str> {
        let (note,_) = self.get_note(addr);
        // TODO handle these unwraps
        let surface = self.font.render(note.as_str()).blended(self.colors.line_color).unwrap();
        let texture = renderer.create_texture_from_surface(&surface).unwrap();

        let color = match note == "C" {
            true => self.colors.b,
            false => {
                match note.contains("#") || note.contains("b") {
                    true => self.colors.c,
                    false => self.colors.d,
                }
            }
        };
        Ok(HexKey {
            pressed_color: self.colors.f,
            color: color,
            label: texture, 
        })
    }
}

impl<'a> JammerKeyboard<'a> {
    fn get_note(&self, addr: HexAddr) -> (String,u8) {
        let bottom_row =     [ "Bb", "C", "D", "E", "F#", "G#" ];
        let bottom_row_num =     [ 0, 2, 4, 6, 8, 10 ];
        let top_row =     [ "F", "G", "A", "B", "C#", "Eb" ];
        let top_row_num =     [ 7, 9, 11, 12+1, 12+3, 12+5 ];
        let x = (addr.x % 6) as usize;
        let keyset = addr.x/6;
        //println!("keyset {:?}, {}", addr, keyset);
        // TODO remove the + 12 for this  and make some keys "invalid"
        let octave = 144 - (12 + addr.y/2 * 12 + 10 - keyset*12) as u8;
        if addr.y % 2 == 0 {
            (bottom_row[x].to_string(), octave + bottom_row_num[x]+12)
        }else{
            (top_row[x].to_string(), octave + top_row_num[x])
        }
    }
}
struct HarmonicKeyboard<'a> {
    font : Font<'a>,
    colors: &'a ColorProfile,
    connection_out: MidiOutputConnection,
}

impl<'a> Keyboard for HarmonicKeyboard<'a> {
    fn on_key_press(&mut self, addr: HexAddr) {
        let (note, note_num) = {
            self.get_note(addr)
        };
        self.connection_out.send(&[144, note_num, 70]);
        println!("Note Down: {:?} -> {}", note, note_num);
    }
    fn on_key_release(&mut self, addr: HexAddr) {
        let (note, note_num) = {
            self.get_note(addr)
        };
        self.connection_out.send(&[144, note_num, 0]);
        println!("Note Up: {}", note);
    }
    fn get_key_info(&self, addr: HexAddr, renderer: &mut Renderer) -> Result<HexKey, &'static str> {
        let (note,_) = self.get_note(addr);
        // TODO handle these unwraps
        let surface = self.font.render(note.as_str()).blended(self.colors.line_color).unwrap();
        let texture = renderer.create_texture_from_surface(&surface).unwrap();

        let color = match note == "C" {
            true => self.colors.b,
            false => {
                match note.contains("#") || note.contains("b") {
                    true => self.colors.c,
                    false => self.colors.d,
                }
            }
        };
        Ok(HexKey {
            pressed_color: self.colors.f,
            color: color,
            label: texture, 
        })
    }
}

impl<'a> HarmonicKeyboard<'a> {
    fn get_note(&self, addr: HexAddr) -> (String,u8) {
        let notes = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "G#", "A", "Bb", "B" ];
        let note_num = 120 + addr.x - 3 * addr.y - addr.y/2;
        if note_num < 0 {
            return (" ".to_string(), 0);
        }
        let note_label = note_num % 12;
        return (format!("{}:{},{}", notes[note_label as usize], addr.x, addr.y), note_num as u8)
    }
}

struct ColorProfile {
    line_color: Color,
    b: Color,
    c: Color,
    d: Color,
    e: Color,
    f: Color,
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
        font:&Font, 
        colors: &ColorProfile, 
        hexagon: &HexagonDescription, 
        keyboard: &Keyboard,
        pressed_keys: Vec<HexAddr>) -> Result<(),String> {
    // TODO math for the number of cols and rows based on window size.
    let rows = 20;
    let cols = 10;

    for row in 0..rows {
        for col in 0..cols {
            let addr = HexAddr{x:col, y:row};
            let key_info = try!(keyboard.get_key_info(addr, renderer));

            let is_even = row % 2 == 0;
            let (mut x_offset, y_offset) = match is_even {
                true => ((hexagon.width + hexagon.radius) * col, row * hexagon.half_height),
                false => ((hexagon.width + hexagon.radius) * col + hexagon.radius + hexagon.radius/2, row * hexagon.half_height),
            };
            x_offset -= hexagon.width/2;

            let (xs, ys) = translate_hexagon(hexagon.x_vec, hexagon.y_vec, x_offset, y_offset);
            let polygon_color = match pressed_keys.contains(&addr) {
                true => key_info.pressed_color,
                false => key_info.color,
            };

            try!(renderer.filled_polygon(&xs, &ys, polygon_color));
            //println!("{}x{} {:?} {:?}", row, col, xs.to_vec(), ys);
            try!(renderer.polygon(&xs, &ys, colors.line_color));

            // TODO cache textures for the hex labels
            // if we don't have a keyboard then just print the row and column numbers
            let mut texture = key_info.label;

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
    hexagon: &'a HexagonDescription,
}

impl<'a> KeyboardState<'a> {
   fn on_press(&mut self, oid: i64, x:f32, y:f32, keyboard: &mut Keyboard) {
        let addr = get_hex_address(x, y, self.hexagon);
        self.active_presses_map.insert(oid, addr);
        keyboard.on_key_press(addr);
    }
    fn on_release(&mut self, oid: i64, keyboard: &mut Keyboard) {
        match self.active_presses_map.remove(&oid) {
            Some(addr) => keyboard.on_key_release(addr),
            None => (),
        }
    }
    fn on_move(&mut self, oid: i64, x:f32, y:f32, keyboard: &mut Keyboard) {
        let addr = get_hex_address(x, y, self.hexagon);
        match self.active_presses_map.get(&oid) {
            None => keyboard.on_key_press(addr),
            Some(&old_addr) => {
                if addr != old_addr { 
                    keyboard.on_key_press(addr);
                    keyboard.on_key_release(old_addr);
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

fn main() {
    /////////////////////////
    ///// CONSTANTS
    /////////////////////////
    
    // https://coolors.co/f4d06f-ff8811-9dd9d2-fff8f0-392f5a
    let colors = ColorProfile {
        line_color: Color::RGB(0, 0, 0),
        b : Color::RGB(0xf4,0xD0,0x6F),
        c : Color::RGB(0xff,0x88,0x11),
        d : Color::RGB(0x9D,0x9D,0xD2),
        e : Color::RGB(0xFF,0xF8,0xF0),
        f : Color::RGB(0x39,0x2F,0x5A),
    };

    let radius = 75;

    let font_path = Path::new("/home/trichard/Documents/Software/Blue/personal/isomidi/FantasqueSansMono-Regular.ttf");
    let screen_height = 1200;
    let screen_width = 1800;


    /////////////////////////
    ///// Derived Constants
    /////////////////////////


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

    /////////////////////////
    //// SDL Setup
    /////////////////////////
    let sdl_context = sdl2::init().unwrap(); 
    let video_subsystem = sdl_context.video().unwrap();
    video_subsystem.gl_attr().set_multisample_samples(8);
    let ttf_context = sdl2::ttf::init().unwrap();

    let window = video_subsystem.window("rust-sdl2 demo: Video", screen_width, screen_height)
        .position_centered()
        .opengl()
        .build()
        .unwrap();
    
    let mut renderer = window.renderer().build().unwrap();
    
    let font = ttf_context.load_font(font_path, 20).unwrap();
    let keyboard_font = ttf_context.load_font(font_path, 20).unwrap();
    //keyboard_font.set_style(sdl2::ttf::STYLE_BOLD);

    // Draw a black screen
    renderer.set_draw_color(Color::RGB(0, 0, 0));
    renderer.clear();
    renderer.present();
    
    let mut event_pump = sdl_context.event_pump().unwrap();

    /////////////////////////
    //// Load the keyboard
    /////////////////////////
    let midi_out = MidiOutput::new("ISOMIDI").unwrap();
    let mut connection_out = midi_out.connect(0, "Renoise Midi Input").map_err(|e| e.kind()).unwrap();
    let mut keyboard = HarmonicKeyboard { 
        font: keyboard_font, 
        colors: &colors,
        connection_out : connection_out,
    };
    

    let mut keyboard_state = KeyboardState { 
        hexagon: &hexagon, 
        active_presses_map: HashMap::new() 
    };

    /////////////////////////
    //// Main loop
    /////////////////////////
    let mut frame_count = 0;
    let mut last_time = Instant::now();
    let mut first_run = true;
    'running: loop {
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
                    break 'running
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
                _ => {}
            }
        }
        if trigger_draw {
            renderer.set_draw_color(colors.line_color);
            renderer.clear();
            draw_keyboard(&mut renderer, &font, &colors, &hexagon, &keyboard, keyboard_state.get_pressed());
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
