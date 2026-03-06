use alloc::boxed::Box;

pub trait BLEDriver: Send {
    type Error;

    fn is_connected(&self) -> Result<bool, Self::Error>;
    fn is_advertising(&self) -> Result<bool, Self::Error>;
    fn is_configured(&self) -> Result<bool, Self::Error>;

    async fn start_advertising(&mut self) -> Result<(), Self::Error>;
    async fn stop(&mut self) -> Result<(), Self::Error>;

    async fn initialize(&mut self) -> Result<(), Self::Error>;

    async fn set_connected_callback(&mut self, callback: Box<dyn Fn() + Send + 'static>);
    async fn set_disconnected_callback(&mut self, callback: Box<dyn Fn() + Send + 'static>);
    async fn set_data_callback(&mut self, callback: Box<dyn Fn(&[u8]) + Send + 'static>);

    async fn notify(&mut self, data: &[u8]) -> Result<(), Self::Error>;
}

pub struct NoBLE;

impl NoBLE {
    pub fn new() -> Self {
        Self
    }
}

impl BLEDriver for NoBLE {
    type Error = core::convert::Infallible;

    fn is_connected(&self) -> Result<bool, Self::Error> {
        Ok(false)
    }

    fn is_advertising(&self) -> Result<bool, Self::Error> {
        Ok(false)
    }

    fn is_configured(&self) -> Result<bool, Self::Error> {
        Ok(false)
    }

    async fn start_advertising(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn initialize(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn set_connected_callback(&mut self, _callback: Box<dyn Fn() + Send + 'static>) {}

    async fn set_disconnected_callback(&mut self, _callback: Box<dyn Fn() + Send + 'static>) {}

    async fn set_data_callback(&mut self, _callback: Box<dyn Fn(&[u8]) + Send + 'static>) {}

    async fn notify(&mut self, _data: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }
}
