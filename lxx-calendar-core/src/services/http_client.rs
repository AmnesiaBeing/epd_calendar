use core::fmt::{Debug, Write};
use embassy_net::Stack;
use embassy_net::dns::DnsQueryType;
use embassy_net::tcp::TcpSocket;
use embedded_io_async::Write as IoWrite;
use heapless::String;
use lxx_calendar_common::http::http::{HttpClient, HttpMethod, HttpRequest, HttpResponse};
use lxx_calendar_common::{error, info};

const RX_BUFFER_SIZE: usize = 4096;
const TX_BUFFER_SIZE: usize = 4096;
const TLS_RX_BUFFER_SIZE: usize = 16384;
const TLS_TX_BUFFER_SIZE: usize = 16384;

pub struct HttpClientImpl<'a> {
    stack: Stack<'static>,
    tls_rx_buf: &'a mut [u8],
    tls_tx_buf: &'a mut [u8],
}

impl<'a> Debug for HttpClientImpl<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "HttpClientImpl")
    }
}

impl<'a> HttpClientImpl<'a> {
    pub fn new(stack: Stack<'static>, tls_rx_buf: &'a mut [u8], tls_tx_buf: &'a mut [u8]) -> Self {
        Self {
            stack,
            tls_rx_buf,
            tls_tx_buf,
        }
    }

    async fn request_inner(
        &mut self,
        method: HttpMethod,
        url: &str,
        body: Option<&[u8]>,
        headers: Option<&[(&str, &str)]>,
    ) -> Result<(u16, heapless::Vec<u8, 16384>), HttpError> {
        let (_scheme, _host, _port, _path) = parse_full_url(url)?;
        let is_https = _scheme == "https";

        info!("HTTP: Resolving DNS for {}", _host);
        let ip_addrs = self
            .stack
            .dns_query(_host, DnsQueryType::A)
            .await
            .map_err(|e| {
                log::error!("HTTP: DNS query failed for {}: {:?}", _host, e);
                HttpError::DnsFailed
            })?;

        let ip = ip_addrs.first().ok_or(HttpError::DnsFailed)?;
        info!("HTTP: DNS resolved {} to {}", _host, ip);

        // 分配 TCP 缓冲区
        let mut rx_buf = [0u8; RX_BUFFER_SIZE];
        let mut tx_buf = [0u8; TX_BUFFER_SIZE];

        // 创建 TCP socket
        let mut socket = TcpSocket::new(self.stack, &mut rx_buf, &mut tx_buf);

        info!("HTTP: Connecting to {}:{} (HTTPS={})", ip, _port, is_https);

        socket.connect((*ip, _port)).await.map_err(|e| {
            log::error!("HTTP: Connection failed to {}:{}: {:?}", ip, _port, e);
            HttpError::ConnectionFailed
        })?;

        info!("HTTP: Connected to {}:{}", ip, _port);

        // 发送 HTTP 请求
        self.send_request(socket, method, url, body, headers).await
    }

    async fn send_request<S>(
        &mut self,
        socket: S,
        method: HttpMethod,
        url: &str,
        body: Option<&[u8]>,
        _headers: Option<&[(&str, &str)]>,
    ) -> Result<(u16, heapless::Vec<u8, 16384>), HttpError>
    where
        S: embedded_io_async::Read + embedded_io_async::Write,
    {
        // 解析 URL
        let (_scheme, _host, _port, path) = parse_full_url(url)?;

        // 构建 HTTP 请求
        let method_str = method_to_str(method);
        let mut request = String::<512>::new();

        // 请求行（只使用路径部分）
        core::write!(request, "{} {} HTTP/1.1\r\n", method_str, path)
            .map_err(|_| HttpError::RequestFailed)?;

        // Host header (从 URL 提取)
        if let Some(host) = extract_host(url) {
            core::write!(request, "Host: {}\r\n", host).map_err(|_| HttpError::RequestFailed)?;
        }

        // Connection: close
        core::write!(request, "Connection: close\r\n").map_err(|_| HttpError::RequestFailed)?;

        // Content-Length for body
        if let Some(body_data) = body {
            core::write!(request, "Content-Length: {}\r\n", body_data.len())
                .map_err(|_| HttpError::RequestFailed)?;
        }

        // 空行结束 headers
        core::write!(request, "\r\n").map_err(|_| HttpError::RequestFailed)?;

        info!("HTTP Request: {}", request);

        // 发送请求头
        let mut wrapped_socket = socket;
        wrapped_socket
            .write_all(request.as_bytes())
            .await
            .map_err(|_| HttpError::RequestFailed)?;

        // 发送 body
        if let Some(body_data) = body {
            wrapped_socket
                .write_all(body_data)
                .await
                .map_err(|_| HttpError::RequestFailed)?;
        }

        wrapped_socket
            .flush()
            .await
            .map_err(|_| HttpError::RequestFailed)?;

        // 读取响应
        self.read_response(wrapped_socket).await
    }

