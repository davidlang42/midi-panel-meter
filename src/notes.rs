use rpi_led_matrix::{LedCanvas, LedColor};
use wmidi::{Note, Velocity, Channel, U7};

use crate::helper::{add_assign, scale};

#[derive(Debug)]
pub struct NoteSlot<const C: usize> {
    pub note: Note,
    pub channels: [Velocity; C]
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

    pub fn is_empty(&self) -> bool {
        self.channels.iter().all(|v| *v == U7::MIN)
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
        for s in 0..N {
            if let Some(slot) = &self.slots[s] {
                slot.draw(canvas, first_column + s as i32, self.colors);
            }
        }
    }

    const MIN_NOTE: Note = Note::A0;
    const MAX_NOTE: Note = Note::C8;

    pub fn set_note(&mut self, n: Note, ch: Channel, v: Velocity) {
        let c = ch.index() as usize;
        if c < C && n >= Self::MIN_NOTE && n <= Self::MAX_NOTE {
            let s = if let Some(existing) = self.find_slot(n) {
                // if note already exists, use that slot
                existing
            } else {
                // find ideal slot by scaling all 88 piano notes into the number of slots
                let ideal = (N * (n as usize - Self::MIN_NOTE as usize)) / (Self::MAX_NOTE as usize - Self::MIN_NOTE as usize + 1);
                // create a slot for this note (moving others if nessesary)
                self.create_slot(n, ideal)
            };
            // update slot
            self.slots[s].as_mut().unwrap().channels[c] = v;
            if self.slots[s].as_ref().unwrap().is_empty() {
                self.slots[s] = None;
            }
        }
    }

    fn find_slot(&mut self, n: Note) -> Option<usize> {
        for s in 0..N {
            if let Some(existing) = &mut self.slots[s] {
                if existing.note == n {
                    return Some(s);
                }
            }
        }
        None
    }

    fn create_slot(&mut self, n: Note, ideal: usize) -> usize {
        todo!()
        //     if self.slots[i].is_none() {
        //         // if ideal slot is empty, take it
        //         if v != U7::MIN {
        //             let channels = [0; C];
        //             channels[i] = v;
        //             self.slots[i] = Some(NoteSlot {
        //                 note: n,
        //                 channels
        //             });
        //         }
        //     } else {
        //         // find or create slot for this note
        //         let existing = self.find_or_create_slot(n);
        //         existing.channels[i] = v;
        //         self.remove_if_empty(i);
        //     }
        // }
    }
    /*otherwise look at what note is in the slot and move up if the new note is higher/down if the new note is lower
If higher and it keeps being higher, keep moving up until free slot, if lower and keeps being lower keep moving down until free slot
If you up and find a higher note than new (or down and find a lower note than new) without finding a gap, shift the notes up/down to make a free slot
If shifting requires going past upper or lower bound, shift others the other direction instead
Only if all slots are full of unique notes, overwrite whatever note is in slot where the new note should go (based on ordering rules above)
If a note on matches an existing slot taken (but in a new channel) then add that channel to the taken slot (with its new velocity as well)
If a note on matches an existing slot taken (in an existing channel) then update that channel in the taken slot with its new velocity, also do this if key/channel pressure message is sent
When a note is off, find that note in a slot, and remove the channel which went off, if all channels removed, free slot
When rendering the column, scale the velocity as above for each channel and "overlap" them by combining colours
 */

    pub fn set_channel(&mut self, ch: Channel, v: Velocity) {
        let c = ch.index() as usize;
        if c < C {
            for s in 0..N {
                let mut delete = false;
                if let Some(slot) = &mut self.slots[s] {
                    if slot.channels[c] > U7::MIN {
                        slot.channels[c] = v;
                        delete = slot.is_empty();
                    }
                }
                if delete {
                    self.slots[s] = None;
                }
            }
        }
    }
}