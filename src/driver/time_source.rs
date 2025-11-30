// src/driver/time_source.rs
use core::net::{IpAddr, SocketAddr};

#[cfg(feature = "embedded_esp")]
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
#[cfg(feature = "embedded_esp")]
use embassy_sync::mutex::Mutex;
use embassy_time::Instant;
#[cfg(feature = "embedded_esp")]
use esp_hal::{rtc_cntl::Rtc, timer::SystemTimer};
use jiff::Timestamp;
use sntpc::{NtpContext, NtpTimestampGenerator};

use crate::common::error::{AppError, Result};
#[cfg(feature = "embedded_esp")]
use crate::driver::network::NetworkDriver;

pub trait TimeSource {
    /// 获取当前时间
    async fn get_time(&self) -> Result<Timestamp>;

    /// 通过SNTP更新时间
    async fn update_time_by_sntp(&mut self) -> Result<()>;

    /// 获取时间戳（64位微秒，使用ESP32 RTC精度）
    fn get_timestamp_us(&self) -> Result<u64>;
}

/// ESP32 RTC时间源 - 使用硬件RTC
#[cfg(feature = "embedded_esp")]
pub struct RtcTimeSource {
    // ESP32 RTC实例
    rtc: Rtc,
    // 系统定时器用于高精度计时
    systimer: SystemTimer,
    // NTP时间源
    ntp_time_source: EspNtp,
    // 基础时间戳（RTC时间）
    base_timestamp_us: u64,
    // 是否已同步
    synchronized: bool,
}

#[cfg(feature = "embedded_esp")]
impl RtcTimeSource {
    pub fn new(rtc: Rtc, systimer: SystemTimer, ntp_time_source: EspNtp) -> Self {
        log::info!("Initializing RtcTimeSource with hardware RTC");

        // 从RTC获取当前时间
        let base_timestamp_us = Self::read_rtc_time_us(&rtc, &systimer);

        Self {
            rtc,
            systimer,
            ntp_time_source,
            base_timestamp_us,
            synchronized: false,
        }
    }

    /// 从ESP32 RTC读取时间（微秒）
    fn read_rtc_time_us(rtc: &Rtc, systimer: &SystemTimer) -> u64 {
        // 使用RTC的慢速时钟计数器
        // RTC时钟通常是150kHz或32kHz，需要转换为微秒
        let rtc_count = rtc.get_count();

        // 根据RTC时钟频率转换为微秒
        // ESP32 RTC通常运行在150kHz (SLOW_CLK)
        let rtc_frequency = 150_000; // 150kHz
        let microseconds = (rtc_count * 1_000_000) / rtc_frequency;

        log::debug!(
            "RTC count: {}, converted to us: {}",
            rtc_count,
            microseconds
        );
        microseconds
    }

    /// 写入时间到ESP32 RTC（微秒）
    fn write_rtc_time_us(&mut self, timestamp_us: u64) -> Result<()> {
        log::info!("Writing time to RTC: {} us", timestamp_us);

        // 将微秒转换为RTC计数
        let rtc_frequency = 150_000; // 150kHz
        let rtc_count = (timestamp_us * rtc_frequency) / 1_000_000;

        // 设置RTC计数器
        self.rtc.set_count(rtc_count);

        // 验证写入
        let read_back = Self::read_rtc_time_us(&self.rtc, &self.systimer);
        if (read_back as i64 - timestamp_us as i64).abs() > 1000 {
            log::error!(
                "RTC time write verification failed: wrote {}, read back {}",
                timestamp_us,
                read_back
            );
            return Err(AppError::TimeError);
        }

        self.base_timestamp_us = timestamp_us;
        self.synchronized = true;
        log::info!("RTC time updated successfully");
        Ok(())
    }

    /// 获取当前RTC时间（考虑RTC漂移补偿）
    fn get_current_rtc_time_us(&self) -> u64 {
        Self::read_rtc_time_us(&self.rtc, &self.systimer)
    }
}

#[cfg(feature = "embedded_esp")]
impl TimeSource for RtcTimeSource {
    async fn get_time(&self) -> Result<Timestamp> {
        let timestamp_us = self.get_timestamp_us()?;
        log::debug!("Getting time from RTC timestamp: {timestamp_us} us");

        // 从微秒时间戳创建Timestamp
        let timestamp = Timestamp::from_micros(timestamp_us);
        log::debug!(
            "Current RTC time: {}",
            timestamp.to_utc().format("%Y-%m-%d %H:%M:%S")
        );
        Ok(timestamp)
    }

    async fn update_time_by_sntp(&mut self) -> Result<()> {
        log::info!("Starting SNTP time update for ESP32 RTC");

        match self.ntp_time_source.get_ntp_time().await {
            Ok(ntp_time) => {
                log::info!(
                    "SNTP time received: {}",
                    ntp_time.to_utc().format("%Y-%m-%d %H:%M:%S")
                );

                // 将NTP时间转换为微秒时间戳
                let timestamp_us = ntp_time.as_micros();

                // 写入到硬件RTC
                self.write_rtc_time_us(timestamp_us)?;
                log::info!("RTC time synchronized with SNTP");
                Ok(())
            }
            Err(e) => {
                log::error!("SNTP time update failed: {:?}", e);
                Err(AppError::TimeError)
            }
        }
    }

    fn get_timestamp_us(&self) -> Result<u64> {
        let current_timestamp = self.get_current_rtc_time_us();

        log::debug!(
            "Getting current RTC timestamp: {current_timestamp} us (synchronized: {})",
            self.synchronized
        );

        Ok(current_timestamp)
    }
}

// ESP32 NTP时间源
#[cfg(feature = "embedded_esp")]
pub struct EspNtp {
    network_driver: &'static Mutex<CriticalSectionRawMutex, NetworkDriver>,
}

#[cfg(feature = "embedded_esp")]
impl EspNtp {
    pub fn new(network_driver: &'static Mutex<CriticalSectionRawMutex, NetworkDriver>) -> Self {
        log::info!("Creating new EspNtp for ESP32");
        Self { network_driver }
    }

    /// 创建SNTP上下文
    fn create_ntp_context(&self) -> NtpContext<EmbassyTimestampGenerator> {
        log::debug!("Creating NTP context for ESP32");
        let timestamp_gen = EmbassyTimestampGenerator::new();
        NtpContext::new(timestamp_gen)
    }
}

#[cfg(feature = "embedded_esp")]
impl NtpTimeSource for EspNtp {
    async fn get_ntp_time(&self) -> Result<Timestamp> {
        use sntpc::get_time;

        log::info!("Starting NTP time request on ESP32");
        let context = self.create_ntp_context();

        // Create UDP socket
        log::debug!("Creating UDP socket for NTP on ESP32");
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

            let ntp_addrs = match self
                .network_driver
                .lock()
                .await
                .stack
                .dns_query(server, DnsQueryType::A)
                .await
            {
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
                        Timestamp::from_secs_and_nanos(unix_timestamp as u64, subsec_nanos);

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

// 默认时间源选择
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub type DefaultTimeSource = SimulatedRtc;

#[cfg(feature = "embedded_esp")]
pub type DefaultTimeSource = RtcTimeSource;

// 默认NTP时间源选择
#[cfg(any(feature = "simulator", feature = "embedded_linux"))]
pub type DefaultNtpTimeSource = SimulatedNtp;

#[cfg(feature = "embedded_esp")]
pub type DefaultNtpTimeSource = EspNtp;
