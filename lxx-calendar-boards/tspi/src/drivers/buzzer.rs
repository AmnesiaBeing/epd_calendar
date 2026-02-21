use lxx_calendar_common::BuzzerDriver;

pub struct LinuxBuzzer;

impl BuzzerDriver for LinuxBuzzer {
    type Error = core::convert::Infallible;

    fn play_tone(&mut self, frequency: u32, duration_ms: u32) -> Result<(), Self::Error> {
        std::thread::sleep(std::time::Duration::from_millis(duration_ms as u64));
        todo!()
    }
}
