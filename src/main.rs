mod meter;
mod midi;

use meter::PanelMeter;
use midi::InputDevice;
use rpi_led_matrix::LedCanvas;
use rpi_led_matrix::{LedMatrix, LedColor, LedFont, LedMatrixOptions};
use chrono::Local;
use std::path::Path;
use std::time::Duration;
use std::thread;
use std::fs;
use std::error::Error;
use std::time::Instant;

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
            if let Ok(mut midi) = InputDevice::open(&device, true) {
                let mut panel = PanelMeter::new();
                while let Ok(message) = midi.read() {
                    panel.handle(message);
                    panel.draw(&mut canvas);
                    canvas = matrix.swap(canvas);
                }
            }
        }
        let ms = updated.elapsed().as_millis();
        if ms < 1000 {
            thread::sleep(Duration::from_millis((1000 - ms).try_into().unwrap()));
        }
    }
}

fn show_error(mut canvas: LedCanvas, matrix: &LedMatrix, font: &LedFont, text: &str) -> LedCanvas {
    canvas.clear();
    canvas.draw_text(font, text, 1, 11, &LedColor { red: 255, green: 255, blue: 255 }, 0, false);
    matrix.swap(canvas)
}

fn list_files(root: &str, prefix: &str) -> Result<Vec<String>, Box<dyn Error>> {
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