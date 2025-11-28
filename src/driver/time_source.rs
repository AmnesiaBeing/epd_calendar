// src/driver/time_source.rs
use chrono::{DateTime, Local, Utc};
use core::net::{IpAddr, SocketAddr};
use core::sync::atomic::{AtomicU64, Ordering};
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
use embassy_net::Stack;
use embassy_net::dns::DnsQueryType;
use embassy_net::udp::{PacketMetadata, UdpSocket};
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
use embassy_sync::mutex::Mutex;
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
use embassy_time::Instant;
use sntpc::{NtpContext, NtpTimestampGenerator};

use crate::common::error::{AppError, Result};
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
use crate::driver::network::NetworkDriver;

pub trait TimeSource {
    /// 获取当前时间
    async fn get_time(&self) -> Result<DateTime<Local>>;

    /// 通过SNTP更新时间
    async fn update_time_by_sntp(&mut self) -> Result<()>;

    /// 获取时间戳（64位微秒，模拟ESP32 RTC精度）
    fn get_timestamp_us(&self) -> Result<u64>;
}

/// 模拟器RTC时间源 - 模拟ESP32的RTC行为
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub struct SimulatedRtc {
    // 模拟ESP32 RTC的64位微秒时间戳
    timestamp_us: AtomicU64,
    // 起始时间点
    start_time: Instant,
    // NTP时间源
    ntp_time_source: DefaultNtpTimeSource,
}

impl SimulatedRtc {
    pub fn new(ntp_time_source: DefaultNtpTimeSource) -> Self {
        let now = Utc::now();
        let timestamp_us =
            (now.timestamp() * 1_000_000 + now.timestamp_subsec_micros() as i64) as u64;
        Self {
            timestamp_us: AtomicU64::new(timestamp_us),
            start_time: Instant::now(),
            ntp_time_source,
        }
    }

    /// 更新时间戳（内部方法）
    fn update_timestamp(&self, new_timestamp_us: u64) {
        self.timestamp_us.store(new_timestamp_us, Ordering::Release);
    }
}

impl TimeSource for SimulatedRtc {
    async fn get_time(&self) -> Result<DateTime<Local>> {
        let timestamp_us = self.timestamp_us.load(Ordering::Acquire);
        let seconds = (timestamp_us / 1_000_000) as i64;
        let micros = (timestamp_us % 1_000_000) as u32;

        let utc = DateTime::from_timestamp(seconds, micros * 1000).ok_or(AppError::TimeError)?;

        // 转换为本地时间
        Ok(utc.with_timezone(&Local))
    }

    async fn update_time_by_sntp(&mut self) -> Result<()> {
        match self.ntp_time_source.get_ntp_time().await {
            Ok(ntp_time) => {
                // 将NTP时间转换为微秒时间戳
                let timestamp_us = (ntp_time.timestamp() * 1_000_000
                    + ntp_time.timestamp_subsec_micros() as i64)
                    as u64;

                // 更新RTC时间戳
                self.update_timestamp(timestamp_us);
                self.start_time = Instant::now(); // 重置起始时间
                Ok(())
            }
            Err(e) => {
                log::error!("SNTP time update failed: {:?}", e);
                Err(AppError::TimeError)
            }
        }
    }

    fn get_timestamp_us(&self) -> Result<u64> {
        let elapsed = self.start_time.elapsed().as_micros() as u64;
        let base_timestamp = self.timestamp_us.load(Ordering::Acquire);
        Ok(base_timestamp + elapsed)
    }
}

// 默认时间源选择
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub type DefaultTimeSource = SimulatedRtc;

#[cfg(feature = "embedded_esp")]
pub type DefaultTimeSource = RtcTimeSource;

#[derive(Clone, Copy)]
pub struct EmbassyTimestampGenerator {
    start_time: Option<Instant>,
}

impl EmbassyTimestampGenerator {
    pub fn new() -> Self {
        Self {
            start_time: Some(Instant::now()),
        }
    }
}

impl NtpTimestampGenerator for EmbassyTimestampGenerator {
    fn init(&mut self) {
        self.start_time = Some(Instant::now());
    }

    fn timestamp_sec(&self) -> u64 {
        self.start_time
            .map(|start| start.elapsed().as_secs())
            .unwrap_or(0)
    }

    fn timestamp_subsec_micros(&self) -> u32 {
        self.start_time
            .map(|start| {
                let elapsed = start.elapsed();
                (elapsed.as_micros() % 1_000_000) as u32
            })
            .unwrap_or(0)
    }
}

pub trait NtpTimeSource {
    /// 获取NTP时间（返回UTC时间）
    async fn get_ntp_time(&self) -> Result<DateTime<Utc>>;
}

/// 模拟器NTP时间源
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub struct SimulatedNtp {
    network_driver: &'static Mutex<ThreadModeRawMutex, NetworkDriver>,
}

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
impl SimulatedNtp {
    pub fn new(network_driver: &'static Mutex<ThreadModeRawMutex, NetworkDriver>) -> Self {
        Self { network_driver }
    }

    /// 创建SNTP上下文
    fn create_ntp_context(&self) -> NtpContext<EmbassyTimestampGenerator> {
        let timestamp_gen = EmbassyTimestampGenerator::new();
        NtpContext::new(timestamp_gen)
    }
}

impl NtpTimeSource for SimulatedNtp {
    async fn get_ntp_time(&self) -> Result<DateTime<Utc>> {
        use sntpc::get_time;

        let context = self.create_ntp_context();

        // Create UDP socket
        let mut rx_meta = [PacketMetadata::EMPTY; 16];
        let mut rx_buffer = [0; 4096];
        let mut tx_meta = [PacketMetadata::EMPTY; 16];
        let mut tx_buffer = [0; 4096];

        let mut socket = UdpSocket::new(
            self.network_driver.lock().await.stack,
            &mut rx_meta,
            &mut rx_buffer,
            &mut tx_meta,
            &mut tx_buffer,
        );
        socket.bind(123).unwrap();

        let ntp_addrs = self
            .network_driver
            .lock()
            .await
            .stack
            .dns_query("cn.pool.ntp.org", DnsQueryType::A)
            .await
            .map_err(|_| AppError::DnsError)?;
        if ntp_addrs.is_empty() {
            log::error!("Failed to resolve DNS");
            return Err(AppError::DnsError);
        }

        let addr: IpAddr = ntp_addrs[0].into();
        let result = get_time(SocketAddr::from((addr, 123)), &socket, context).await;

        match result {
            Ok(ntp_result) => {
                log::info!("Time: {:?}", ntp_result);

                // 将NTP时间转换为UTC DateTime
                let unix_timestamp = ntp_result.sec() as i64 - 2_208_988_800; // NTP epoch to Unix epoch
                let subsec_nanos = (u64::from(ntp_result.sec_fraction()) * 1_000_000_000
                    / u64::from(u32::MAX)) as u32;

                let utc = DateTime::from_timestamp(unix_timestamp, subsec_nanos)
                    .ok_or(AppError::TimeError)?;

                Ok(utc)
            }
            Err(e) => {
                log::error!("NTP request failed: {:?}", e);
                return Err(AppError::TimeError);
            }
        }
    }
}

// 默认NTP时间源选择
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub type DefaultNtpTimeSource = SimulatedNtp;
