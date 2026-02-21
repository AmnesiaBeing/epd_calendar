pub use sntpc::Error as SntpError;
pub use sntpc::NtpContext;
pub use sntpc::NtpTimestampGenerator;
pub use sntpc::NtpUdpSocket;
pub use sntpc::get_time as ntp_get_time;

use embassy_net::Stack;
use embassy_net::udp::PacketMetadata;
use embassy_time::Duration;

pub const NTP_SERVER_ALIYUN: &str = "ntp.aliyun.com";
pub const NTP_SERVER_TENCENT: &str = "ntp.tencent.com";
pub const NTP_SERVER_POOL: &str = "cn.pool.ntp.org";
pub const NTP_SERVER_DEFAULT: &str = "time.pool.aliyun.com";

const NTP_SERVERS: [&str; 4] = [
    NTP_SERVER_DEFAULT,
    NTP_SERVER_ALIYUN,
    NTP_SERVER_TENCENT,
    NTP_SERVER_POOL,
];

pub const NTP_PORT: u16 = 123;
pub const NTP_TIMEOUT_MS: u64 = 5000;
pub const NTP_PACKET_SIZE: usize = 48;

pub trait SntpClient {
    async fn get_time(&mut self) -> Result<i64, SntpError>;
}

pub struct EmbassySntpWithStack<'a> {
    stack: Stack<'a>,
}

impl<'a> EmbassySntpWithStack<'a> {
    pub fn new(stack: Stack<'a>) -> Self {
        Self { stack }
    }
}

impl<'a> SntpClient for EmbassySntpWithStack<'a> {
    async fn get_time(&mut self) -> Result<i64, SntpError> {
        self.get_time_with_timeout(Duration::from_millis(NTP_TIMEOUT_MS))
            .await
    }
}

impl<'a> EmbassySntpWithStack<'a> {
    pub async fn get_time_with_timeout(&mut self, timeout: Duration) -> Result<i64, SntpError> {
        for server in NTP_SERVERS {
            if let Ok(timestamp) = self.try_get_time_from_server(server, timeout).await {
                return Ok(timestamp);
            }
        }
        Err(SntpError::AddressResolve)
    }

    async fn try_get_time_from_server(
        &mut self,
        server: &str,
        timeout: Duration,
    ) -> Result<i64, SntpError> {
        use sntpc::{sntp_process_response, sntp_send_request};
        use sntpc_net_embassy::UdpSocketWrapper;

        let mut rx_meta = [PacketMetadata::EMPTY; 1];
        let mut rx_buffer = [0u8; NTP_PACKET_SIZE];
        let mut tx_meta = [PacketMetadata::EMPTY; 1];
        let mut tx_buffer = [0u8; NTP_PACKET_SIZE];

        let mut socket = embassy_net::udp::UdpSocket::new(
            self.stack,
            &mut rx_meta,
            &mut rx_buffer,
            &mut tx_meta,
            &mut tx_buffer,
        );

        socket.bind(0).map_err(|_| SntpError::Network)?;

        let wrapper = UdpSocketWrapper::new(socket);

        let addrs = self
            .stack
            .dns_query(server, embassy_net::dns::DnsQueryType::A)
            .await
            .map_err(|_| SntpError::AddressResolve)?;

        let addr = core::net::SocketAddr::from((addrs[0], NTP_PORT));

        let context = NtpContext::new(EmbassyTimestampGen);

        let send_result = sntp_send_request(addr, &wrapper, context)
            .await
            .map_err(|_| SntpError::Network)?;

        let process_fut = sntp_process_response(addr, &wrapper, context, send_result);

        let result = embassy_time::with_timeout(timeout, process_fut)
            .await
            .map_err(|_| SntpError::Network)??;

        let ntp_timestamp = result.sec();
        const NTP_TO_UNIX_OFFSET: u32 = 2208988800;
        let unix_timestamp = (ntp_timestamp - NTP_TO_UNIX_OFFSET) as i64;

        Ok(unix_timestamp)
    }
}

#[derive(Copy, Clone)]
pub struct EmbassyTimestampGen;

impl NtpTimestampGenerator for EmbassyTimestampGen {
    fn init(&mut self) {}

    fn timestamp_sec(&self) -> u64 {
        embassy_time::Instant::now().elapsed().as_secs()
    }

    fn timestamp_subsec_micros(&self) -> u32 {
        embassy_time::Instant::now().elapsed().as_micros() as u32 % 1_000_000
    }
}
