/// This file contains the interface for a hex based keyboard
extern crate sdl2;

use sdl2::render::Renderer;
use sdl2::render::Texture;
use sdl2::pixels::Color;


#[derive(Debug,PartialEq,Eq,Copy,Clone,Hash)]
pub struct HexAddr {
    pub x : i16,
    pub y : i16,
}

pub struct HexKey {
    pub pressed_color: Color,
    pub color: Color,
    pub label: Texture,
}

pub trait Keyboard {
    fn on_key_press(&self, HexAddr);
    fn on_key_release(&self, HexAddr);
    fn get_key_info(&self, addr: HexAddr, renderer: &mut Renderer) -> Result<HexKey, &'static str>;
}

