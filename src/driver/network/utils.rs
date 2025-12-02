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
    let seed = getrandom::u64().map_err(|_| AppError::NetworkStackInitFailed)?;

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
