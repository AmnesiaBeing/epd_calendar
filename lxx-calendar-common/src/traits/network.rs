pub trait NetworkStack {
    type Error;

    fn is_link_up(&self) -> bool;

    async fn wait_config_up(&self) -> Result<(), Self::Error>;

    fn get_stack(&self) -> Option<&embassy_net::Stack<'static>>;
}

pub struct NoNetwork;

impl NetworkStack for NoNetwork {
    type Error = core::convert::Infallible;

    fn is_link_up(&self) -> bool {
        false
    }

    async fn wait_config_up(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn get_stack(&self) -> Option<&embassy_net::Stack<'static>> {
        None
    }
}
