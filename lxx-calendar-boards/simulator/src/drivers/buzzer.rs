use lxx_calendar_common::{info, BuzzerDriver};

pub struct SimulatorBuzzer;

impl BuzzerDriver for SimulatorBuzzer {
    type Error = core::convert::Infallible;

    fn play_tone(&mut self, frequency: u32, duration_ms: u32) -> Result<(), Self::Error> {
        info!(
            "[Simulator Buzzer] Playing {}Hz for {}ms",
            frequency, duration_ms
        );
        std::thread::sleep(std::time::Duration::from_millis(duration_ms as u64));
        Ok(())
    }
}
