// src/kernel/system/api.rs
//! 系统API接口模块
//! 定义系统级别的模块化API接口，包含硬件、网络和配置存储子接口

#![allow(unused)]

use crate::common::GlobalMutex;
use crate::common::error::{AppError, Result};
use crate::kernel::data::{DataSourceRegistry, DynamicValue};
use crate::kernel::driver::buzzer::{DefaultBuzzerDriver, BuzzerDriver};
use crate::kernel::driver::display::DefaultDisplayDriver;
use crate::kernel::driver::led::{DefaultLedDriver, LedDriver, LedState};
use crate::kernel::driver::network::{DefaultNetworkDriver, NetworkDriver};
use crate::kernel::driver::power::{BatteryLevel, DefaultPowerDriver, PowerDriver};
use crate::kernel::driver::sensor::{DefaultSensorDriver, SensorDriver};
use crate::kernel::driver::storage::{ConfigStorage, DefaultConfigStorageDriver};

use alloc::boxed::Box;
use async_trait::async_trait;
use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use heapless::{String, Vec};
use reqwless::client::HttpClient;
use reqwless::request::RequestBuilder;

/// 硬件API接口
/// 处理硬件相关操作：电池/充电状态、系统时间戳/tick、WiFi连接/状态等
#[async_trait(?Send)]
pub trait HardwareApi {
    /// 获取电池电量
    async fn get_battery_level(&self) -> Result<BatteryLevel>;

    /// 获取充电状态
    async fn is_charging(&self) -> Result<bool>;

    /// 获取湿度
    async fn get_humidity(&self) -> Result<i32>;

    /// 获取温度
    async fn get_temperature(&self) -> Result<i32>;

    /// 检查WiFi连接状态
    async fn is_wifi_connected(&self) -> Result<bool>;

    /// 连接到WiFi
    async fn connect_wifi(&self, ssid: &str, password: &str) -> Result<()>;

    /// 断开WiFi连接
    async fn disconnect_wifi(&self) -> Result<()>;

    /// 启动WiFi配对模式
    async fn start_wifi_pairing(&self) -> Result<()>;

    /// 停止WiFi配对模式
    async fn stop_wifi_pairing(&self) -> Result<()>;

    /// 刷新屏幕显示
    async fn update_screen(&self) -> Result<()>;

    /// 设置LED状态
    async fn set_led_state(&self, status: LedState) -> Result<()>;

    /// 播放蜂鸣器音调
    async fn play_tone(&self, frequency: u32, duration: u32) -> Result<()>;

    /// 播放音乐
    async fn play_music(&self, music_id: u8) -> Result<()>;

    /// 停止蜂鸣器
    async fn stop_buzzer(&self) -> Result<()>;
}

/// 网络客户端API接口
/// 专做网络请求：HTTP GET/POST，纯数据传输，不涉及硬件控制
#[async_trait(?Send)]
pub trait NetworkClientApi {
    /// 发送HTTP GET请求
    async fn http_get(&self, url: &str) -> Result<String<256>>;

    /// 发送HTTPS GET请求
    async fn https_get(&self, url: &str) -> Result<String<256>>;
}

/// 配置存储API接口
/// 底层配置存储：仅做配置的原始读写（无默认值、无缓存），对接存储驱动
#[async_trait(?Send)]
pub trait ConfigApi {
    /// 读取配置数据
    async fn read_config(&self) -> Result<Option<Vec<u8, 1024>>>;

    /// 写入配置数据
    async fn write_config(&self, data: &[u8]) -> Result<()>;
}

/// 系统API接口
/// 聚合接口：提供子接口的统一访问入口，无独立逻辑
#[async_trait(?Send)]
pub trait SystemApi {
    type HardwareApi: HardwareApi;
    type NetworkClientApi: NetworkClientApi;
    type ConfigApi: ConfigApi;

    /// 获取硬件API实例
    fn get_hardware_api(&self) -> &Self::HardwareApi;

    /// 获取网络客户端API实例
    fn get_network_client_api(&self) -> &Self::NetworkClientApi;

    /// 获取配置存储API实例
    fn get_config_api(&self) -> &Self::ConfigApi;

