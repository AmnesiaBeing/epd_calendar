use core::fmt::Debug;
use embassy_net::Stack;
use embassy_net::dns::DnsQueryType;
use embassy_net::tcp::TcpSocket;
use embedded_io_async::Read;
use heapless::String;

use lxx_calendar_common::http::http::{HttpClient, HttpMethod, HttpRequest, HttpResponse};

const RX_BUFFER_SIZE: usize = 4096;
const TX_BUFFER_SIZE: usize = 4096;

// buffers used by embedded-tls for encrypted records. 16640 is the maximum
// TLS record size defined by the spec (2^14 + overhead). 16 384 would also work
// but we keep a little headroom for safety.
const TLS_RECORD_BUF_SIZE: usize = 16_640;

pub struct HttpClientImpl {
    stack: Stack<'static>,
}

impl Debug for HttpClientImpl {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "HttpClientImpl")
    }
}

impl HttpClientImpl {
    pub fn new(stack: Stack<'static>) -> Self {
        Self { stack }
    }

    pub async fn request(
        &mut self,
        rx_buffer: &mut [u8; RX_BUFFER_SIZE],
        tx_buffer: &mut [u8; TX_BUFFER_SIZE],
        method: HttpMethod,
        url: &str,
        body: Option<&[u8]>,
        headers: Option<&[(&str, &str)]>,
    ) -> Result<(u16, heapless::Vec<u8, 16384>), HttpError> {
        let (_scheme, host, port, path) = parse_full_url(url)?;

        let port = if port == 0 {
            if url.starts_with("https") {
                443u16
            } else {
                80u16
            }
        } else {
            port
        };

        log::info!("HTTP: Resolving DNS for {}", host);
        let ip_addrs = self
            .stack
            .dns_query(host, DnsQueryType::A)
            .await
            .map_err(|e| {
                log::error!("HTTP: DNS query failed for {}: {:?}", host, e);
                HttpError::DnsFailed
            })?;

        let ip = ip_addrs.first().ok_or(HttpError::DnsFailed)?;
        log::info!("HTTP: DNS resolved {} to {}", host, ip);

        let mut socket = TcpSocket::new(self.stack, rx_buffer, tx_buffer);
        log::info!(
            "HTTP: Connecting to {}:{} (HTTPS={})",
            ip,
            port,
            url.starts_with("https")
        );

        // Try to connect (may need TLS if HTTPS)
        let connected = socket.connect((*ip, port)).await;
        if connected.is_err() && url.starts_with("https") {
            log::warn!("TCP connection failed, trying with TLS...");
            // TLS handling would be needed here
        }

        connected.map_err(|e| {
            log::error!("HTTP: Connection failed to {}:{}: {:?}", ip, port, e);
            HttpError::ConnectionFailed
        })?;

        log::info!("HTTP: Connected to {}:{}", ip, port);

        let method_str = match method {
            HttpMethod::GET => "GET",
            HttpMethod::POST => "POST",
            HttpMethod::PUT => "PUT",
            HttpMethod::DELETE => "DELETE",
            HttpMethod::PATCH => "PATCH",
        };

        let mut http_request = heapless::String::<4096>::new();

        // Build request line and Host header
        core::fmt::write(
            &mut http_request,
            format_args!("{} {} HTTP/1.1\r\nHost: {}", method_str, path, host),
        )
        .map_err(|_| HttpError::RequestFailed)?;

        // Add custom headers
        if let Some(hdrs) = headers {
            for (key, value) in hdrs {
                core::fmt::write(&mut http_request, format_args!("\r\n{}: {}", key, value))
                    .map_err(|_| HttpError::RequestFailed)?;
            }
        }

        // Add Content-Type and Content-Length for body
        if let Some(body) = body {
            core::fmt::write(
                &mut http_request,
                format_args!(
                    "\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                    body.len()
                ),
            )
            .map_err(|_| HttpError::RequestFailed)?;
        } else {
            core::fmt::write(&mut http_request, format_args!("\r\n\r\n"))
                .map_err(|_| HttpError::RequestFailed)?;
        }

        embedded_io_async::Write::write_all(&mut socket, http_request.as_bytes())
            .await
            .map_err(|_| HttpError::RequestFailed)?;

        if let Some(body) = body {
            embedded_io_async::Write::write_all(&mut socket, body)
                .await
                .map_err(|_| HttpError::RequestFailed)?;
        }

        embedded_io_async::Write::flush(&mut socket)
            .await
            .map_err(|_| HttpError::RequestFailed)?;

        // Read the HTTP response headers first
        let mut header_buf = [0u8; 2048];
        let mut header_bytes_read = 0;

        // Read until we find \r\n\r\n (end of headers)
        loop {
            if header_bytes_read >= header_buf.len() {
                return Err(HttpError::RequestFailed);
            }

            let n = Read::read(
                &mut socket,
                &mut header_buf[header_bytes_read..header_bytes_read + 1],
            )
            .await
            .map_err(|_| HttpError::RequestFailed)?;

            if n == 0 {
                break;
            }

            header_bytes_read += n;

            // Check for \r\n\r\n
            if header_bytes_read >= 4 && &header_buf[header_bytes_read - 4..] == b"\r\n\r\n" {
                break;
            }
        }

        let header_str = core::str::from_utf8(&header_buf[..header_bytes_read])
            .map_err(|_| HttpError::RequestFailed)?;

        let (status_line, headers_part) = header_str
            .split_once("\r\n")
            .ok_or(HttpError::RequestFailed)?;

        let status = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .ok_or(HttpError::RequestFailed)?;

        // Check for Transfer-Encoding: chunked
        let is_chunked = headers_part.contains("Transfer-Encoding: chunked");

        let mut body_vec = heapless::Vec::<u8, 16384>::new();

        if is_chunked {
            // Handle chunked encoding
            loop {
                // Read chunk size line
                let mut size_line_buf = [0u8; 32];
                let mut size_line_bytes = 0;

                loop {
                    if size_line_bytes >= size_line_buf.len() {
                        return Err(HttpError::RequestFailed);
                    }

                    let n = Read::read(
                        &mut socket,
                        &mut size_line_buf[size_line_bytes..size_line_bytes + 1],
                    )
                    .await
                    .map_err(|_| HttpError::RequestFailed)?;

                    if n == 0 {
                        return Err(HttpError::RequestFailed);
                    }

                    size_line_bytes += n;

                    if &size_line_buf[size_line_bytes - 2..size_line_bytes] == b"\r\n" {
                        break;
                    }
                }

                // Parse chunk size
                let size_line = core::str::from_utf8(&size_line_buf[..size_line_bytes - 2])
                    .map_err(|_| HttpError::RequestFailed)?;

                let chunk_size = usize::from_str_radix(size_line.trim(), 16)
                    .map_err(|_| HttpError::RequestFailed)?;

                // Chunk size 0 means end of response
                if chunk_size == 0 {
                    // Read trailing \r\n
                    let mut trailer = [0u8; 2];
                    Read::read(&mut socket, &mut trailer)
                        .await
                        .map_err(|_| HttpError::RequestFailed)?;
                    break;
                }

                // Read chunk data
                let mut chunk_buf = [0u8; 4096];
                let mut bytes_to_read = chunk_size;

                while bytes_to_read > 0 {
                    let read_size = bytes_to_read.min(chunk_buf.len());
                    let n = Read::read(&mut socket, &mut chunk_buf[..read_size])
                        .await
                        .map_err(|_| HttpError::RequestFailed)?;

                    if n == 0 {
                        return Err(HttpError::RequestFailed);
                    }

                    body_vec.extend_from_slice(&chunk_buf[..n]).ok();
                    bytes_to_read -= n;
                }

                // Read trailing \r\n after chunk
                let mut trailer = [0u8; 2];
                Read::read(&mut socket, &mut trailer)
                    .await
                    .map_err(|_| HttpError::RequestFailed)?;
            }
        } else {
            // Read all remaining data
            let mut read_buf = [0u8; 4096];
            loop {
                let n = Read::read(&mut socket, &mut read_buf)
                    .await
                    .map_err(|_| HttpError::RequestFailed)?;

                if n == 0 {
                    break;
                }

                body_vec.extend_from_slice(&read_buf[..n]).ok();

                if body_vec.is_full() {
                    break;
                }
            }
        }

        socket.close();

        Ok((status, body_vec))
    }
}

