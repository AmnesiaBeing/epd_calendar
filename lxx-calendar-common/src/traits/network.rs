#![allow(async_fn_in_trait)]

use core::net::IpAddr;

pub trait NetworkStack: Send + Sync {
    type Error;

    // async fn dns_query(&self, host: &str) -> Result<alloc::vec::Vec<IpAddr>, Self::Error>;

    fn is_link_up(&self) -> bool;

    async fn wait_config_up(&self) -> Result<(), Self::Error>;

    fn is_config_up(&self) -> bool;
}

pub struct NoNetwork;

impl NetworkStack for NoNetwork {
    type Error = core::convert::Infallible;

    // async fn dns_query(&self, _host: &str) -> Result<alloc::vec::Vec<IpAddr>, Self::Error> {
    //     Ok(alloc::vec::Vec::new())
    // }

    fn is_link_up(&self) -> bool {
        false
    }

    async fn wait_config_up(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn is_config_up(&self) -> bool {
        false
    }
}