    /// 通过字符串路径获取数据
    /// 路径格式：数据源名称.字段名称，例如："config.wifi_ssid"、"datetime.date.year"
    async fn get_data_by_path(&self, path: &str) -> Result<DynamicValue>;
}

/// 默认系统API实现
pub struct DefaultSystemApi {
    /// 电源驱动实例
    power_driver: GlobalMutex<DefaultPowerDriver>,
    /// 网络驱动实例（全局互斥锁保护）
    network_driver: &'static GlobalMutex<DefaultNetworkDriver>,
    /// 配置存储驱动实例
    config_storage_driver: GlobalMutex<DefaultConfigStorageDriver>,
    /// 传感器驱动实例
    sensor_driver: GlobalMutex<DefaultSensorDriver>,
    /// 执行器驱动实例
    led_driver: GlobalMutex<DefaultLedDriver>,
    /// 蜂鸣器驱动实例
    buzzer_driver: &'static GlobalMutex<DefaultBuzzerDriver>,
    /// 数据源注册表（全局互斥锁保护）
    data_source_registry: Option<&'static GlobalMutex<DataSourceRegistry>>,
    /// 屏幕驱动实例
    display_driver: &'static GlobalMutex<DefaultDisplayDriver>,
}

impl DefaultSystemApi {
    /// 创建新的默认系统API实例
    pub fn new(
        power_driver: DefaultPowerDriver,
        network_driver: &'static GlobalMutex<DefaultNetworkDriver>,
        storage_driver: DefaultConfigStorageDriver,
        sensor_driver: DefaultSensorDriver,
        led_driver: DefaultLedDriver,
        buzzer_driver: &'static GlobalMutex<DefaultBuzzerDriver>,
        display_driver: &'static GlobalMutex<DefaultDisplayDriver>,
    ) -> Self {
        Self {
            power_driver: GlobalMutex::new(power_driver),
            network_driver,
            config_storage_driver: GlobalMutex::new(storage_driver),
            sensor_driver: GlobalMutex::new(sensor_driver),
            led_driver: GlobalMutex::new(led_driver),
            buzzer_driver: buzzer_driver,
            data_source_registry: None,
            display_driver,
        }
    }

    /// 设置数据源注册表
    pub fn set_data_source_registry(
        &mut self,
        data_source_registry: &'static GlobalMutex<DataSourceRegistry>,
    ) {
        self.data_source_registry = Some(data_source_registry);
    }
}

#[async_trait(?Send)]
impl SystemApi for DefaultSystemApi {
    type HardwareApi = DefaultSystemApi;
    type NetworkClientApi = DefaultSystemApi;
    type ConfigApi = DefaultSystemApi;

    /// 获取硬件API实例
    fn get_hardware_api(&self) -> &Self::HardwareApi {
        // 返回硬件API实现
        self
    }

    /// 获取网络客户端API实例
    fn get_network_client_api(&self) -> &Self::NetworkClientApi {
        // 返回网络客户端API实现
        self
    }

    /// 获取配置存储API实例
    fn get_config_api(&self) -> &Self::ConfigApi {
        // 返回配置存储API实现
        self
    }

    async fn get_data_by_path(
        &self,
        path: &str,
    ) -> Result<crate::kernel::data::types::DynamicValue> {
        match &self.data_source_registry {
            Some(registry) => {
                let registry_guard = registry.lock().await;
                registry_guard.get_value_by_path_async(path).await
            }
            None => Err(AppError::DataSourceNotFound),
        }
    }
}

// 硬件API实现
#[async_trait(?Send)]
impl HardwareApi for DefaultSystemApi {
    async fn get_battery_level(&self) -> Result<BatteryLevel> {
        self.power_driver
            .lock()
            .await
            .battery_level()
            .await
            .map_err(|_| AppError::PowerError)
    }

    async fn is_charging(&self) -> Result<bool> {
        self.power_driver
            .lock()
            .await
            .is_charging()
            .await
            .map_err(|_| AppError::PowerError)
    }

    /// 获取湿度
    async fn get_humidity(&self) -> Result<i32> {
        self.sensor_driver
            .lock()
            .await
            .get_humidity()
            .await
            .map_err(|_| AppError::SensorError)
    }

    /// 获取温度
    async fn get_temperature(&self) -> Result<i32> {
        self.sensor_driver
            .lock()
            .await
            .get_temperature()
            .await
            .map_err(|_| AppError::SensorError)
    }