    async fn read_response<S>(
        &mut self,
        mut socket: S,
    ) -> Result<(u16, heapless::Vec<u8, 16384>), HttpError>
    where
        S: embedded_io_async::Read + embedded_io_async::Write,
    {
        info!("HTTP: Starting to read response");

        // 读取响应头
        let mut header_buf = [0u8; 4096];
        let mut header_bytes_read = 0;

        // 读取直到 \r\n\r\n
        loop {
            if header_bytes_read >= header_buf.len() {
                error!("HTTP: Header buffer overflow");
                return Err(HttpError::RequestFailed);
            }

            let n = socket
                .read(&mut header_buf[header_bytes_read..header_bytes_read + 1])
                .await
                .map_err(|_e| {
                    error!("HTTP: Failed to read header byte");
                    HttpError::RequestFailed
                })?;

            if n == 0 {
                error!("HTTP: Connection closed while reading header");
                break;
            }

            header_bytes_read += n;

            if header_bytes_read >= 4 && &header_buf[header_bytes_read - 4..] == b"\r\n\r\n" {
                info!("HTTP: Header end found after {} bytes", header_bytes_read);
                break;
            }
        }

        info!("HTTP: Read {} header bytes", header_bytes_read);

        let header_str = core::str::from_utf8(&header_buf[..header_bytes_read]).map_err(|_| {
            error!("HTTP: Invalid UTF-8 in header");
            HttpError::RequestFailed
        })?;

        info!("HTTP Response Headers: {}", header_str);

        // 解析状态行
        let status_line = header_str.lines().next().ok_or(HttpError::RequestFailed)?;
        let status: u16 = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .ok_or(HttpError::RequestFailed)?;

        info!("HTTP: Response status: {}", status);

        // 检查 Content-Length
        let content_length = header_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("content-length:"))
            .and_then(|l| l.split(':').nth(1))
            .and_then(|v| v.trim().parse::<usize>().ok());

        info!("HTTP: Content-Length: {:?}", content_length);

        // 检查 Transfer-Encoding: chunked
        let is_chunked = header_str.contains("Transfer-Encoding: chunked");
        info!("HTTP: Is chunked: {}", is_chunked);

        // 读取响应 body
        let mut body_vec = heapless::Vec::<u8, 16384>::new();
        info!("HTTP: Starting to read body, is_chunked={}", is_chunked);

