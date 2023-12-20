use rpi_led_matrix::LedColor;

pub fn scale(orginal: &LedColor, scale: u8) -> LedColor {
    LedColor {
        red: (orginal.red as usize * scale as usize / 256) as u8,
        green: (orginal.green as usize * scale as usize / 256) as u8,
        blue: (orginal.blue as usize * scale as usize / 256) as u8
    }
}

pub fn add_assign(a: &mut LedColor, b: &LedColor) {
    let mut r: usize = a.red as usize + b.red as usize;
    let mut g: usize = a.green as usize + b.green as usize;
    let mut b: usize = a.blue as usize + b.blue as usize;
    if r > 255 {
        r = 255
    }
    if g > 255 {
        g = 255
    }
    if b > 255 {
        b = 255
    }
    a.red = r as u8;
    a.green = g as u8;
    a.blue = b as u8;
}