use arduino_hal::port::{
    mode::{Input, Output, PullUp},
    Pin, PinOps,
};
use arduino_hal::prelude::*;
use arduino_hal::Delay;

/// Represents the raw 4-byte payload returned by a DHT11 sensor.
/// The DHT11 does not use floating-point math; it splits the integer
/// and decimal values into separate 8-bit bytes physically.
pub struct Dht11Reading {
    pub temp_int: u8,
    pub temp_dec: u8,
    pub humidity_int: u8,
    pub humidity_dec: u8,
}

// Internal helper to track timeouts safely
/// A helper struct to safely track how many microseconds have passed
/// while we wait for the sensor's electrical state to change.
/// This prevents the microcontroller from freezing forever if the sensor is disconnected.
struct DelayTracker<'a> {
    delay: &'a mut Delay,
    elapsed: u32,
}

impl<'a> DelayTracker<'a> {
    fn new(delay: &'a mut Delay) -> Self {
        Self { delay, elapsed: 0 }
    }
    fn delay_us(&mut self, us: u32) {
        self.delay.delay_us(us);
        self.elapsed += us;
    }
}

// Waits for a pin to reach a specific state, aborting if it takes too long
/// Blocks execution until the target pin reaches the desired state (HIGH or LOW).
/// If the pin does not reach the target state within `timeout_us`, it aborts
/// and returns a "TIMEOUT" error. This is crucial for bit-banging communication.
fn wait_for_state<P: PinOps>(
    pin: &Pin<Input<PullUp>, P>,
    target_state: bool,
    delay: &mut Delay,
    timeout_us: u32,
) -> Result<(), &'static str> {
    let mut tracker = DelayTracker::new(delay);
    while pin.is_high() != target_state {
        if tracker.elapsed > timeout_us {
            return Err("TIMEOUT");
        }
        tracker.delay_us(2);
    }
    Ok(())
}

/// Executes the complete 1-wire protocol to read data from a DHT11 sensor.
///
/// This function temporarily takes ownership of the Arduino Pin, converts it
/// back and forth between Output (to send the wake-up signal) and Input (to listen
/// to the 40 bits of data), and then returns the pin back to the caller alongside the result.
pub fn read<P: PinOps>(
    mut pin: Pin<Output, P>,
    delay: &mut Delay,
) -> (Pin<Output, P>, Result<Dht11Reading, &'static str>) {
    // 1. Send start signal (pull LOW for 20ms, then HIGH for 40us)
    pin.set_low();
    delay.delay_ms(20u16);
    pin.set_high();
    delay.delay_us(40u32);

    // 2. Safely transition pin to Input (with pull-up resistor)
    let pin_in = pin.into_pull_up_input();

    // Define the core read logic in a closure so we can use `?` for early returns
    let mut execute_read = || -> Result<Dht11Reading, &'static str> {
        // Wait for DHT11 acknowledge (LOW, HIGH, LOW)
        wait_for_state(&pin_in, false, delay, 10_000)?;
        wait_for_state(&pin_in, true, delay, 10_000)?;
        wait_for_state(&pin_in, false, delay, 10_000)?;

        let mut data = [0u8; 5];
        for byte in data.iter_mut() {
            for _ in 0..8 {
                // Wait for the pulse to go HIGH
                wait_for_state(&pin_in, true, delay, 10_000)?;

                // Wait 30us. If it's still HIGH, it's a '1'. If LOW, it's a '0'.
                delay.delay_us(30u32);

                *byte <<= 1;
                if pin_in.is_high() {
                    *byte |= 1;
                    // Wait for the pulse to finish (go LOW again)
                    wait_for_state(&pin_in, false, delay, 10_000)?;
                }
            }
        }

        // 3. Verify Checksum
        let checksum = data[0]
            .wrapping_add(data[1])
            .wrapping_add(data[2])
            .wrapping_add(data[3]);

        if checksum != data[4] {
            return Err("CHECKSUM_MISMATCH");
        }

        Ok(Dht11Reading {
            humidity_int: data[0],
            humidity_dec: data[1],
            temp_int: data[2],
            temp_dec: data[3],
        })
    };

    let result = execute_read();

    // 4. Safely transition the pin back to Output and idle HIGH
    let mut pin_out = pin_in.into_output();
    pin_out.set_high();

    // Return the pin back to the main loop alongside the result
    (pin_out, result)
}
