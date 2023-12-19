use std::iter::Map;

use wmidi::{Note, Velocity, Channel};

#[derive(Debug)]
pub struct NoteSlot {
    note: Note,
    channels: Map<Channel, Velocity>
}