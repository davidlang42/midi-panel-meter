# midi-panel-meter

A CLI application which reads MIDI_IN and displays current status of pedals and notes on a panel meter.

For instructions on how to run this on a Raspberry Pi 0-2w, click [here](hardware/SETUP.md).

## Development instructions

To modify this code locally on a non-Raspberry Pi architecture linux, you will need to [do the following](https://medium.com/swlh/compiling-rust-for-raspberry-pi-arm-922b55dbb050):
- `sudo apt install gcc-arm-linux-gnueabihf`
- `rustup target add armv7-unknown-linux-gnueabihf`
- `cargo check --target=armv7-unknown-linux-gnueabihf`

Unfortunately building is only possible on the RPi itself due to the LED driver's requirements.
