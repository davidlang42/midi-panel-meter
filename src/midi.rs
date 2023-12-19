use std::path::PathBuf;
use std::sync::mpsc;
use std::fs;
use std::thread;
use std::io::{Read, Write};
use std::error::Error;
use std::thread::JoinHandle;
use std::time::Duration;
use wmidi::FromBytesError;
use wmidi::MidiMessage;
use wmidi::U7;
use nonblock::NonBlockingReader;

pub trait MidiReceiver {
    fn passthrough_midi(&mut self, message: MidiMessage<'static>) -> Option<MidiMessage<'static>> {
        Some(message)
    }
}

pub struct InputDevice {
    receiver: mpsc::Receiver<MidiMessage<'static>>,
    threads: Vec<JoinHandle<()>>
}

pub struct ClockDevice {
    path: PathBuf
}

pub struct OutputDevice {
    sender: mpsc::Sender<MidiMessage<'static>>,
    thread: JoinHandle<()>
}

pub const TICKS_PER_BEAT: usize = 24;

impl InputDevice {
    pub fn open(midi_in: &str, include_clock_ticks: bool) -> Result<Self, Box<dyn Error>> {
        let (tx, rx) = mpsc::channel();
        let mut input = fs::File::options().read(true).open(midi_in).map_err(|e| format!("Cannot open MIDI IN '{}': {}", midi_in, e))?;
        let join_handle = thread::Builder::new().name(format!("midi-in")).spawn(move || Self::read_into_queue(&mut input, tx, include_clock_ticks, true))?;
        Ok(Self {
            receiver: rx,
            threads: vec![join_handle]
        })
    }

    pub fn open_with_external_clock(midi_in: &str, clock_in: &str) -> Result<Self, Box<dyn Error>> {
        let (tx, rx) = mpsc::channel();
        let include_clock_ticks = midi_in == clock_in;
        let mut input = fs::File::options().read(true).open(midi_in).map_err(|e| format!("Cannot open MIDI IN '{}': {}", midi_in, e))?;
        let clock = ClockDevice::init(clock_in)?;
        let mut threads = Vec::new();
        if !include_clock_ticks {
            threads.push(clock.connect(tx.clone())?);
        }
        threads.push(thread::Builder::new().name(format!("midi-in")).spawn(move || Self::read_into_queue(&mut input, tx, include_clock_ticks, true))?);
        Ok(Self {
            receiver: rx,
            threads
        })
    }

    pub fn read(&mut self) -> Result<MidiMessage<'static>, Box<dyn Error>> {
        for thread in &self.threads {
            if thread.is_finished() {
                return Err("InputDevice thread finished".into());
            }
        }
        self.receiver.recv().map_err(|e| e.into())
    }

    fn read_into_queue(f: &mut fs::File, tx: mpsc::Sender<MidiMessage>, include_clock_ticks: bool, rewrite_note_zero_as_off: bool) {
        let mut buf: [u8; 1] = [0; 1];
        let mut bytes = Vec::new();
        while f.read_exact(&mut buf).is_ok() {
            bytes.push(buf[0]);
            match MidiMessage::try_from(bytes.as_slice()) {
                Ok(MidiMessage::TimingClock) if !include_clock_ticks => {
                    // skip clock tick if not required
                    bytes.clear();
                },
                Ok(MidiMessage::NoteOn(c, n, U7::MIN)) if rewrite_note_zero_as_off => {
                    // some keyboards send NoteOn(velocity: 0) instead of NoteOff (eg. Kaysound MK-4902)
                    if tx.send(MidiMessage::NoteOff(c, n, U7::MIN)).is_err() {
                        panic!("Error rewriting NoteOn(0) as NoteOff to queue.");
                    }
                    bytes.clear();
                },
                Ok(message) => {
                    // message complete, send to queue
                    if tx.send(message.to_owned()).is_err() {
                        panic!("Error sending to queue.");
                    }
                    bytes.clear();
                },
                Err(FromBytesError::NoBytes) | Err(FromBytesError::NoSysExEndByte) | Err(FromBytesError::NotEnoughBytes) => {
                    // wait for more bytes
                }, 
                _ => {
                    // invalid message, clear and wait for next message
                    bytes.clear();
                }
            }
        }
        panic!("Input device has disconnected.");
    }
}

impl ClockDevice {
    const MIDI_TICK: u8 = 0xF8;
    
    pub fn init(midi_clock: &str) -> Result<Self, Box<dyn Error>> {
        let mut clock = Self {
            path: PathBuf::from(midi_clock)
        };
        clock.wait_for_tick(1000)?;
        Ok(clock)
    }

