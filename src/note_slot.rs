use std::iter::Map;
use rpi_led_matrix::{LedCanvas, LedColor};

use wmidi::{Note, Velocity, Channel};

#[derive(Debug)]
pub struct NoteSlot {
    note: Note,
    channels: Map<Channel, Velocity>
}

impl NoteSlot {
    pub fn draw(&self, canvas: &mut LedCanvas, x: i32) {
        //TODO draw notes
    }
}