use rpi_led_matrix::{LedCanvas, LedColor};
use wmidi::{Note, Velocity, Channel};

use crate::helper::{add_assign, scale};

#[derive(Debug)]
pub struct NoteSlot<const C: usize> {
    note: Note,
    channels: [Velocity; C]
}

impl<const C: usize> NoteSlot<C> {
    const BLANK: LedColor = LedColor { red: 0, green: 0, blue: 0 };

    pub fn draw(&self, canvas: &mut LedCanvas, x: i32, colors: &[LedColor; C]) {
        let mut full_pixels = [0; C];
        let mut last_pixel = [0; C];
        for i in 0..C {
            let v: u8 = self.channels[i].into();
            full_pixels[i] = v as usize / 8;
            last_pixel[i] = v % 8 * 32;
        }
        for led in 0..16 {
            let scales = Self::scales(led, &full_pixels, &last_pixel);
            let mut color: LedColor = Self::BLANK;
            for i in 0..C {
                if scales[i] > 0 {
                    if scales[i] < 255 {
                        add_assign(&mut color, &scale(&colors[i], scales[i]));
                    } else {
                        add_assign(&mut color, &colors[i]);
                    }
                }
            }
            canvas.set(x, 15 - led as i32, &color);
        }
    }

    fn scales(led: usize, full_pixels: &[usize; C], last_pixel: &[u8; C]) -> [u8; C] {
        let mut scales = [0; C];
        for i in 0..C {
            scales[i] = if led < full_pixels[i] {
                255
            } else if led == full_pixels[i] {
                last_pixel[i]
            } else {
                0
            };
        }
        scales
    }
}

pub struct NoteSlots<'a, const N: usize, const C: usize> {
    slots: [Option<NoteSlot<C>>; N],
    colors: &'a [LedColor; C]
}

impl<'a, const N: usize, const C: usize> NoteSlots<'a, N, C> {
    pub fn new(colors: &'a [LedColor; C]) -> Self {
        let mut slots = Vec::new();
        for _ in 0..N {
            slots.push(None);
        }
        Self {
            slots: slots.try_into().unwrap(),
            colors
        }
    }

    pub fn draw(&self, canvas: &mut LedCanvas, first_column: i32) {
        for i in 0..N {
            if let Some(slot) = &self.slots[i] {
                slot.draw(canvas, first_column + i as i32, self.colors);
            }
        }
    }

    pub fn set_note(&mut self, n: Note, ch: Channel, v: Velocity) {
        //TODO set notes
        // let i = ch.index() as usize;
        // if i < C {
        // }
    }

    pub fn set_channel(&mut self, ch: Channel, v: Velocity) {
        //TODO set channels
    }
}