use rpi_led_matrix::{LedCanvas, LedColor};
use wmidi::{Note, Velocity, Channel, U7};

use crate::helper::{add_assign, scale};

#[derive(Debug)]
pub struct NoteSlot<const C: usize> {
    pub note: Note,
    pub channels: [Velocity; C]
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum Direction {
    None, Up, Down
}

impl<const C: usize> NoteSlot<C> {
    pub fn new(n: Note) -> Self {
        Self {
            note: n,
            channels: [U7::MIN; C]
        }
    }

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
                // move the ideal to be valid compared to other notes already existing
                let valid = self.valid_relative_to_existing(ideal, n);
                // create a slot for this note (moving others if nessesary)
                let index = self.make_free_slot(n, valid, Direction::None);
                self.slots[index] = Some(NoteSlot::new(n));
                index
            };
            // update slot
            self.slots[s].as_mut().unwrap().channels[c] = v;
            if self.slots[s].as_ref().unwrap().is_empty() {
                self.slots[s] = None;
            }
        }
    }

    fn valid_relative_to_existing(&self, ideal: usize, n: Note) -> usize {
        let mut valid = None;
        for i in (ideal + 1)..N {
            if let Some(slot) = &self.slots[i] {
                if slot.note < n {
                    valid = Some(i);
                } else if slot.note > n {
                    break;
                }
            }
        }
        if let Some(v) = valid {
            return v;
        }
        for i in (0..ideal).rev() {
            if let Some(slot) = &self.slots[i] {
                if slot.note > n {
                    valid = Some(i);
                } else if slot.note < n {
                    break;
                }
            }
        }
        if let Some(v) = valid {
            return v;
        }
        ideal
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

    fn make_free_slot(&mut self, n: Note, ideal: usize, previous: Direction) -> usize {
        if let Some(existing) = &self.slots[ideal] {
            if n > existing.note {
                // we need to move up
                if ideal == N - 1 || previous == Direction::Down {
                    // put it here
                    if self.shift_down(ideal) {
                        // shifted others down
                        ideal
                    } else if ideal < N - 1 && self.shift_up(ideal + 1) {
                        // shifted others up
                        ideal + 1
                    } else {
                        // slots full, therefore overwrite
                        ideal
                    }
                } else {
                    // keep moving up
                    self.make_free_slot(n, ideal + 1, Direction::Up)
                }
            } else if n < existing.note {
                // we need to move down
                if ideal == 0 || previous == Direction::Up {
                    // put it here
                    if self.shift_up(ideal) {
                        // shifted others up
                        ideal
                    } else if ideal > 0 && self.shift_down(ideal - 1) {
                        // shifted others down
                        ideal - 1
                    } else {
                        // slots full, therefore overwrite
                        ideal
                    }
                } else {
                    // keep moving down
                    self.make_free_slot(n, ideal - 1, Direction::Down)
                }
            } else {
                panic!("Tried to create a slot for a note that exists")
            }
        } else {
            // empty, use this slot
            ideal
        }
    }

    fn shift_up(&mut self, lower: usize) -> bool {
        let mut gap = None;
        for s in lower..N {
            if self.slots[s].is_none() {
                gap = Some(s);
                break;
            }
        }
        if let Some(upper) = gap {
            self.slots[lower..(upper + 1)].rotate_right(1);
            true
        } else {
            false
        }
    }

    fn shift_down(&mut self, upper: usize) -> bool {
        let mut gap = None;
        for s in (0..(upper + 1)).rev() {
            if self.slots[s].is_none() {
                gap = Some(s);
                break;
            }
        }
        if let Some(lower) = gap {
            self.slots[lower..(upper + 1)].rotate_left(1);
            true
        } else {
            false
        }
    }
}