#![no_std]
#![no_main]

mod dht11;

use arduino_hal::prelude::*;
use panic_halt as _;
use weather_core::TelemetryPayload;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);
    let mut delay = arduino_hal::Delay::new();
    let mut led = pins.d13.into_output();

    // Initialize DHT11 pin as an Output, idling HIGH
    let mut dht_pin = pins.d8.into_output();
    dht_pin.set_high();

    // Let the serial interface settle before broadcasting
    delay.delay_ms(300u16);
    ufmt::uwriteln!(&mut serial, "{{\"status\":\"FIRMWARE_INIT\"}}").ok();

    /// THE MAIN EVENT LOOP
    /// 1. Toggles the heartbeat LED so we know the loop isn't frozen.
    /// 2. Hands the D8 pin to the `dht11::read` function.
    /// 3. Catches the returned pin and data.
    /// 4. Converts the raw `u8` bytes into our `TelemetryPayload` from the `weather-core` crate.
    /// 5. Serializes the payload to JSON and blasts it out over the 57600 baud serial connection.
    loop {
        led.toggle();

        // Pass ownership of the pin to `read()`, and receive it back as `restored_pin`
        let (restored_pin, reading) = dht11::read(dht_pin, &mut delay);

        // Reassign the restored pin for the next loop iteration
        dht_pin = restored_pin;

        match reading {
            Ok(data) => {
                let payload = TelemetryPayload::from_raw_dht11(
                    data.temp_int,
                    data.temp_dec,
                    data.humidity_int,
                    data.humidity_dec,
                );
                ufmt::uwriteln!(&mut serial, "{}", payload).ok();
            }
            Err(err) => {
                ufmt::uwriteln!(&mut serial, "{{\"error\":\"{}\"}}", err).ok();
            }
        }

        arduino_hal::delay_ms(2000);
    }
}
