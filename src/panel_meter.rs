use rpi_led_matrix::{LedCanvas, LedColor};
use wmidi::{U7, MidiMessage, ControlFunction};
use crate::midi;
use crate::helper::scale;

use super::note_slot::NoteSlot;

pub struct PanelMeter {
    expression_cc: [U7; Self::MIDI_CHANNELS],
    notes: [Option<NoteSlot>; Self::NOTE_SLOTS],
    damper_cc: [bool; Self::MIDI_CHANNELS],
    tick: usize
}

impl PanelMeter {
    const MIDI_CHANNELS: usize = 3;
    const NOTE_SLOTS: usize = 24;

    pub fn new() -> Self {
        let zero: U7 = 0.try_into().unwrap();
        let mut notes = Vec::new();
        for _ in 0..20 {
            notes.push(None);
        }
        Self {
            expression_cc: [zero; Self::MIDI_CHANNELS],
            notes: notes.try_into().unwrap(),
            damper_cc: [false; Self::MIDI_CHANNELS],
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
            MidiMessage::ControlChange(ch, ControlFunction::DAMPER_PEDAL, v) => {
                let i: usize = ch.index() as usize;
                if i < Self::MIDI_CHANNELS {
                    let v_u8: u8 = v.into();
                    self.damper_cc[i] = v_u8 > 64;
                }
            },
            MidiMessage::ControlChange(ch, ControlFunction::EXPRESSION_CONTROLLER, v) => {
                let i = ch.index() as usize;
                if i < Self::MIDI_CHANNELS {
                    self.expression_cc[i] = v;
                }
            },
            _ => {
                //TODO handle notes
            }
        }
    }

    const CH_COLORS: [LedColor; Self::MIDI_CHANNELS] = [
        LedColor { red: 255, green: 0, blue: 0 },
        LedColor { red: 0, green: 255, blue: 0 },
        LedColor { red: 0, green: 0, blue: 255 }
    ];

    const FLASH: LedColor = LedColor { red: 255, green: 255, blue: 255 };

    pub fn draw(&self, canvas: &mut LedCanvas) {
        canvas.clear();
        // LHS expression pedal
        const FIRST_EXP_COL: i32 = 0;
        for i in 0..self.expression_cc.len() {
            Self::draw_value(canvas, self.expression_cc[i], FIRST_EXP_COL + i as i32, &Self::CH_COLORS[i]);
        }
        // notes in the middle
        const FIRST_NOTE_COL: i32 = 4;
        for i in 0..self.notes.len() {
            if let Some(note) = &self.notes[i] {
                note.draw(canvas, FIRST_NOTE_COL + i as i32);
            }
        }
        // RHS damper pedal
        const FIRST_DAMP_COL: i32 = 29;
        for i in 0..self.damper_cc.len() {
            Self::draw_bool(canvas, self.damper_cc[i], FIRST_DAMP_COL + i as i32, &Self::CH_COLORS[i]);
        }
        // top right corner flash on beat
        if self.tick < 6 {
            for x in FIRST_DAMP_COL..canvas.canvas_size().0 {
                canvas.draw_line(x, 0, x, 2, &Self::FLASH);
            }
        }
    }

    fn draw_bool(canvas: &mut LedCanvas, b: bool, x: i32, color: &LedColor) {
        if b {
            canvas.draw_line(x, 4, x, 15, color)
        }
    }

    fn draw_value(canvas: &mut LedCanvas, value: U7, x: i32, color: &LedColor) {
        let v: u8 = value.into();
        if v == 127 {
            canvas.draw_line(x, 0, x, 15, color);
        } else {
            let full_pixels = v as i32 / 8;
            let last_pixel = v % 8 * 32;
            if full_pixels > 0 {
                canvas.draw_line(x, 16 - full_pixels, x, 15, color)
            }
            if last_pixel > 0 {
                canvas.set(x, 15 - full_pixels, &scale(color, last_pixel))
            }
        }
    }
}