    async fn is_wifi_connected(&self) -> Result<bool> {
        // 检查WiFi连接状态
        Ok(self.network_driver.lock().await.is_connected())
    }

    async fn connect_wifi(&self, ssid: &str, password: &str) -> Result<()> {
        // 保存WiFi凭据到配置
        // let mut config = crate::kernel::data::sources::config::SystemConfig::get_instance().await;
        // config.set("wifi_ssid", ssid.to_string()).await?;
        // config.set("wifi_password", password.to_string()).await?;
        // config.save().await?;

        // 连接WiFi
        let mut network_driver = self.network_driver.lock().await;
        network_driver.connect(ssid, password).await
    }

    async fn disconnect_wifi(&self) -> Result<()> {
        // 断开WiFi连接
        let mut network_driver = self.network_driver.lock().await;
        network_driver.disconnect().await
    }

    /// 启动WiFi配对模式
    async fn start_wifi_pairing(&self) -> Result<()> {
        let mut network_driver = self.network_driver.lock().await;
        network_driver.start_ap("EPD_Calendar", None).await
    }

    /// 停止WiFi配对模式
    async fn stop_wifi_pairing(&self) -> Result<()> {
        let mut network_driver = self.network_driver.lock().await;
        network_driver.stop_ap().await
    }

    /// 刷新屏幕显示
    async fn update_screen(&self) -> Result<()> {
        // 调用渲染任务更新屏幕
        // let display_driver = self.display_driver.lock().await;
        // let display_buffer = self.display_buffer.lock().await;
        // let data_source_registry = self.data_source_registry.lock().await;

        // crate::tasks::main_task::render_layout(
        //     &display_driver,
        //     &display_buffer,
        //     &data_source_registry,
        // )
        // .await;
        // Ok(())
        unimplemented!()
    }

    /// 设置LED状态
    async fn set_led_state(&self, state: LedState) -> Result<()> {
        self.led_driver.lock().await.set_led_state(state)
    }

    /// 播放蜂鸣器音调
    async fn play_tone(&self, frequency: u32, duration: u32) -> Result<()> {
        self.buzzer_driver.lock().await.tone(frequency, duration).await
    }

    /// 播放音乐
    async fn play_music(&self, music_id: u8) -> Result<()> {
        self.buzzer_driver.lock().await.play_music(music_id).await
    }

    /// 停止蜂鸣器
    async fn stop_buzzer(&self) -> Result<()> {
        self.buzzer_driver.lock().await.stop().await
    }
}

// 网络客户端API实现
#[async_trait(?Send)]
impl NetworkClientApi for DefaultSystemApi {
    async fn http_get(&self, url: &str) -> Result<String<256>> {
        // 锁定网络驱动以获取栈引用
        let network_guard = self.network_driver.lock().await;
        let stack = network_guard.get_stack().ok_or(AppError::NetworkError)?;

        // 创建TcpClientState并绑定到变量，确保其生命周期足够长
        let state = TcpClientState::<1, 4096, 4096>::new();
        let mut tcp_client = TcpClient::new(*stack, &state);
        let dns_socket = DnsSocket::new(*stack);

        // 创建不带TLS的HTTP客户端
        let mut client = HttpClient::new(&tcp_client, &dns_socket);

        let mut buffer: [u8; 4096] = [0; 4096];

        // 从URL中提取主机名作为Host头部
        let host = extract_host(url);

        // 将headers数组绑定到变量，确保其生命周期足够长
        let headers = [("Host", host)];

        // 分解请求构建步骤，避免临时值生命周期问题
        let mut request_builder = client
            .request(reqwless::request::Method::GET, url)
            .await
            .map_err(|_| AppError::NetworkRequestFailed)?;

        request_builder = request_builder.content_type(reqwless::headers::ContentType::TextPlain);

        request_builder = request_builder.headers(&headers);

        let request = request_builder
            .send(&mut buffer)
            .await
            .map_err(|_| AppError::NetworkRequestFailed)?;

        // 读取响应状态码
        let status = request.status;
        if !status.is_successful() {
            return Err(AppError::NetworkRequestFailed);
        }

        // 读取响应体
        let response_body = request.body();

        // 将响应体转换为String<256>
        let mut result = String::<256>::new();
        let body = response_body
            .read_to_end()
            .await
            .map_err(|_| AppError::NetworkRequestFailed)?;

        // 将body内容转换为字符串并复制到result中
        if let Ok(body_str) = core::str::from_utf8(body) {
            result
                .push_str(body_str)
                .map_err(|_| AppError::NetworkResponseTooLarge)?;
        } else {
            return Err(AppError::NetworkResponseInvalid);
        }

        Ok(result)
    }

