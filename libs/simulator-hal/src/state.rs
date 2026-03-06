//! Simulator Shared State

use crate::{SimulatorConfig, SimulatedBle, SimulatedWifi};
use std::sync::Arc;

/// Simulator State - holds all shared components
pub struct SimulatorState {
    pub config: Arc<SimulatorConfig>,
    pub wifi: Arc<SimulatedWifi>,
    pub ble: Arc<SimulatedBle>,
}

impl SimulatorState {
    pub fn new() -> Arc<Self> {
        let config = SimulatorConfig::new();
        let wifi = SimulatedWifi::new();
        let ble = SimulatedBle::new(Arc::clone(&config), Arc::clone(&wifi));

        Arc::new(Self {
            config,
            wifi,
            ble,
        })
    }
}

impl Default for SimulatorState {
    fn default() -> Self {
        Self::new()
    }
}
