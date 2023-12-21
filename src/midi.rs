use std::collections::VecDeque;
use std::fs;
use std::error::Error;
use wmidi::FromBytesError;
use wmidi::MidiMessage;
use wmidi::U7;
use nonblock::NonBlockingReader;

pub const TICKS_PER_BEAT: usize = 24;

pub struct NonBlockingInputDevice {
    reader: NonBlockingReader<fs::File>,
    bytes: Vec<u8>,
    messages: VecDeque<MidiMessage<'static>>,
    include_clock_ticks: bool,
    rewrite_note_zero_as_off: bool
}

impl NonBlockingInputDevice {
    pub fn is_connected(&self) -> bool {
        !self.reader.is_eof()
    }

    pub fn open(midi_in: &str, include_clock_ticks: bool) -> Result<Self, Box<dyn Error>> {
        let input = fs::File::options().read(true).open(midi_in).map_err(|e| format!("Cannot open MIDI IN '{}': {}", midi_in, e))?;
        let reader = NonBlockingReader::from_fd(input)?;
        Ok(Self {
            reader,
            bytes: Vec::new(),
            messages: VecDeque::new(),
            include_clock_ticks,
            rewrite_note_zero_as_off: true
        })
    }

    pub fn read(&mut self) -> Result<Option<MidiMessage<'static>>, Box<dyn Error>> {
        let mut buf = Vec::new();
        self.reader.read_available(&mut buf)?;
        for byte in buf {
            self.process(byte);
        }
        Ok(self.messages.pop_front())
    }

    fn process(&mut self, byte: u8) {
        self.bytes.push(byte);
        match MidiMessage::try_from(self.bytes.as_slice()) {
            Ok(MidiMessage::TimingClock) if !self.include_clock_ticks => {
                // skip clock tick if not required
                self.bytes.clear();
            },
            Ok(MidiMessage::NoteOn(c, n, U7::MIN)) if self.rewrite_note_zero_as_off => {
                // some keyboards send NoteOn(velocity: 0) instead of NoteOff (eg. Kaysound MK-4902)
                self.messages.push_back(MidiMessage::NoteOff(c, n, U7::MIN));
                self.bytes.clear();
            },
            Ok(message) => {
                // message complete
                self.messages.push_back(message.to_owned());
                self.bytes.clear();
            },
            Err(FromBytesError::NoBytes) | Err(FromBytesError::NoSysExEndByte) | Err(FromBytesError::NotEnoughBytes) => {
                // wait for more bytes
            }, 
            _ => {
                // invalid message, clear and wait for next message
                self.bytes.clear();
            }
        }
    }
}