    pub fn wait_for_tick(&mut self, timeout_ms: u64) -> Result<(), Box<dyn Error>> {
        const SLEEP_MS: u64 = 100;
        let f = fs::File::options().read(true).open(&self.path)
            .map_err(|e| format!("Cannot open Clock device '{}': {}", self.path.display(), e))?;
        let mut noblock = NonBlockingReader::from_fd(f)?;
        let mut elapsed = 0;
        while !noblock.is_eof() && elapsed < timeout_ms {
            let mut buf = Vec::new();
            noblock.read_available(&mut buf)?;
            for byte in buf {
                if byte == Self::MIDI_TICK {
                    // tick detected
                    return Ok(());
                }
            }
            thread::sleep(Duration::from_millis(SLEEP_MS));
            elapsed += SLEEP_MS;
        }
        if noblock.is_eof() {
            Err(format!("Clock device disconnected: {}", self.path.display()).into())
        } else {
            Err(format!("Clock device did not send a clock signal within {}ms: {}", timeout_ms, self.path.display()).into())
        }
    }

    pub fn connect(self, sender: mpsc::Sender<MidiMessage<'static>>) -> Result<JoinHandle<()>, Box<dyn Error>> {
        let mut clock = fs::File::options().read(true).open(&self.path)
            .map_err(|e| format!("Cannot open Clock device '{}': {}", self.path.display(), e))?;
        Ok(thread::Builder::new().name(format!("midi-clock")).spawn(move || Self::read_clocks_into_queue(&mut clock, sender))?)
    }

    fn read_clocks_into_queue(f: &mut fs::File, tx: mpsc::Sender<MidiMessage>) {
        let mut buf: [u8; 1] = [0; 1];
        while f.read_exact(&mut buf).is_ok() {
            if buf[0] == Self::MIDI_TICK {
                // tick detected, send to queue
                if tx.send(MidiMessage::TimingClock).is_err() {
                    panic!("Error sending to queue.");
                }
            }
        }
        panic!("Clock device has disconnected.");
    }
}

impl OutputDevice {
    pub fn open(midi_out: &str) -> Result<Self, Box<dyn Error>> {
        let (tx, rx) = mpsc::channel();
        let mut output = fs::File::options().write(true).open(midi_out).map_err(|e| format!("Cannot open MIDI OUT '{}': {}", midi_out, e))?;
        let thread = thread::Builder::new().name(format!("midi-out")).spawn(move || Self::write_from_queue(&mut output, rx))?;
        Ok(Self {
            sender: tx,
            thread
        })
    }

    pub fn send(&self, message: MidiMessage<'static>) -> Result<(), mpsc::SendError<MidiMessage<'static>>> {
        if self.thread.is_finished() {
            panic!("OutputDevice thread finished");
        }
        self.sender.send(message)
    }

    pub fn clone_sender(&self) -> mpsc::Sender<MidiMessage<'static>> {
        self.sender.clone()
    }

    fn write_from_queue(f: &mut fs::File, rx: mpsc::Receiver<MidiMessage>) {
        let mut buf = Vec::new();
        for received in rx {
            let expected = received.bytes_size();
            buf.resize(expected, 0);
            match received.copy_to_slice(&mut buf) {
                Ok(found) if found != expected => panic!("Error writing midi message: Not enough bytes (expected {} found {}).", expected, found),
                Err(_) => panic!("Error writing midi message: Too many bytes (expected {}).", expected),
                _ => {}
            }
            if f.write_all(&buf).is_err() {
                panic!("Error writing to device.")
            }
            if f.flush().is_err() {
                panic!("Error flushing to device.");
            }
        }
        panic!("Writing from queue has finished.");
    }
}

/////////////////////////
// use std::sync::mpsc;
// use std::fs;
// use std::thread;
// use std::io::{Read, Write};
// use std::error::Error;
// use wmidi::{MidiMessage, ControlFunction, FromBytesError, Channel, U7};

// #[derive(Serialize, Deserialize)]
// pub struct Patch {
//     pub name: String,
//     channel: Option<u8>,
//     bank_msb: Option<u8>,
//     bank_lsb: Option<u8>,
//     program: Option<u8>
// }

// impl Patch {
//     pub fn send(&self, tx: &mpsc::Sender<MidiMessage<'static>>) {
//         let channel = Channel::from_index(match self.channel {
//             Some(ch) if ch < 16 => ch,
//             _ => 0
//         }).unwrap();
//         if let Some(msb) = self.bank_msb {
//             tx.send(MidiMessage::ControlChange(channel, ControlFunction::BANK_SELECT, U7::from_u8_lossy(msb))).unwrap();
//         }
//         if let Some(lsb) = self.bank_lsb {
//             tx.send(MidiMessage::ControlChange(channel, ControlFunction::BANK_SELECT_LSB, U7::from_u8_lossy(lsb))).unwrap();
//         }
//         if let Some(prog) = self.program {
//             tx.send(MidiMessage::ProgramChange(channel, U7::from_u8_lossy(prog))).unwrap();
//         }
//     }
// }

// pub struct ThruDevice {
//     patch_sender: mpsc::Sender<MidiMessage<'static>>,
//     patch_list: Vec<Patch>,
//     patch_index: usize
// }