        if is_chunked {
            // header_buf 中可能已经包含了部分或全部 chunked 数据
            // 找到 \r\n\r\n 的位置，从那里开始是 body 数据
            let mut current_pos = 0usize;
            // 查找 \r\n\r\n 的位置
            for i in 0..header_bytes_read.saturating_sub(3) {
                if &header_buf[i..i + 4] == b"\r\n\r\n" {
                    current_pos = i + 4;
                    info!("HTTP: Body starts at position {}", current_pos);
                    break;
                }
            }

            let header_end_pos = header_bytes_read;
            info!(
                "HTTP: Header buffer has {} bytes total, {} bytes of body data",
                header_end_pos,
                header_end_pos - current_pos
            );

            // 处理 chunked 编码
            loop {
                // 读取 chunk size 行
                let mut size_line_buf = [0u8; 32];
                let mut size_line_bytes = 0;

                loop {
                    if size_line_bytes >= size_line_buf.len() {
                        error!("HTTP: Chunk size buffer overflow");
                        return Err(HttpError::RequestFailed);
                    }

                    // 先从 header_buf 中读取剩余数据（如果有）
                    if current_pos < header_end_pos {
                        let byte = header_buf[current_pos];
                        current_pos += 1;
                        size_line_buf[size_line_bytes] = byte;
                        size_line_bytes += 1;

                        if size_line_bytes >= 2
                            && &size_line_buf[size_line_bytes - 2..size_line_bytes] == b"\r\n"
                        {
                            break;
                        }
                        continue;
                    }

                    // 从 socket 读取
                    let n = socket
                        .read(&mut size_line_buf[size_line_bytes..size_line_bytes + 1])
                        .await;

                    match n {
                        Ok(0) => {
                            // 连接关闭，如果已经读取了数据，可能是最后一个 chunk
                            if size_line_bytes == 0 {
                                info!("HTTP: Connection closed after last chunk");
                                break;
                            }
                            return Err(HttpError::RequestFailed);
                        }
                        Ok(n) => {
                            size_line_bytes += n;
                            if size_line_bytes >= 2
                                && &size_line_buf[size_line_bytes - 2..size_line_bytes] == b"\r\n"
                            {
                                break;
                            }
                        }
                        Err(_) => {
                            error!("HTTP: Failed to read chunk size");
                            return Err(HttpError::RequestFailed);
                        }
                    }
                }

                // 如果 size_line_bytes 为 0，说明连接已关闭
                if size_line_bytes == 0 {
                    info!("HTTP: No more chunk size data");
                    break;
                }

                // 解析 chunk size
                let size_line = core::str::from_utf8(&size_line_buf[..size_line_bytes - 2])
                    .map_err(|_| HttpError::RequestFailed)?;

                let chunk_size = usize::from_str_radix(size_line.trim(), 16).map_err(|_e| {
                    error!("HTTP: Failed to parse chunk size '{}'", size_line);
                    HttpError::RequestFailed
                })?;

                info!("HTTP: Chunk size: {}", chunk_size);

                // Chunk size 0 表示结束
                if chunk_size == 0 {
                    info!("HTTP: Chunked body end");
                    break;
                }

                // 读取 chunk 数据
                let mut bytes_read = 0;
                while bytes_read < chunk_size {
                    let read_size = (chunk_size - bytes_read).min(4096);

                    // 先从 header_buf 中读取（如果有剩余数据）
                    if current_pos < header_end_pos {
                        let available = header_end_pos - current_pos;
                        let to_read = read_size.min(available);
                        body_vec
                            .extend_from_slice(&header_buf[current_pos..current_pos + to_read])
                            .ok();
                        current_pos += to_read;
                        bytes_read += to_read;
                        continue;
                    }

                    // 从 socket 读取
                    let mut chunk_buf = [0u8; 4096];
                    let n = socket
                        .read(&mut chunk_buf[..read_size])
                        .await
                        .map_err(|_| HttpError::RequestFailed)?;

                    if n == 0 {
                        error!("HTTP: Connection closed while reading chunk data");
                        return Err(HttpError::RequestFailed);
                    }

                    body_vec.extend_from_slice(&chunk_buf[..n]).ok();
                    bytes_read += n;
                }

                info!("HTTP: Read chunk of {} bytes", bytes_read);

                // 读取 trailing \r\n
                let mut trailer = [0u8; 2];
                // 先尝试从 header_buf 读取
                if current_pos + 2 <= header_end_pos {
                    trailer[0] = header_buf[current_pos];
                    trailer[1] = header_buf[current_pos + 1];
                    current_pos += 2;
                } else {
                    let _ = socket.read(&mut trailer).await;
                }
            }

            info!("HTTP: Total body bytes read: {}", body_vec.len());
        } else {
            // 读取所有剩余数据
            let mut read_buf = [0u8; 4096];
            loop {
                info!("HTTP: Reading body chunk...");
                let n = socket.read(&mut read_buf).await.map_err(|_e| {
                    error!("HTTP: Failed to read body");
                    HttpError::RequestFailed
                })?;

                info!("HTTP: Read {} bytes", n);

                if n == 0 {
                    info!("HTTP: Body read complete");
                    break;
                }

                if body_vec.is_full() {
                    error!("HTTP: Body buffer full");
                    break;
                }

                body_vec.extend_from_slice(&read_buf[..n]).ok();
                info!("HTTP: Total body bytes: {}", body_vec.len());
            }
        }

        info!("HTTP: Read {} bytes from response", body_vec.len());

        if body_vec.len() > 0 {
            if let Ok(body_str) = core::str::from_utf8(&body_vec) {
                info!(
                    "HTTP: Response body (first 200 chars): {}",
                    &body_str[..body_str.len().min(200)]
                );
            }
        }

