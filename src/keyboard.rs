/// This file contains the interface for a hex based keyboard
extern crate sdl2;



#[derive(Debug,PartialEq,Eq,Copy,Clone,Hash)]
pub struct HexAddr {
    pub x : i16,
    pub y : i16,
}

pub struct HexKey {
    pub label: String,
    pub note: u8,
}


/// A Keyboard is responsible for labeling it's keys and knowing what to do when a given key is
/// pressed.
/// Keyboards must map from a given hexagon address to an action
pub trait Keyboard {
    fn get_key_info(&self, addr: HexAddr) -> Option<HexKey>;
}

pub struct JammerKeyboard {
}

impl Keyboard for JammerKeyboard {
    fn get_key_info(&self, addr: HexAddr) -> Option<HexKey> {
        let bottom_row =     [ "Bb", "C", "D", "E", "F#", "G#" ];
        let bottom_row_num =     [ 0, 2, 4, 6, 8, 10 ];
        let top_row =     [ "F", "G", "A", "B", "C#", "Eb" ];
        let top_row_num =     [ 7, 9, 11, 12+1, 12+3, 12+5 ];
        let x = (addr.x % 6) as usize;
        let keyset = addr.x/6;
        //println!("keyset {:?}, {}", addr, keyset);
        // TODO remove the + 12 for this  and make some keys "invalid"
        let octave = 144 - (12 + addr.y/2 * 12 + 10 - keyset*12) as u8;
        let (note, note_num) = match addr.y % 2 == 0 {
            true => (bottom_row[x].to_string(), octave + bottom_row_num[x]+12),
            false => (top_row[x].to_string(), octave + top_row_num[x]),
        };
        Some(HexKey {
            label: note, 
            note: note_num,
        })
    }
}

impl JammerKeyboard {
}
pub struct HarmonicKeyboard {
}
impl Keyboard for HarmonicKeyboard {
    fn get_key_info(&self, addr: HexAddr) -> Option<HexKey> {
        let notes = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "G#", "A", "Bb", "B" ];
        let note_num = 120 + addr.x - 3 * addr.y - addr.y/2;
        if note_num < 0 {
            return None
        }
        let note_label = note_num % 12;
        let note = notes[note_label as usize];
        Some(HexKey {
            label: note.to_string(), 
            note: note_num as u8,
        })
    }
}
