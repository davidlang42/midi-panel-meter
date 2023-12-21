mod notes;
mod meter;
mod midi;
mod helper;

use meter::PanelMeter;
use midi::NonBlockingInputDevice;
use rpi_led_matrix::LedCanvas;
use rpi_led_matrix::{LedMatrix, LedColor, LedFont, LedMatrixOptions};
use chrono::Local;
use std::path::Path;
use std::time::Duration;
use std::thread;
use std::fs;
use std::error::Error;
use std::time::Instant;

const CLOCK_UPDATE_MS: u128 = 1000;
const METER_UPDATE_MS: u128 = 100;

fn main() {
    // set up screen
    let mut options = LedMatrixOptions::new();
    options.set_rows(16);
    options.set_cols(32);
    let matrix = LedMatrix::new(Some(options), None).unwrap();
    // draw clock while waiting for midi
    let mut canvas = matrix.offscreen_canvas();
    let font_path = Path::new("6x9.bdf");
    let font = LedFont::new(&font_path).unwrap();
    let color = LedColor { red: 255, green: 255, blue: 255 };
    let mut colon = true;
    loop {
        let updated = Instant::now();
        let time = if colon {
            colon = false;
            format!("{}", Local::now().format("%H:%M"))
        } else  {
            colon = true;
            format!("{}", Local::now().format("%H %M"))
        };
        canvas.clear();
        canvas.draw_text(&font, &time, 1, 11, &color, 0, false);
        canvas = matrix.swap(canvas);
        if let Some(device) = list_files("/dev", "midi").unwrap().into_iter().next() {
            match NonBlockingInputDevice::open(&device, true) {
                Ok(midi) => canvas = show_midi_panel(midi, canvas, &matrix),
                Err(err) => println!("Error opening MIDI device: {}", err)
            }
        }
        let ms = updated.elapsed().as_millis();
        if ms < CLOCK_UPDATE_MS {
            thread::sleep(Duration::from_millis((CLOCK_UPDATE_MS - ms).try_into().unwrap()));
        }
    }
}

fn list_files(root: &str, prefix: &str) -> Result<Vec<String>, Box<dyn Error>> { //TODO doesn't find midi device when running with sudo (or on startup)
    let md = fs::metadata(root)?;
    if md.is_dir() {
        let mut files = Vec::new();
        for entry in fs::read_dir(root)? {
            let path = entry?.path();
            if !path.is_dir() && path.file_name().unwrap().to_string_lossy().starts_with(prefix) {
                files.push(path.display().to_string());
            }
        }
        files.sort();
        Ok(files)
    } else {
        Ok(vec![root.to_string()])
    }
}

fn show_midi_panel(mut midi: NonBlockingInputDevice, mut canvas: LedCanvas, matrix: &LedMatrix) -> LedCanvas {
    let mut panel = PanelMeter::new();
    panel.draw(&mut canvas);
    canvas = matrix.swap(canvas);
    while midi.is_connected() {
        let updated = Instant::now();
        let mut changed = false;
        while let Some(message) = midi.read() {
            panel.handle(message);
            changed = true;
        }
        if changed {
            panel.draw(&mut canvas);
            canvas = matrix.swap(canvas);
        }
        let ms = updated.elapsed().as_millis();
        if ms < METER_UPDATE_MS {
            thread::sleep(Duration::from_millis((METER_UPDATE_MS - ms).try_into().unwrap()));
        }
    }
    canvas
}