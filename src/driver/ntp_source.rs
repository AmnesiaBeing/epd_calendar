// src/driver/ntp_source.rs
use alloc::string::ToString;
use core::net::{IpAddr, SocketAddr};
use embassy_executor::Spawner;
use embassy_net::dns::DnsQueryType;
use embassy_net::udp::PacketMetadata;
use embassy_net::udp::UdpSocket;
use embassy_time::with_timeout;
use embassy_time::{Duration, Instant, Ticker};
use jiff::Timestamp;
use sntpc::{NtpContext, NtpTimestampGenerator, get_time};

use crate::common::GlobalMutex;
use crate::common::error::AppError;
use crate::common::error::Result;
use crate::driver::network::{DefaultNetworkDriver, NetworkDriver};
use crate::driver::time_source::DefaultTimeSource;
use crate::driver::time_source::TimeSource;

const SNTP_TIMEOUT_SECONDS: u64 = 5;
#[cfg(any(feature = "embedded_esp", feature = "embedded_linux"))]
const SNTP_SYNC_INTERVAL_SECONDS: u64 = 12 * 60 * 60;
#[cfg(feature = "simulator")]
const SNTP_SYNC_INTERVAL_SECONDS: u64 = 60;

#[embassy_executor::task]
async fn start_sntp_task(
    time_source: &'static GlobalMutex<DefaultTimeSource>,
    mut sntp_service: SntpService,
) {
    log::info!("🕒 SNTP task started");

    let mut ticker = Ticker::every(Duration::from_secs(SNTP_SYNC_INTERVAL_SECONDS));

    // 任务启动时立即同步一次
    match perform_sntp_sync(&mut sntp_service, time_source).await {
        Ok(()) => log::info!("Initial SNTP sync successful"),
        Err(e) => log::warn!("Initial SNTP sync failed: {:?}", e),
    }

    loop {
        ticker.next().await;

        log::info!("Performing scheduled SNTP time sync");
        match perform_sntp_sync(&mut sntp_service, time_source).await {
            Ok(()) => log::info!("Scheduled SNTP sync completed successfully"),
            Err(e) => log::warn!("Scheduled SNTP sync failed: {:?}", e),
        }
    }
}

async fn perform_sntp_sync(
    sntp_service: &mut SntpService,
    time_source: &'static GlobalMutex<DefaultTimeSource>,
) -> Result<()> {
    let timestamp = sntp_service.request_time().await?;
    log::info!("Received NTP timestamp: {}", timestamp);
    let _ = time_source.lock().await.set_time(timestamp);
    Ok(())
}

// 使用多个NTP服务器以提高可靠性
const NTP_SERVERS: &[&str] = &[
    "cn.ntp.org.cn",
    "pool.ntp.org",
    "ntp.ntsc.ac.cn",
    "ntp7.aliyun.com",
    "time.windows.com",
    "time1.google.com",
];

// SNTP协议常量
const NTP_PORT: u16 = 123;

/// SNTP时间源实现
pub struct SntpService {
    network_driver: &'static GlobalMutex<DefaultNetworkDriver>,
}

impl SntpService {
    pub fn initialize(
        spawner: &Spawner,
        network_driver: &'static GlobalMutex<DefaultNetworkDriver>,
        time_source: &'static GlobalMutex<DefaultTimeSource>,
    ) {
        let sntp_service = Self::new(network_driver);
        spawner
            .spawn(start_sntp_task(time_source, sntp_service))
            .unwrap();
    }

    fn new(network_driver: &'static GlobalMutex<DefaultNetworkDriver>) -> Self {
        Self { network_driver }
    }

    /// 创建SNTP上下文
    fn create_ntp_context(&self) -> NtpContext<EmbassyTimestampGenerator> {
        log::debug!("Creating NTP context");
        let timestamp_gen = EmbassyTimestampGenerator::new();
        NtpContext::new(timestamp_gen)
    }

    /// 发送SNTP请求并获取时间
    async fn request_time(&mut self) -> Result<Timestamp> {
        if let Some(stack) = self.network_driver.lock().await.get_stack() {
            log::info!("Starting NTP time request");
            let context = self.create_ntp_context();

            log::debug!("Creating UDP socket for NTP");
            let mut rx_meta = [PacketMetadata::EMPTY; 16];
            let mut rx_buffer = [0; 4096];
            let mut tx_meta = [PacketMetadata::EMPTY; 16];
            let mut tx_buffer = [0; 4096];

            // 创建UDP套接字
            let mut socket = UdpSocket::new(
                *stack,
                &mut rx_meta,
                &mut rx_buffer,
                &mut tx_meta,
                &mut tx_buffer,
            );
            socket.bind(123).unwrap();

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
                let result = with_timeout(
                    Duration::from_secs(SNTP_TIMEOUT_SECONDS),
                    get_time(SocketAddr::from((addr, NTP_PORT)), &socket, context),
                )
                .await;

                match result {
                    Ok(Ok(ntp_result)) => {
                        log::info!("NTP request to {server} success");

                        // 将NTP时间转换为Timestamp
                        let ntp_seconds = ntp_result.sec();
                        let ntp_fraction = ntp_result.sec_fraction();
                        // 转换为微秒
                        let microsecond = ntp_seconds as u64 * 1_000_000
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
                    _ => {
                        log::warn!("NTP request to {server} failed: {:?}", result);
                        last_error = Some(AppError::TimeError);
                        continue;
                    }
                }
            }

            log::error!("All NTP servers failed. Last error: {:?}", last_error);
            return Err(last_error.unwrap_or(AppError::TimeError));
        }
        log::error!("Network stack not available for NTP request");
        Err(AppError::NetworkError)
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