        Ok((status, body_vec))
    }
}

impl HttpClient for HttpClientImpl<'_> {
    type Error = HttpError;
    type Response = ResponseImpl;

    async fn request(&mut self, req: &impl HttpRequest) -> Result<Self::Response, Self::Error> {
        let (status, body_vec) = self
            .request_inner(req.method(), req.url(), req.body(), req.headers())
            .await?;

        Ok(ResponseImpl::new(status, &body_vec))
    }
}

fn method_to_str(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::GET => "GET",
        HttpMethod::POST => "POST",
        HttpMethod::PUT => "PUT",
        HttpMethod::DELETE => "DELETE",
        HttpMethod::PATCH => "PATCH",
    }
}

fn extract_host(url: &str) -> Option<&str> {
    // 从 URL 中提取 host
    if let Some(after_scheme) = url.split("://").nth(1) {
        let host_port = after_scheme.split('/').next().unwrap_or(after_scheme);
        let host = host_port.split(':').next().unwrap_or(host_port);
        Some(host)
    } else {
        None
    }
}

fn parse_full_url(url: &str) -> Result<(&str, &str, u16, &str), HttpError> {
    let url = url.trim();
    info!("Parsing URL: {}", url);

    let (scheme, rest) = url.split_once("://").ok_or(HttpError::InvalidUrl)?;
    info!("Scheme: {}, Rest: {}", scheme, rest);

    let default_port = if scheme == "https" { 443 } else { 80 };

    let (host_port, path) = match rest.find('/') {
        Some(pos) => (&rest[..pos], &rest[pos..]),
        None => (rest, "/"),
    };
    info!("Host:Port: {}, Path: {}", host_port, path);

    let (host, port) = match host_port.find(':') {
        Some(pos) => (
            &host_port[..pos],
            host_port[pos + 1..]
                .parse()
                .map_err(|_| HttpError::InvalidUrl)?,
        ),
        None => (host_port, default_port),
    };
    info!("Host: {}, Port: {}", host, port);

    Ok((scheme, host, port, path))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpError {
    NotImplemented,
    InvalidUrl,
    DnsFailed,
    ConnectionFailed,
    RequestFailed,
    TlsHandshakeFailed,
}

#[derive(Debug)]
pub struct RequestImpl {
    method: HttpMethod,
    url: String<512>,
    body: Option<heapless::Vec<u8, 4096>>,
    headers: Option<heapless::Vec<(&'static str, &'static str), 8>>,
}

impl RequestImpl {
    pub fn new(method: HttpMethod, url: &str) -> Self {
        let url: String<512> = String::try_from(url).unwrap_or_default();
        Self {
            method,
            url,
            body: None,
            headers: None,
        }
    }

    pub fn with_body(mut self, body: &[u8]) -> Self {
        let mut body_vec = heapless::Vec::new();
        body_vec.extend_from_slice(body).ok();
        self.body = Some(body_vec);
        self
    }

    pub fn with_header(mut self, key: &'static str, value: &'static str) -> Self {
        let mut headers = self.headers.unwrap_or_default();
        headers.push((key, value)).ok();
        self.headers = Some(headers);
        self
    }
}

impl HttpRequest for RequestImpl {
    fn method(&self) -> HttpMethod {
        self.method
    }

    fn url(&self) -> &str {
        &self.url
    }

    fn headers(&self) -> Option<&[(&str, &str)]> {
        self.headers.as_ref().map(|h| h.as_slice())
    }

    fn body(&self) -> Option<&[u8]> {
        self.body.as_ref().map(|b| b.as_slice())
    }
}

#[derive(Debug)]
pub struct ResponseImpl {
    status: u16,
    body: heapless::Vec<u8, 16384>,
}

impl ResponseImpl {
    pub fn new(status: u16, body: &[u8]) -> Self {
        let mut body_vec = heapless::Vec::new();
        body_vec.extend_from_slice(body).ok();

        Self {
            status,
            body: body_vec,
        }
    }
}

impl HttpResponse for ResponseImpl {
    fn status(&self) -> u16 {
        self.status
    }

    fn headers(&self) -> Option<&[(&str, &str)]> {
        None
    }

    fn body(&self) -> &[u8] {
        &self.body
    }
}
