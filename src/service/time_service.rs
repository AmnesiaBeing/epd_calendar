// src/driver/time_source.rs
use core::net::{IpAddr, SocketAddr};

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
use core::sync::atomic::{AtomicU64, Ordering};
#[cfg(feature = "embedded_esp")]
use embassy_net::dns::DnsQueryType;
use embassy_net::udp::{PacketMetadata, UdpSocket};
#[cfg(feature = "embedded_esp")]
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
#[cfg(any(
    feature = "simulator",
    feature = "embedded_linux",
    feature = "embedded_esp"
))]
use embassy_sync::mutex::Mutex;
use embassy_time::Instant;
#[cfg(feature = "embedded_esp")]
use esp_hal::rtc_cntl::Rtc;
use jiff::Timestamp;
use sntpc::{NtpContext, NtpTimestampGenerator};

use crate::common::error::{AppError, Result};
#[cfg(any(
    feature = "simulator",
    feature = "embedded_linux",
    feature = "embedded_esp"
))]
use crate::driver::network::NetworkDriver;

pub trait TimeSource {
    /// 获取当前时间
    async fn get_time(&self) -> Result<Timestamp>;

    /// 通过SNTP更新时间
    async fn update_time_by_sntp(&mut self) -> Result<()>;

    /// 获取时间戳（64位微秒）
    fn get_timestamp_us(&self) -> Result<u64>;
}

/// 通用时间源实现
pub struct CommonTimeSource<T: TimeBackend> {
    backend: T,
    ntp_time_source: DefaultNtpTimeSource<T>,
    synchronized: bool,
}

impl<T: TimeBackend> CommonTimeSource<T> {
    pub fn new(backend: T, ntp_time_source: DefaultNtpTimeSource) -> Self {
        log::info!("Initializing CommonTimeSource");
        Self {
            backend,
            ntp_time_source,
            synchronized: false,
        }
    }
}

impl<T: TimeBackend> TimeSource for CommonTimeSource<T> {
    async fn get_time(&self) -> Result<Timestamp> {
        let timestamp_us = self.get_timestamp_us()?;
        log::debug!("Getting time from timestamp: {timestamp_us} us");

        // 使用jiff的正确API
        let timestamp = Timestamp::from_microsecond(timestamp_us as i64).map_err(|_| {
            log::error!(
                "Failed to create Timestamp from microseconds: {}",
                timestamp_us
            );
            AppError::TimeError
        })?;

        log::debug!(
            "Current time: {}",
            timestamp.to_utc().format("%Y-%m-%d %H:%M:%S")
        );
        Ok(timestamp)
    }

    async fn update_time_by_sntp(&mut self) -> Result<()> {
        log::info!("Starting SNTP time update");

        match self.ntp_time_source.get_ntp_time().await {
            Ok(ntp_time) => {
                log::info!(
                    "SNTP time received: {}",
                    ntp_time.to_utc().format("%Y-%m-%d %H:%M:%S")
                );

                // 将NTP时间转换为微秒时间戳
                let timestamp_us = ntp_time.as_micros();

                // 更新后端时间
                self.backend.set_time_us(timestamp_us)?;
                self.synchronized = true;
                log::info!("SNTP time update successful");
                Ok(())
            }
            Err(e) => {
                log::error!("SNTP time update failed: {:?}", e);
                Err(AppError::TimeError)
            }
        }
    }

    fn get_timestamp_us(&self) -> Result<u64> {
        let current_timestamp = self.backend.get_time_us();
        log::debug!(
            "Getting current timestamp: {current_timestamp} us (synchronized: {})",
            self.synchronized
        );
        Ok(current_timestamp)
    }
}

/// 时间后端trait - 抽象不同平台的时间获取方式
pub trait TimeBackend {
    /// 获取当前时间（微秒）
    fn get_time_us(&self) -> u64;

    /// 设置时间（微秒）
    fn set_time_us(&mut self, timestamp_us: u64) -> Result<()>;
}

/// 模拟器时间后端
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub struct SimulatedTimeBackend {
    timestamp_us: AtomicU64,
    start_time: Instant,
}

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
impl SimulatedTimeBackend {
    pub fn new() -> Self {
        // 使用当前系统时间初始化
        let now = Timestamp::now();
        let timestamp_us = now.as_micros();
        log::info!("SimulatedTimeBackend initialized with timestamp: {timestamp_us} us");

        Self {
            timestamp_us: AtomicU64::new(timestamp_us),
            start_time: Instant::now(),
        }
    }
}

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
impl TimeBackend for SimulatedTimeBackend {
    fn get_time_us(&self) -> u64 {
        let elapsed = self.start_time.elapsed().as_micros() as u64;
        let base_timestamp = self.timestamp_us.load(Ordering::Acquire);
        base_timestamp + elapsed
    }

    fn set_time_us(&mut self, timestamp_us: u64) -> Result<()> {
        log::debug!(
            "Updating simulated time from {} to {} us",
            self.timestamp_us.load(Ordering::Acquire),
            timestamp_us
        );
        self.timestamp_us.store(timestamp_us, Ordering::Release);
        self.start_time = Instant::now();
        Ok(())
    }
}

/// ESP32 RTC时间后端
#[cfg(feature = "embedded_esp")]
pub struct EspRtcTimeBackend<'d> {
    rtc: Rtc<'d>,
}

