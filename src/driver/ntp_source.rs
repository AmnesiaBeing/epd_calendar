// src/driver/ntp_source.rs
use crate::common::GlobalMutex;
use crate::common::error::AppError;
use crate::driver::network::{DefaultNetworkDriver, NetworkDriver};
use alloc::string::ToString;
use core::net::{IpAddr, SocketAddr};
use embassy_net::dns::DnsQueryType;
use embassy_net::udp::PacketMetadata;
use embassy_net::udp::UdpSocket;
use embassy_time::{Duration, Instant, Timer};
use jiff::Timestamp;
use sntpc::{NtpContext, NtpTimestampGenerator, get_time};

// 使用多个NTP服务器以提高可靠性
const NTP_SERVERS: &[&str] = &[
    "pool.ntp.org",
    "time.google.com",
    "time.windows.com",
    "ntp.ntsc.ac.cn",
];
// SNTP协议常量
const NTP_PORT: u16 = 123;
const NTP_PACKET_SIZE: usize = 48;
const NTP_EPOCH: u64 = 2208988800; // 1970-01-01 00:00:00 UTC to 1900-01-01 00:00:00 UTC

/// SNTP时间源实现
pub struct SntpSource {
    network_driver: &'static GlobalMutex<DefaultNetworkDriver>,
    buffer: [u8; NTP_PACKET_SIZE],
}

impl SntpSource {
    pub fn new(network_driver: &'static GlobalMutex<DefaultNetworkDriver>) -> Self {
        Self {
            network_driver,
            buffer: [0u8; NTP_PACKET_SIZE],
        }
    }

    /// 创建SNTP上下文
    fn create_ntp_context(&self) -> NtpContext<EmbassyTimestampGenerator> {
        log::debug!("Creating NTP context");
        let timestamp_gen = EmbassyTimestampGenerator::new();
        NtpContext::new(timestamp_gen)
    }

    /// 发送SNTP请求并获取时间
    async fn request_time(&mut self) -> Result<Timestamp, AppError> {
        if let Some(stack) = self.network_driver.lock().await.get_stack() {
            log::info!("Starting NTP time request");
            let context = self.create_ntp_context();

            log::debug!("Creating UDP socket for NTP on ESP32");
            let mut rx_meta = [PacketMetadata::EMPTY; 16];
            let mut rx_buffer = [0; 4096];
            let mut tx_meta = [PacketMetadata::EMPTY; 16];
            let mut tx_buffer = [0; 4096];

            // 创建UDP套接字
            let socket = UdpSocket::new(
                *stack,
                &mut rx_meta,
                &mut rx_buffer,
                &mut tx_meta,
                &mut tx_buffer,
            );

            let mut last_error = None;

            for server in NTP_SERVERS.iter() {
                log::info!("Resolving NTP server: {server}");

                let ntp_addrs = match stack.dns_query(server, DnsQueryType::A).await {
                    Ok(addrs) => addrs,
                    Err(e) => {
                        log::warn!("DNS query for {server} failed: {:?}", e);
                        last_error = Some(AppError::DnsError);
                        continue;
                    }
                };

                if ntp_addrs.is_empty() {
                    log::warn!("No addresses found for {server}");
                    last_error = Some(AppError::DnsError);
                    continue;
                }

                let addr: IpAddr = ntp_addrs[0].into();
                log::info!("NTP server {server} resolved to: {addr}");

                log::debug!("Sending NTP request to {addr}:123");
                let result = get_time(SocketAddr::from((addr, NTP_PORT)), &socket, context).await;

                match result {
                    Ok(ntp_result) => {
                        log::info!("NTP request to {server} success");

                        // 将NTP时间转换为Timestamp
                        let ntp_seconds = ntp_result.sec();
                        let ntp_fraction = ntp_result.sec_fraction();
                        // 转换为微秒
                        let microsecond = (ntp_seconds as u64 - NTP_EPOCH) * 1_000_000
                            + (u64::from(ntp_fraction) * 1_000_000 / u64::from(u32::MAX));

                        // 使用jiff创建Timestamp
                        match Timestamp::from_microsecond(microsecond as i64) {
                            Ok(timestamp) => {
                                log::info!("NTP time from {server}: {}", timestamp.to_string());
                                return Ok(timestamp);
                            }
                            Err(e) => {
                                log::warn!("Failed to create timestamp from microsecond: {:?}", e);
                                last_error = Some(AppError::TimeError);
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("NTP request to {server} failed: {:?}", e);
                        last_error = Some(AppError::TimeError);
                        continue;
                    }
                }
            }

            log::error!("All NTP servers failed");
            return Err(last_error.unwrap_or(AppError::TimeError));
        }
        Err(AppError::NetworkError)
    }
}

impl SntpSource {
    /// 异步获取并同步时间（供任务调用）
    pub async fn sync_time(&mut self) -> Result<Timestamp, AppError> {
        // 重试机制
        for _ in 0..3 {
            match self.request_time().await {
                Ok(time) => return Ok(time),
                Err(e) => {
                    Timer::after(Duration::from_secs(1)).await;
                    log::warn!("SNTP request failed: {:?}", e);
                }
            }
        }
        Err(AppError::SntpSyncFailed)
    }
}

// 共享的时间戳生成器
#[derive(Clone, Copy)]
pub struct EmbassyTimestampGenerator {
    start_time: Option<Instant>,
}

impl EmbassyTimestampGenerator {
    pub fn new() -> Self {
        log::debug!("Creating new EmbassyTimestampGenerator");
        Self {
            start_time: Some(Instant::now()),
        }
    }
}

impl NtpTimestampGenerator for EmbassyTimestampGenerator {
    fn init(&mut self) {
        log::debug!("Initializing timestamp generator");
        self.start_time = Some(Instant::now());
    }

    fn timestamp_sec(&self) -> u64 {
        let sec = self
            .start_time
            .map(|start| start.elapsed().as_secs())
            .unwrap_or(0);
        log::trace!("Timestamp seconds: {sec}");
        sec
    }

    fn timestamp_subsec_micros(&self) -> u32 {
        let micros = self
            .start_time
            .map(|start| {
                let elapsed = start.elapsed();
                (elapsed.as_micros() % 1_000_000) as u32
            })
            .unwrap_or(0);
        log::trace!("Timestamp microseconds: {micros}");
        micros
    }
}
