use esp_hal::analog::adc::AdcPin;
use esp_hal::analog::adc::{Adc, AdcConfig, Attenuation};
use esp_hal::gpio::AnyPin;
use esp_hal::peripherals::GPIO2;
use esp_hal::peripherals::Peripherals;
use lxx_calendar_common::Battery;

use lxx_calendar_common::*;

const VOLTAGE_THRESHOLD_MV: u16 = 3000;

pub struct Esp32Battery {
    adc: Adc<'static, esp_hal::peripherals::ADC1<'static>, esp_hal::Blocking>,
    pin: AdcPin<GPIO2<'static>, esp_hal::peripherals::ADC1<'static>>,
}

impl Esp32Battery {
    pub fn new(peripherals: &Peripherals) -> Self {
        let mut adc_config = AdcConfig::new();
        let voltage_pin = adc_config.enable_pin(
            unsafe { peripherals.GPIO2.clone_unchecked() },
            Attenuation::_11dB,
        );
        let adc = Adc::new(unsafe { peripherals.ADC1.clone_unchecked() }, adc_config);

        Self {
            adc,
            pin: voltage_pin,
        }
    }
}

impl Battery for Esp32Battery {
    type Error = core::convert::Infallible;

    async fn initialize(&mut self) -> Result<(), Self::Error> {
        info!("Initializing ESP32 battery driver (GPIO2 ADC1_CH2)");
        info!("Battery driver initialized");
        Ok(())
    }

    async fn read_voltage(&mut self) -> Result<u16, Self::Error> {
        let pin_value: u16 = self.adc.read_oneshot(&mut self.pin).unwrap();
        let voltage_mv = (pin_value as u32 * 3300) / 4095;
        Ok(voltage_mv as u16)
    }

    async fn is_low_battery(&mut self) -> Result<bool, Self::Error> {
        let voltage = self.read_voltage().await?;
        Ok(voltage < VOLTAGE_THRESHOLD_MV)
    }

    async fn is_charging(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }

    // TODO: 中断暂未完成，使用ADC Monitor中断
    fn enable_voltage_interrupt<F>(&mut self, _callback: F) -> Result<(), Self::Error>
    where
        F: Fn() + Send + 'static,
    {
        info!(
            "Voltage interrupt enabled, threshold: {}mV",
            VOLTAGE_THRESHOLD_MV
        );
        Ok(())
    }

    // TODO: 中断暂未完成，应使用GPIO中断
    fn enable_charging_interrupt<F>(&mut self, _callback: F) -> Result<(), Self::Error>
    where
        F: Fn() + Send + 'static,
    {
        info!("Charging interrupt enabled");
        Ok(())
    }
}
