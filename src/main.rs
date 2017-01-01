extern crate sdl2;
extern crate time;

use std::{thread};
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;
use sdl2::gfx::primitives::DrawRenderer;
use std::f32;
use std::f32::consts::PI;
const INCREMENT_ANGLE:f32 = 2.0*PI/6.0; // 60 degrees in radians

fn get_hexagon(radius:i16) -> ([i16;6], [i16;6]) {
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
        xs[i] = xo.round() as i16;
        ys[i] = yo.round() as i16;
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


fn main() {
    /////////////////////////
    ///// CONSTANTS
    /////////////////////////
    
    // https://coolors.co/f4d06f-ff8811-9dd9d2-fff8f0-392f5a
    let color_black = Color::RGB(0,0,0);
    let color1 = Color::RGB(0xf4,0xD0,0x6F);
    let color2 = Color::RGB(0xff,0x88,0x11);
    let color3 = Color::RGB(0x9D,0x9D,0xD2);
    let color4 = Color::RGB(0xFF,0xF8,0xF0);
    let color5 = Color::RGB(0x39,0x2F,0x5A);

    let radius = 75;
    let buffer_hack = 0;

    /////////////////////////
    ///// Derived Constants
    /////////////////////////


    let hex_width = (radius * 2 + buffer_hack) as i16;
    let half_hex_height = ((INCREMENT_ANGLE).sin()*radius as f32).round() as i16;
    let hex_height = half_hex_height * 2;
    let hex_edge_len = radius; // because hexagons are made of equalateral triangles this is always equal to the radius//((INCREMENT_ANGLE/2.0).sin()*radius as f32) as i16 * 2;
    println!("hex_width {}", hex_width);
    println!("half_hex_height {}", half_hex_height);
    println!("hex_height {}", hex_height);
    println!("hex_edge_len {}", hex_edge_len);
    println!("radius {}", radius);
    let (hexagon_x, hexagon_y) = get_hexagon(radius);

    /////////////////////////
    //// SDL Setup
    /////////////////////////
    let sdl_context = sdl2::init().unwrap(); 
    let video_subsystem = sdl_context.video().unwrap();
    video_subsystem.gl_attr().set_multisample_samples(8);
    
    let window = video_subsystem.window("rust-sdl2 demo: Video", 800, 600)
        .position_centered()
        .opengl()
        .build()
        .unwrap();
    
    let mut renderer = window.renderer().build().unwrap();

    // Draw a black screen
    renderer.set_draw_color(Color::RGB(0, 0, 0));
    renderer.clear();
    renderer.present();
    
    let mut event_pump = sdl_context.event_pump().unwrap();

    /////////////////////////
    //// Main loop
    /////////////////////////
    let mut frame_count = 0;
    let mut last_time = time::now().tm_sec;
    'running: loop {
        // TODO sleep till next event?
        let ten_millis = std::time::Duration::from_millis(10);
        thread::sleep(ten_millis);

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    println!("Exiting");
                    break 'running
                },
                _ => {}
            }
        }
        renderer.set_draw_color(color_black);
        renderer.clear();
        let rows = 20;
        let cols = 10;
        for row in 0..rows {
            for col in 0..cols {
                let isEven = row % 2 == 0;
                let (xs, ys) = match isEven {
                    true => translate_hexagon(hexagon_x, hexagon_y, (hex_width + hex_edge_len) * col, row * half_hex_height),
                    false => translate_hexagon(hexagon_x, hexagon_y, (hex_width + hex_edge_len) * col + radius + hex_edge_len/2, row * half_hex_height),
                };
                match isEven {
                    true => renderer.filled_polygon(&xs, &ys, color4),
                    false => renderer.filled_polygon(&xs, &ys, color5)
                };
                //println!("{}x{} {:?} {:?}", row, col, xs.to_vec(), ys);
                renderer.polygon(&xs, &ys, color_black);
            }
        }
        renderer.present();

        // TODO render a stats section in app
        // TODO get off of the time crate. theres a std:time library that is lighter weight
        frame_count += 1;
        let time = time::now().tm_sec;
        if time != last_time  {
            println!("fps {}", frame_count);
            frame_count = 0;
            last_time = time;
        }
    }
}
