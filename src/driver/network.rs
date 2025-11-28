// src/driver/network.rs
use embassy_executor::Spawner;
use embassy_net::{
    Config, Ipv4Address, Ipv4Cidr, Stack, StackResources,
    dns::DnsSocket,
    tcp::client::{TcpClient, TcpClientState},
};
use embassy_net_tuntap::TunTapDevice;
use heapless::Vec;
use reqwless::{
    client::{HttpClient, TlsConfig, TlsVerify},
    headers::ContentType,
    request::{Method, RequestBuilder},
    response::Status,
};
use static_cell::StaticCell;

use crate::common::error::{AppError, Result};
use crate::driver::lcg;

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, TunTapDevice>) -> ! {
    runner.run().await
}

pub struct NetworkDriver {
    pub stack: Stack<'static>,
    tcp_state: TcpClientState<1, 4096, 4096>,
}

impl NetworkDriver {
    pub async fn new(spawner: &Spawner) -> Result<Self> {
        // 运行此代码前，需执行'sudo tap.sh'创建"tap99"这个通道
        let device = TunTapDevice::new("tap99").map_err(|_| AppError::NetworkError)?;

        let config = Config::ipv4_static(embassy_net::StaticConfigV4 {
            address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 69, 2), 24),
            dns_servers: Vec::from_slice(&[Ipv4Address::new(223, 5, 5, 5)]).unwrap(),
            gateway: Some(Ipv4Address::new(192, 168, 69, 100)),
        });

        // Generate random seed
        let mut lcg = lcg::Lcg::new();
        let seed = lcg.next();

        // Init network stack
        static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
        let (stack, runner) = embassy_net::new(
            device,
            config,
            RESOURCES.init(StackResources::new()),
            seed as u64,
        );

        // Launch network task
        if let Err(e) = spawner.spawn(net_task(runner)) {
            log::error!("Failed to spawn net task: {}", e);
            return Err(AppError::NetworkError);
        }

        // 初始化 TCP 客户端状态
        let tcp_state = TcpClientState::<1, 4096, 4096>::new();

        Ok(Self { stack, tcp_state })
    }

    pub async fn is_connected(&self) -> bool {
        self.stack.is_link_up()
    }

    pub async fn https_get<'a>(
        &self,
        host: &str,
        url: &str,
        buffer: &'a mut [u8; 4096],
    ) -> Result<&'a [u8]> {
        // 创建 TCP 客户端和 DNS socket
        let mut tcp_client = TcpClient::new(self.stack, &self.tcp_state);
        let dns_socket = DnsSocket::new(self.stack);

        // 配置 TLS（使用默认配置，不验证证书 - 注意：生产环境应该验证）
        let mut lcg = lcg::Lcg::new();
        let seed = lcg.next();
        let mut rx_buffer: [u8; 4096] = [0; 4096];
        let mut tx_buffer: [u8; 4096] = [0; 4096];

        let config = TlsConfig::new(seed as u64, &mut rx_buffer, &mut tx_buffer, TlsVerify::None);

        // 创建 HTTP 客户端
        let mut client = HttpClient::new_with_tls(&mut tcp_client, &dns_socket, config);

        log::debug!("Making HTTPS request to: {}", url);

        // 发送 HTTP GET 请求
        let mut request_builder = client.request(Method::GET, url).await.map_err(|e| {
            log::warn!("Failed to create request builder: {:?}", e);
            AppError::NetworkError
        })?;
        request_builder = request_builder
            .content_type(ContentType::TextPlain)
            .headers(&[
                ("User-Agent", "ESP32-Weather-Client"),
                ("Accept", "application/json"),
            ]);
        let response = request_builder.send(buffer).await.map_err(|e| {
            log::warn!("HTTP send failed: {:?}", e);
            AppError::NetworkError
        })?;

        // 检查 HTTP 状态码 - 先保存状态码，避免借用问题
        let status = response.status;
        if status != Status::Ok {
            log::warn!("HTTP request failed with status: {:?}", status);
            return Err(AppError::NetworkError);
        }

        // 获取响应体
        let body = response.body();
        let bytes_read = body.read_to_end().await.map_err(|e| {
            log::warn!("Failed to read response body: {:?}", e);
            AppError::NetworkError
        })?;

        log::debug!("Received {} bytes from {}", bytes_read.len(), host);
        Ok(bytes_read)
    }
}
