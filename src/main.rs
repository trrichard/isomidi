extern crate sdl2;

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

const INCREMENT_ANGLE:f32 = 2.0*PI/6.0; // 60 degrees in radians

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

// handle the annoying Rect i32
macro_rules! rect(
    ($x:expr, $y:expr, $w:expr, $h:expr) => (
        Rect::new($x as i32, $y as i32, $w as u32, $h as u32)
    )
);

// Scale fonts to a reasonable size when they're too big (though they might look less smooth)
fn get_centered_rect(rect_width: u32, rect_height: u32, cons_width: u32, cons_height: u32) -> Rect {
    let wr = rect_width as f32 / cons_width as f32;
    let hr = rect_height as f32 / cons_height as f32;

    let (w, h) = if wr > 1f32 || hr > 1f32 {
        if wr > hr {
            println!("Scaling down! The text will look worse!");
            let h = (rect_height as f32 / wr) as i32;
            (cons_width as i32, h)
        } else {
            println!("Scaling down! The text will look worse!");
            let w = (rect_width as f32 / hr) as i32;
            (w, cons_height as i32)
        }
    } else {
        (rect_width as i32, rect_height as i32)
    };

    let cx = (800 as i32 - w) / 2;
    let cy = (600 as i32 - h) / 2;
    rect!(cx, cy, w, h)
}

/// Given the x and y locations of a click, return the address of the hexagon
/// The logic I'm doing in here is a little crazy. 
/// By rotating the cordinate grid three times I can figure out the "index" in terms of number of
/// hexagons from a the starting point
/// This effectivly tesalates the hexagons into 6 triangles, this algorithm gets the location of
/// the triangle clicked, then figures out which hexagon that triangle belongs in.
fn get_hex_address(xo:f32, yo:f32, hexagon:&HexagonDescription) -> (i16, i16) {
    let hex_height = hexagon.half_height as f32;
    
    let plane1 = yo / hex_height;
    
    let incangle = INCREMENT_ANGLE * -2.0; // -120 degrees
    let x = xo * incangle.cos() + yo * incangle.sin();
    let y = xo * incangle.sin() + yo * incangle.cos();
    let plane2 = -y / hex_height; // TODO why did I need to multiply this by two??
    
    let incangle = INCREMENT_ANGLE * -4.0; // -120 degrees
    let x = xo * incangle.cos() + yo * incangle.sin();
    let y = xo * incangle.sin() + yo * incangle.cos();
    let plane3 = y / hex_height ;
   
    let mut cord1 = plane1.floor() as i16;
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
    (x, y)
}


/// A Keyboard is responsible for labeling it's keys and knowing what to do when a given key is
/// pressed.
/// Keyboards must map from a given hexagon address to an action
struct JammerKeyboard<'a> {
    font : Font<'a>,
    colors: &'a ColorProfile,
}

impl<'a> Keyboard for JammerKeyboard<'a> {
    fn get_key_label(&self, addr: HexAddr, renderer: &mut Renderer) -> Result<Texture, sdl2::render::TextureValueError> {
        let note = self.get_note(addr);
        let surface = self.font.render(note).blended(self.colors.line_color).unwrap();
        renderer.create_texture_from_surface(&surface)
    }
    fn key_pressed(&self, addr: HexAddr) {
        // noop
    }
}

impl<'a> JammerKeyboard<'a> {
    fn get_note(&self, addr: HexAddr) -> &str {
        let bottom_row = [ "Bb", "C", "D", "E", "F#", "G#" ];
        let top_row = [ "F", "G", "A", "B", "C#", "D#" ];
        let x = (addr.x % 6) as usize;
        if addr.y % 2 == 0 {
            bottom_row[x]
        }else{
            top_row[x]
        }
        
    }
}
#[derive(Debug)]
struct HexAddr {
    x : i16,
    y : i16,
}

trait Keyboard {
    fn get_key_label(&self, HexAddr, renderer: &mut Renderer) -> Result<Texture, sdl2::render::TextureValueError>; // todo decide on return type.
    fn key_pressed(&self, HexAddr);
}


