use lxx_calendar_common::*;

pub struct PowerManager<B: Battery> {
    initialized: bool,
    battery_device: Option<B>,
    event_sender: Option<LxxChannelSender<'static, SystemEvent>>,
}

impl<B: Battery> PowerManager<B> {
    pub fn new(sender: LxxChannelSender<'static, SystemEvent>) -> Self {
        Self {
            initialized: false,
            battery_device: None,
            event_sender: Some(sender),
        }
    }

    pub fn set_battery_device(&mut self, device: B) {
        self.battery_device = Some(device);
    }

    pub async fn initialize(&mut self) -> SystemResult<()> {
        info!("Initializing power manager");

        if let Some(ref mut device) = self.battery_device {
            device.initialize().await.ok();

            let sender = self.event_sender.clone();
            device
                .enable_voltage_interrupt(move || {
                    if let Some(ref s) = sender {
                        let _ = s.try_send(SystemEvent::PowerEvent(
                            PowerEvent::LowPowerModeChanged(true),
                        ));
                    }
                })
                .ok();

            let sender = self.event_sender.clone();
            device
                .enable_charging_interrupt(move || {
                    if let Some(ref s) = sender {
                        let _ = s.try_send(SystemEvent::PowerEvent(
                            PowerEvent::ChargingStateChanged(true),
                        ));
                    }
                })
                .ok();
        }

        self.initialized = true;
        info!("Power manager initialized");
        Ok(())
    }

    pub async fn is_low_battery(&mut self) -> SystemResult<bool> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        if let Some(ref mut device) = self.battery_device {
            return device
                .is_low_battery()
                .await
                .map_err(|_| SystemError::HardwareError(HardwareError::PowerError));
        }
        Ok(false)
    }

    pub async fn is_charging(&mut self) -> SystemResult<bool> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        if let Some(ref mut device) = self.battery_device {
            return device
                .is_charging()
                .await
                .map_err(|_| SystemError::HardwareError(HardwareError::PowerError));
        }
        Ok(false)
    }

    pub async fn get_voltage(&mut self) -> SystemResult<u16> {
        if !self.initialized {
            return Err(SystemError::HardwareError(HardwareError::NotInitialized));
        }
        if let Some(ref mut device) = self.battery_device {
            return device
                .read_voltage()
                .await
                .map_err(|_| SystemError::HardwareError(HardwareError::PowerError));
        }
        Ok(3700)
    }
}