#[cfg(feature = "embedded_esp")]
impl<'d> EspRtcTimeBackend<'d> {
    pub fn new(rtc: Rtc<'d>) -> Self {
        log::info!("Initializing EspRtcTimeBackend with hardware RTC");
        Self { rtc }
    }
}

#[cfg(feature = "embedded_esp")]
impl<'d> TimeBackend for EspRtcTimeBackend<'d> {
    fn get_time_us(&self) -> u64 {
        self.rtc.current_time_us()
    }

    fn set_time_us(&mut self, timestamp_us: u64) -> Result<()> {
        log::info!("Setting RTC time: {} us", timestamp_us);

        // 使用ESP-HAL提供的设置当前时间的方法
        self.rtc.set_current_time_us(timestamp_us);

        // 验证设置
        let current_time = self.get_time_us();
        let time_diff = (current_time as i64 - timestamp_us as i64).abs();

        if time_diff > 1000 {
            log::error!(
                "RTC time set verification failed: expected {}, got {}, diff: {}",
                timestamp_us,
                current_time,
                time_diff
            );
            return Err(AppError::TimeError);
        }

        log::info!("RTC time set successfully");
        Ok(())
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

pub trait NtpTimeSource {
    /// 获取NTP时间（返回UTC时间）
    async fn get_ntp_time(&self) -> Result<Timestamp>;
}

/// 通用NTP时间源
pub struct CommonNtpTimeSource<T: NetworkStack> {
    network_driver: T,
}

impl<T: NetworkStack> CommonNtpTimeSource<T> {
    pub fn new(network_driver: T) -> Self {
        log::info!("Creating new CommonNtpTimeSource");
        Self { network_driver }
    }

    /// 创建SNTP上下文
    fn create_ntp_context(&self) -> NtpContext<EmbassyTimestampGenerator> {
        log::debug!("Creating NTP context");
        let timestamp_gen = EmbassyTimestampGenerator::new();
        NtpContext::new(timestamp_gen)
    }
}

impl<T: NetworkStack> NtpTimeSource for CommonNtpTimeSource<T> {
    async fn get_ntp_time(&self) -> Result<Timestamp> {
        use sntpc::get_time;

        log::info!("Starting NTP time request");
        let context = self.create_ntp_context();

        // Create UDP socket
        log::debug!("Creating UDP socket for NTP");
        let mut rx_meta = [PacketMetadata::EMPTY; 16];
        let mut rx_buffer = [0; 4096];
        let mut tx_meta = [PacketMetadata::EMPTY; 16];
        let mut tx_buffer = [0; 4096];

        let mut socket = UdpSocket::new(
            self.network_driver.get_stack(),
            &mut rx_meta,
            &mut rx_buffer,
            &mut tx_meta,
            &mut tx_buffer,
        );

        // 使用临时端口
        if let Err(e) = socket.bind(0) {
            log::error!("Failed to bind UDP socket: {:?}", e);
            return Err(AppError::NetworkError);
        }
        log::debug!("UDP socket bound to ephemeral port");

        // 使用多个NTP服务器以提高可靠性
        let ntp_servers = [
            "pool.ntp.org",
            "time.google.com",
            "time.windows.com",
            "ntp.ntsc.ac.cn",
        ];

        let mut last_error = None;

        for server in ntp_servers.iter() {
            log::info!("Resolving NTP server: {server}");

            let ntp_addrs = match self.network_driver.dns_query(server, DnsQueryType::A).await {
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
            let result = get_time(SocketAddr::from((addr, 123)), &socket, context).await;

            match result {
                Ok(ntp_result) => {
                    log::info!("NTP request to {server} success");

                    // 将NTP时间转换为Timestamp
                    let ntp_seconds = ntp_result.sec();

                    // 直接将秒数转换为Unix时间戳
                    let unix_timestamp = ntp_seconds as i64;

                    // 计算纳秒部分
                    let subsec_nanos = (u64::from(ntp_result.sec_fraction()) * 1_000_000_000
                        / u64::from(u32::MAX)) as u32;

                    // 使用jiff创建Timestamp
                    let timestamp =
                        Timestamp::from_secs_and_nanos(unix_timestamp as u64, subsec_nanos)
                            .map_err(|_| {
                                log::error!("Failed to create Timestamp from NTP result");
                                AppError::TimeError
                            })?;

                    log::info!(
                        "NTP time from {server}: {}",
                        timestamp.to_utc().format("%Y-%m-%d %H:%M:%S")
                    );
                    return Ok(timestamp);
                }
                Err(e) => {
                    log::warn!("NTP request to {server} failed: {:?}", e);
                    last_error = Some(AppError::TimeError);
                    continue;
                }
            }
        }

        log::error!("All NTP servers failed");
        Err(last_error.unwrap_or(AppError::TimeError))
    }
}

#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub type DefaultTimeSource = CommonTimeSource<SimulatedTimeBackend>;

#[cfg(feature = "embedded_esp")]
pub type DefaultTimeSource<'d> = CommonTimeSource<EspRtcTimeBackend<'d>>;

#[cfg(any(
    feature = "simulator",
    feature = "embedded_linux",
    feature = "embedded_esp"
))]
pub type DefaultNtpTimeSource<T> = CommonNtpTimeSource<T>;