fn draw_keyboard(renderer:&mut Renderer, font:&Font, colors: &ColorProfile, hexagon: &HexagonDescription, keyboard: Option<&Keyboard>) -> Result<(),String> {
    // TODO math for the number of cols and rows based on window size.
    let rows = 20;
    let cols = 10;
    for row in 0..rows {
        for col in 0..cols {
            let is_even = row % 2 == 0;
            let (mut x_offset, mut y_offset) = match is_even {
                true => ((hexagon.width + hexagon.radius) * col, row * hexagon.half_height),
                false => ((hexagon.width + hexagon.radius) * col + hexagon.radius + hexagon.radius/2, row * hexagon.half_height),
            };
            x_offset -= hexagon.width/2;
            let (xs, ys) = translate_hexagon(hexagon.x_vec, hexagon.y_vec, x_offset, y_offset);
            match is_even {
                true => try!(renderer.filled_polygon(&xs, &ys, colors.d)),
                false => try!(renderer.filled_polygon(&xs, &ys, colors.e)),
            };
            //println!("{}x{} {:?} {:?}", row, col, xs.to_vec(), ys);
            try!(renderer.polygon(&xs, &ys, colors.line_color));

            // TODO cache textures for the hex labels
            // if we don't have a keyboard then just print the row and column numbers
            let mut texture = match keyboard {
                Some(keyboard) => keyboard.get_key_label(HexAddr{x:col, y:row}, renderer),
                None => {
                    // TODO handle this font error
                    let surface = font.render(format!("{},{}", col, row).as_str()).blended(colors.line_color).unwrap();
                    renderer.create_texture_from_surface(&surface)
                }
            };
            // TODO handle this texture error
            let mut texture = texture.unwrap();

            let TextureQuery { width, height, .. } = texture.query();
            let target = Rect::new(x_offset as i32, y_offset as i32, width, height);
            try!(renderer.copy(&mut texture, None, Some(target)));
        }
    }
    Ok(())
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
    let buffer_hack = 0;

    let font_path = Path::new("/home/trichard/Documents/Software/Blue/personal/isomidi/FantasqueSansMono-Regular.ttf");
    let screen_height = 1200;
    let screen_width = 1200;


    /////////////////////////
    ///// Derived Constants
    /////////////////////////


    let (hexagon_x, hexagon_y) = get_hexagon(0,0,radius);
    let half_height = ((INCREMENT_ANGLE).sin() * radius as f32).round() as i16;
    let hexagon = HexagonDescription {
        width : (radius * 2 + buffer_hack) as i16,
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
    
    let mut font = ttf_context.load_font(font_path, 20).unwrap();
    let mut keyboard_font = ttf_context.load_font(font_path, 20).unwrap();
    //keyboard_font.set_style(sdl2::ttf::STYLE_BOLD);

    // Draw a black screen
    renderer.set_draw_color(Color::RGB(0, 0, 0));
    renderer.clear();
    renderer.present();
    
    let mut event_pump = sdl_context.event_pump().unwrap();

    /////////////////////////
    //// Load the keyboard
    /////////////////////////
    let keyboard = JammerKeyboard { font: keyboard_font, colors: &colors };


    /////////////////////////
    //// Main loop
    /////////////////////////
    let mut frame_count = 0;
    let mut last_time = Instant::now();
    'running: loop {
        // TODO sleep till next event?
        let sleep_time = Duration::from_millis(10);
        thread::sleep(sleep_time);

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    println!("Exiting");
                    break 'running
                },
                Event::MouseButtonDown {x, y, ..} => {
                    get_hex_address(x as f32, y as f32, &hexagon);
                    println!("Mouse Down {} {}", x, y);
                },
                Event::FingerDown {x, y, ..} => {
                    get_hex_address(x as f32, y as f32, &hexagon);
                    println!("Finger Down {} {}", x, y);
                },
                _ => {}
            }
        }

        renderer.set_draw_color(colors.line_color);
        renderer.clear();
        draw_keyboard(&mut renderer, &font, &colors, &hexagon, Some(&keyboard));

        renderer.present();

        // TODO render a stats section in app
        frame_count += 1;
        if last_time.elapsed() > Duration::from_secs(1) {
            //println!("fps {}", frame_count);
            frame_count = 0;
            last_time = Instant::now();
        }
    }
}
