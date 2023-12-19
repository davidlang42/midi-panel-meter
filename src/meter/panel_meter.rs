use rpi_led_matrix::{LedCanvas, LedColor};
use wmidi::{U7, MidiMessage};
use crate::midi;

use super::note_slot::NoteSlot;

const MIDI_CHANNELS: usize = 16;

pub struct PanelMeter {
    expression_cc: [U7; MIDI_CHANNELS],
    volume_cc: [U7; MIDI_CHANNELS],
    notes: [Option<NoteSlot>; 20],
    damper_cc: [bool; MIDI_CHANNELS],
    tick: usize
}

impl PanelMeter {
    pub fn new() -> Self {
        let zero: U7 = 0.try_into().unwrap();
        let mut notes = Vec::new();
        for _ in 0..20 {
            notes.push(None);
        }
        Self {
            expression_cc: [zero; MIDI_CHANNELS],
            volume_cc: [zero; MIDI_CHANNELS],
            notes: notes.try_into().unwrap(),
            damper_cc: [false; MIDI_CHANNELS],
            tick: 0
        }
    }

    pub fn handle(&mut self, message: MidiMessage<'static>) {
        match message {
            MidiMessage::TimingClock => {
                self.tick = if self.tick == midi::TICKS_PER_BEAT {
                    0
                } else {
                    self.tick + 1
                };
            },
            _ => {
                //TODO
            }
        }
    }

    pub fn draw(&self, canvas: &mut LedCanvas) {
        canvas.clear();
        if self.tick < 8 {
            canvas.set(4, 8, &LedColor { red: 255, green: 0, blue: 0 });
        } else if self.tick < 16 {
            canvas.set(4, 8, &LedColor { red: 0, green: 255, blue: 0 });
        } else if self.tick < 24 {
            canvas.set(4, 8, &LedColor { red: 0, green: 0, blue: 255 });
        }
        //TODO
    }
}