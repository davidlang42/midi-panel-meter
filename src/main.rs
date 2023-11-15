mod meter;

use rpi_led_matrix::{LedMatrix, LedColor};

fn main() {
    // rpi_led_matrix example code
    let matrix = LedMatrix::new(None, None).unwrap();
    let mut canvas = matrix.offscreen_canvas();
    for red in 0..255 {
        for green in 0..255 {
            for blue in 0..255 {
                canvas.fill(&LedColor { red, green, blue });
                canvas = matrix.swap(canvas);
            }
        }
    }
}
