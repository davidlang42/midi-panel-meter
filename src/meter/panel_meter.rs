use wmidi::U7;
use super::note_slot::NoteSlot;

const MIDI_CHANNELS: usize = 16;

pub struct PanelMeter {
    expression_cc: [U7; MIDI_CHANNELS],
    volume_cc: [U7; MIDI_CHANNELS],
    notes: [Option<NoteSlot>; 20],
    damper_cc: [bool; MIDI_CHANNELS],
    tick: u8
}