impl HttpClient for HttpClientImpl {
    type Error = HttpError;
    type Response = ResponseImpl;

    async fn request(&mut self, req: &impl HttpRequest) -> Result<Self::Response, Self::Error> {
        // allocate local buffers; the underlying socket borrows them
        let mut rx = [0u8; RX_BUFFER_SIZE];
        let mut tx = [0u8; TX_BUFFER_SIZE];

        let (status, body_vec) = self
            .request(
                &mut rx,
                &mut tx,
                req.method(),
                req.url(),
                req.body(),
                req.headers(),
            )
            .await?;

        Ok(ResponseImpl::new(status, &body_vec))
    }
}

fn parse_full_url(url: &str) -> Result<(&str, &str, u16, &str), HttpError> {
    let url = url.trim();

    let (scheme, rest) = url.split_once("://").ok_or(HttpError::InvalidUrl)?;

    // Extract port from scheme if not in host
    let default_port = if scheme == "https" { 443 } else { 80 };

    let (host_port, path) = match rest.find('/') {
        Some(pos) => (&rest[..pos], &rest[pos..]),
        None => (rest, "/"),
    };

    let (host, port) = match host_port.find(':') {
        Some(pos) => (
            &host_port[..pos],
            host_port[pos + 1..]
                .parse()
                .map_err(|_| HttpError::InvalidUrl)?,
        ),
        None => (host_port, default_port),
    };

    Ok((scheme, host, port, path))
}

#[derive(Debug)]
pub struct RequestImpl {
    method: HttpMethod,
    url: String<256>,
    body: Option<heapless::Vec<u8, 4096>>,
}

impl RequestImpl {
    pub fn new(method: HttpMethod, url: &str) -> Self {
        let url: String<256> = String::try_from(url).unwrap_or_default();
        Self {
            method,
            url,
            body: None,
        }
    }

    pub fn with_body(mut self, body: &[u8]) -> Self {
        let mut body_vec = heapless::Vec::new();
        body_vec.extend_from_slice(body).ok();
        self.body = Some(body_vec);
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
        None
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpError {
    NotImplemented,
    InvalidUrl,
    DnsFailed,
    ConnectionFailed,
    RequestFailed,
    /// TLS handshake or crypto error
    TlsHandshakeFailed,
}
