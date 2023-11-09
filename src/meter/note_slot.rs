use std::iter::Map;

use wmidi::{Note, Velocity, Channel};

pub struct NoteSlot {
    note: Note,
    channels: Map<Channel, Velocity>
}