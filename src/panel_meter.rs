use rpi_led_matrix::{LedCanvas, LedColor};
use wmidi::{U7, MidiMessage, ControlFunction};
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
            MidiMessage::ControlChange(ch, ControlFunction::DAMPER_PEDAL, v) => {
                self.damper_cc[ch.index() as usize] = v.into() > 64;
            },
            MidiMessage::ControlChange(ch, ControlFunction::EXPRESSION_CONTROLLER, v) => {
                self.expression_cc[ch.index() as usize] = v;
            },
            MidiMessage::ControlChange(ch, ControlFunction::CHANNEL_VOLUME, v) => {
                self.volume_cc[ch.index() as usize] = v;
            },
            _ => {
                //TODO handle notes
            }
        }
    }

    const FLASH: LedColor = LedColor { red: 255, green: 255, blue: 255 };
    const CH1: LedColor = LedColor { red: 255, green: 0, blue: 0 };
    const CH2: LedColor = LedColor { red: 0, green: 255, blue: 0 };
    const CH3: LedColor = LedColor { red: 0, green: 0, blue: 255 };

    pub fn draw(&self, canvas: &mut LedCanvas) {
        canvas.clear();
        // LHS expression pedal
        Self::draw_value(canvas, self.expression_cc[0], 0, &Self::CH1);
        Self::draw_value(canvas, self.expression_cc[1], 1, &Self::CH2);
        Self::draw_value(canvas, self.expression_cc[2], 2, &Self::CH3);
        // LHS volume pedal
        Self::draw_value(canvas, self.volume_cc[0], 4, &Self::CH1);
        Self::draw_value(canvas, self.volume_cc[1], 5, &Self::CH2);
        Self::draw_value(canvas, self.volume_cc[2], 6, &Self::CH3);
        // notes in the middle
        const OFFSET: i32 = 8;
        for i in 0..self.notes.len() {
            if let Some(note) = self.notes[i] {
                note.draw(canvas, OFFSET + i as i32);
            }
        }
        // top right corner flash on beat
        if self.tick < 6 {
            for x in 29..32 {
                canvas.draw_line(x, 0, x, 2, &Self::FLASH);
            }
        }
        // RHS damper pedal
        Self::draw_bool(canvas, self.damper_cc[0], 29, &Self::CH1);
        Self::draw_bool(canvas, self.damper_cc[1], 30, &Self::CH2);
        Self::draw_bool(canvas, self.damper_cc[2], 31, &Self::CH3);
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
            let last_pixel = v as usize % 8 * 32;
            if full_pixels > 0 {
                canvas.draw_line(x, 16 - full_pixels, x, 15, color)
            }
            if last_pixel > 0 {
                let last_color = LedColor {
                    red: (color.red as usize * last_pixel / 256) as u8,
                    green: (color.green as usize * last_pixel / 256) as u8,
                    blue: (color.blue as usize * last_pixel / 256) as u8
                };
                canvas.set(x, 15 - full_pixels, &last_color)
            }
        }
    }
}