// impl ThruDevice {
//     pub fn new(midi_in: Option<&str>, midi_out: &str, patch_file: Option<&str>) -> Result<Self, Box<dyn Error>> {
//         // load patches
//         let patch_list: Vec<Patch> = match patch_file {
//             Some(file) => {
//                 let json = fs::read_to_string(&file).map_err(|e| format!("Cannot read from '{}': {}", file, e))?;
//                 //TODO hack in some nicety for trailing commas, newlines instead of commas, non-quoted keys
//                 serde_json::from_str(&format!("[{}]", json)).map_err(|e| format!("Cannot parse patches from '{}': {}", file, e))?
//             },
//             None => Vec::new()
//         };
//         // open devices & initiate midi-thru
//         let (tx, rx) = mpsc::channel();
//         let mut output = fs::File::options().write(true).open(midi_out).map_err(|e| format!("Cannot open MIDI OUT '{}': {}", midi_out, e))?;
//         thread::Builder::new().name(format!("midi-out")).spawn(move || write_from_queue(&mut output, rx))?;
//         if let Some(input_file) = midi_in {
//             let mut input = fs::File::options().read(true).open(input_file).map_err(|e| format!("Cannot open MIDI IN '{}': {}", input_file, e))?;
//             let tx_clone = tx.clone();
//             thread::Builder::new().name(format!("midi-in")).spawn(move || read_into_queue(&mut input, tx_clone))?;
//         }
//         // send first patch & return connected device
//         let device = Self {
//             patch_sender: tx,
//             patch_list,
//             patch_index: 0
//         };
//         device.resend_patch();
//         Ok(device)
//     }

//     pub fn increment_patch(&mut self, delta: isize) -> Option<(usize, &Patch)> {
//         let new_index = self.patch_index as isize + delta;
//         if new_index < 0 {
//             self.set_patch(0)
//         } else {
//             self.set_patch(new_index as usize)
//         }
//     }

//     pub fn has_patches(&self) -> bool {
//         self.patch_list.len() > 0
//     }

//     pub fn set_patch(&mut self, index: usize) -> Option<(usize, &Patch)> {
//         if !self.has_patches() {
//             return None;
//         }
//         let new_index = if index >= self.patch_list.len() {
//             self.patch_list.len() - 1
//         } else {
//             index
//         };
//         if new_index != self.patch_index {
//             self.patch_index = new_index;
//             self.resend_patch();
//             self.current_patch()
//         } else {
//             None
//         }
//     }

//     fn resend_patch(&self) {
//         if self.has_patches() {
//             self.patch_list[self.patch_index].send(&self.patch_sender);
//         }
//     }

//     pub fn current_patch(&self) -> Option<(usize, &Patch)> {
//         if self.has_patches() {
//             Some((self.patch_index + 1, &self.patch_list[self.patch_index]))
//         } else {
//             None
//         }
//     }

//     pub fn next_patch(&self) -> Option<&Patch> {
//         self.patch_list.get(self.patch_index + 1)
//     }

//     pub fn previous_patch(&self) -> Option<&Patch> {
//         if self.patch_index == 0 {
//             None
//         } else {
//             self.patch_list.get(self.patch_index - 1)
//         }
//     }
// }

// fn read_into_queue(f: &mut fs::File, tx: mpsc::Sender<MidiMessage>) {
//     let mut buf: [u8; 1] = [0; 1];
//     let mut bytes = Vec::new();
//     while f.read_exact(&mut buf).is_ok() {
//         bytes.push(buf[0]);
//         match MidiMessage::try_from(bytes.as_slice()) {
//             Ok(message) => {
//                 // message complete, send to queue
//                 if tx.send(message.to_owned()).is_err() {
//                     panic!("Error sending to queue.");
//                 }
//                 bytes.clear();
//             },
//             Err(FromBytesError::NoBytes) | Err(FromBytesError::NoSysExEndByte) | Err(FromBytesError::NotEnoughBytes) => {
//                 // wait for more bytes
//             }, 
//             _ => {
//                 // invalid message, clear and wait for next message
//                 bytes.clear();
//             }
//         }
//     }
//     println!("NOTE: Input device is not connected.");
// }

// fn write_from_queue(f: &mut fs::File, rx: mpsc::Receiver<MidiMessage>) {
//     let mut buf = Vec::new();
//     for received in rx {
//         let expected = received.bytes_size();
//         buf.resize(expected, 0);
//         match received.copy_to_slice(&mut buf) {
//             Ok(found) if found != expected => panic!("Error writing midi message: Not enough bytes (expected {} found {}).", expected, found),
//             Err(_) => panic!("Error writing midi message: Too many bytes (expected {}).", expected),
//             _ => {}
//         }
//         if f.write_all(&buf).is_err() {
//             panic!("Error writing to device.")
//         }
//         if f.flush().is_err() {
//             panic!("Error flushing to device.");
//         }
//     }
//     panic!("Writing from queue has finished.");
// }