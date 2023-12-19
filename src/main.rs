mod meter;

use rpi_led_matrix::{LedMatrix, LedColor, LedFont};
use chrono::Local;
use std::path::Path;
use std::time::Duration;
use std::thread;

fn main() {
    let matrix = LedMatrix::new(None, None).unwrap();
    // draw clock while we wait for a midi device
    let mut canvas = matrix.offscreen_canvas();
    let font_path = Path::new("6x9.bdf");
    let font = LedFont::new(&font_path).unwrap();
    let color = LedColor { red: 255, green: 255, blue: 255 };
    loop {
        let date = Local::now();
        let time = format!("{}", date.format("%H:%M:%S"));
        canvas.clear();
        canvas.draw_text(&font, &time, 0, 0, &color, 0, false);
        canvas = matrix.swap(canvas);
        thread::sleep(Duration::from_millis(1000));
        //TODO check for midi
    }
}