    async fn https_get(&self, url: &str) -> Result<String<256>> {
        // 锁定网络驱动以获取栈引用
        let network_guard = self.network_driver.lock().await;
        let stack = network_guard.get_stack().ok_or(AppError::NetworkError)?;

        // 创建TcpClientState并绑定到变量，确保其生命周期足够长
        let state = TcpClientState::<1, 4096, 4096>::new();
        let mut tcp_client = TcpClient::new(*stack, &state);
        let dns_socket = DnsSocket::new(*stack);

        let seed = getrandom::u64().map_err(|_| AppError::NetworkStackInitFailed)?;

        let mut rx_buffer: [u8; 4096] = [0; 4096];
        let mut tx_buffer: [u8; 4096] = [0; 4096];

        let config = reqwless::client::TlsConfig::new(
            seed,
            &mut rx_buffer,
            &mut tx_buffer,
            reqwless::client::TlsVerify::None,
        );

        let mut client = HttpClient::new_with_tls(&tcp_client, &dns_socket, config);

        let mut buffer: [u8; 4096] = [0; 4096];

        // 从URL中提取主机名作为Host头部
        let host = extract_host(url);

        // 将headers数组绑定到变量，确保其生命周期足够长
        let headers = [("Host", host)];

        // 分解请求构建步骤，避免临时值生命周期问题
        let mut request_builder = client
            .request(reqwless::request::Method::GET, url)
            .await
            .map_err(|_| AppError::NetworkRequestFailed)?;

        request_builder = request_builder.content_type(reqwless::headers::ContentType::TextPlain);

        request_builder = request_builder.headers(&headers);

        let request = request_builder
            .send(&mut buffer)
            .await
            .map_err(|_| AppError::NetworkRequestFailed)?;

        // 读取响应状态码
        let status = request.status;
        if !status.is_successful() {
            return Err(AppError::NetworkRequestFailed);
        }

        // 读取响应体
        let response_body = request.body();

        // 将响应体转换为String<256>
        let mut result = String::<256>::new();
        let body = response_body
            .read_to_end()
            .await
            .map_err(|_| AppError::NetworkRequestFailed)?;

        // 将body内容转换为字符串并复制到result中
        if let Ok(body_str) = core::str::from_utf8(body) {
            result
                .push_str(body_str)
                .map_err(|_| AppError::NetworkResponseTooLarge)?;
        } else {
            return Err(AppError::NetworkResponseInvalid);
        }

        Ok(result)
    }
}

// 辅助函数：从URL中提取主机名
fn extract_host(url: &str) -> &str {
    // 跳过协议部分
    let url = if let Some(stripped) = url.strip_prefix("http://") {
        stripped
    } else if let Some(stripped) = url.strip_prefix("https://") {
        stripped
    } else {
        url
    };

    // 提取主机名（到第一个/或:为止）
    if let Some(pos) = url.find(|c| ['/', ':'].contains(&c)) {
        &url[..pos]
    } else {
        url
    }
}

// 配置存储API实现
#[async_trait(?Send)]
impl ConfigApi for DefaultSystemApi {
    async fn read_config(&self) -> Result<Option<heapless::Vec<u8, 1024>>> {
        // 读取配置数据
        match self
            .config_storage_driver
            .lock()
            .await
            .read_config_block()?
        {
            Some(data) => {
                let mut result = heapless::Vec::new();
                result
                    .extend_from_slice(&data)
                    .map_err(|_| AppError::ConfigTooLarge)?;
                Ok(Some(result))
            }
            None => Ok(None),
        }
    }

    async fn write_config(&self, data: &[u8]) -> Result<()> {
        // 写入配置数据
        self.config_storage_driver
            .lock()
            .await
            .write_config_block(data)
    }
}
