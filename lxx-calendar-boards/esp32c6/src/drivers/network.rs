use lxx_calendar_common::NetworkStack;
use lxx_calendar_common::*;

pub struct Esp32Network;

impl Esp32Network {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Esp32Network {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkStack for Esp32Network {
    type Error = core::convert::Infallible;

    // async fn dns_query(&self, _host: &str) -> Result<Vec<core::net::IpAddr>, Self::Error> {
    //     // TODO: 使用 embassy-net 的 DNS 功能
    //     // 需要获取 embassy_net::Stack 的引用
    //     info!("DNS query (stub)");
    //     // Ok(vec![])
    //     todo!()
    // }

    fn is_link_up(&self) -> bool {
        true
    }

    async fn wait_config_up(&self) -> Result<(), Self::Error> {
        info!("Waiting for network config (stub)");
        Ok(())
    }

    fn is_config_up(&self) -> bool {
        true
    }
}
