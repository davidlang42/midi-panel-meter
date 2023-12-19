mod meter;

use rpi_led_matrix::{LedMatrix, LedColor, LedFont, LedMatrixOptions};
use chrono::Local;
use std::path::Path;
use std::time::Duration;
use std::thread;

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
        let date = Local::now();
        let mut time = if colon {
            colon = false;
            format!("{}", date.format("%H:%M"))
        } else  {
            colon = true;
            format!("{}", date.format("%H %M"))
        };
        canvas.clear();
        canvas.draw_text(&font, &time, 1, 12, &color, 0, false);
        canvas = matrix.swap(canvas);
        thread::sleep(Duration::from_millis(1000));
        //TODO check for midi
    }
}
