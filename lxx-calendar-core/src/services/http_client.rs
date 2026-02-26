use core::fmt::Debug;
use embassy_net::tcp::TcpSocket;
use embassy_net::Stack;
use embassy_net::IpEndpoint;
use embassy_net::dns::DnsQueryType;
use embedded_io_async::{Read, Write};
use embedded_tls::Aes128GcmSha256;
use embedded_tls::NoVerify;
use embedded_tls::TlsConfig;
use embedded_tls::TlsConnection;
use embedded_tls::TlsContext;
use embedded_tls::UnsecureProvider;
use heapless::String;
use lxx_calendar_common::http::http::{HttpClient, HttpMethod, HttpRequest, HttpResponse};
use rand::SeedableRng;
use rand::rngs::StdRng;

const RX_BUFFER_SIZE: usize = 4096;
const TX_BUFFER_SIZE: usize = 4096;
const TLS_BUFFER_SIZE: usize = 16384;

pub struct HttpClientImpl {
    stack: Stack<'static>,
    tls_read_buffer: [u8; TLS_BUFFER_SIZE],
    tls_write_buffer: [u8; TLS_BUFFER_SIZE],
}

impl Debug for HttpClientImpl {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "HttpClientImpl")
    }
}

impl HttpClientImpl {
    pub fn new(stack: Stack<'static>) -> Self {
        Self { 
            stack,
            tls_read_buffer: [0u8; TLS_BUFFER_SIZE],
            tls_write_buffer: [0u8; TLS_BUFFER_SIZE],
        }
    }

    pub async fn request(
        &mut self,
        rx_buffer: &mut [u8; RX_BUFFER_SIZE],
        tx_buffer: &mut [u8; TX_BUFFER_SIZE],
        method: HttpMethod,
        url: &str,
        body: Option<&[u8]>,
    ) -> Result<(u16, heapless::Vec<u8, 16384>), HttpError> {
        let (scheme, host, port, path) = parse_full_url(url)?;
        
        let is_https = scheme.eq_ignore_ascii_case("https");
        let port: u16 = if port.is_empty() { 
            if is_https { 443 } else { 80 } 
        } else { 
            port.parse().unwrap_or(if is_https { 443 } else { 80 }) 
        };

        let ip_addrs = self.stack
            .dns_query(host, DnsQueryType::A)
            .await
            .map_err(|_| HttpError::DnsFailed)?;
        
        let ip = ip_addrs.first().ok_or(HttpError::DnsFailed)?;

        let endpoint: IpEndpoint = (*ip, port).into();

        let mut socket = TcpSocket::new(self.stack, rx_buffer, tx_buffer);
        socket.connect(endpoint).await.map_err(|_| HttpError::ConnectionFailed)?;

        if is_https {
            // Note: In real embedded, use proper hardware RNG. 
            // For testing, we use a seeded StdRng - replace with chip-specific RNG in production.
            let mut rng = StdRng::from_seed([0u8; 32]);
            
            let tls_config = TlsConfig::new()
                .with_server_name(host);
            
            type TlsSocket<'a> = TlsConnection<'a, TcpSocket<'a>, Aes128GcmSha256>;
            let mut tls: TlsSocket<'_> = TlsConnection::new(
                socket, 
                &mut self.tls_read_buffer[..], 
                &mut self.tls_write_buffer[..]
            );
            
            let provider = UnsecureProvider::<(), _>::new(&mut rng);
            tls.open(TlsContext::new(&tls_config, provider))
                .await
                .map_err(|_| HttpError::TlsFailed)?;
            
            let result = do_https_request(&mut tls, method, path, body).await;
            return result;
        } else {
            let result = do_http_request(&mut socket, method, path, body).await;
            let _ = socket.close();
            result
        }
    }
}

async fn do_http_request(
    socket: &mut TcpSocket<'_>,
    method: HttpMethod,
    path: &str,
    body: Option<&[u8]>,
) -> Result<(u16, heapless::Vec<u8, 16384>), HttpError> {
    let method_str = match method {
        HttpMethod::GET => "GET",
        HttpMethod::POST => "POST",
        HttpMethod::PUT => "PUT",
        HttpMethod::DELETE => "DELETE",
        HttpMethod::PATCH => "PATCH",
    };

    let mut request_str = heapless::String::<512>::try_from("").unwrap_or_default();
    request_str.push_str(method_str).ok();
    request_str.push(' ').ok();
    request_str.push_str(path).ok();
    request_str.push_str(" HTTP/1.1\r\n").ok();
    request_str.push_str("Connection: close\r\n").ok();
    
    if let Some(body) = body {
        request_str.push_str("Content-Type: application/json\r\n").ok();
        request_str.push_str("Content-Length: ").ok();
        let len_str = int_to_heapless_string(body.len());
        request_str.push_str(&len_str).ok();
        request_str.push_str("\r\n").ok();
    }
    
    request_str.push_str("\r\n").ok();
    
    write_all(socket, request_str.as_bytes()).await?;
    
    if let Some(body) = body {
        write_all(socket, body).await?;
    }
    
    socket.flush().await.map_err(|_| HttpError::WriteFailed)?;

    read_response(socket).await
}

async fn write_all(socket: &mut TcpSocket<'_>, data: &[u8]) -> Result<(), HttpError> {
    let mut offset = 0;
    while offset < data.len() {
        let written = socket.write(&data[offset..]).await.map_err(|_| HttpError::WriteFailed)?;
        offset += written;
    }
    Ok(())
}

async fn read_response(socket: &mut TcpSocket<'_>) -> Result<(u16, heapless::Vec<u8, 16384>), HttpError> {
    let mut status = 0;
    let mut headers_done = false;
    let mut content_length = 0;
    
    let mut buf = [0u8; 2048];
    let mut response_buf = heapless::Vec::<u8, 16384>::new();
    
    loop {
        let n = socket.read(&mut buf).await.map_err(|_| HttpError::ReadFailed)?;
        if n == 0 {
            break;
        }
        
        if response_buf.capacity() - response_buf.len() < n {
            break;
        }
        
        response_buf.extend_from_slice(&buf[..n]).ok();
        
        if !headers_done {
            if let Some(pos) = response_buf.windows(4).position(|w| w == b"\r\n\r\n") {
                let header_str = core::str::from_utf8(&response_buf[..pos]).unwrap_or("");
                
                for line in header_str.lines() {
                    if line.starts_with("HTTP/") {
                        if let Some(code) = line.split_whitespace().nth(1) {
                            status = code.parse().unwrap_or(0);
                        }
                    }
                    if line.to_lowercase().starts_with("content-length:") {
                        content_length = line.split(':').nth(1)
                            .map(|s| s.trim().parse().unwrap_or(0))
                            .unwrap_or(0);
                    }
                    if line.to_lowercase().starts_with("transfer-encoding:") && line.to_lowercase().contains("chunked") {
                        content_length = usize::MAX;
                    }
                }
                
                headers_done = true;
                
                if content_length != usize::MAX && response_buf.len() >= content_length {
                    break;
                }
                if content_length == 0 {
                    break;
                }
            }
        } else {
            if content_length != usize::MAX && response_buf.len() >= content_length {
                break;
            }
        }
    }
    
    Ok((status, response_buf))
}

async fn do_https_request<S>(
    tls: &mut TlsConnection<'_, S, Aes128GcmSha256>,
    method: HttpMethod,
    path: &str,
    body: Option<&[u8]>,
) -> Result<(u16, heapless::Vec<u8, 16384>), HttpError>
where
    S: embedded_io_async::Read + embedded_io_async::Write,
{
    let method_str = match method {
        HttpMethod::GET => "GET",
        HttpMethod::POST => "POST",
        HttpMethod::PUT => "PUT",
        HttpMethod::DELETE => "DELETE",
        HttpMethod::PATCH => "PATCH",
    };

    let mut request_str = heapless::String::<512>::try_from("").unwrap_or_default();
    request_str.push_str(method_str).ok();
    request_str.push(' ').ok();
    request_str.push_str(path).ok();
    request_str.push_str(" HTTP/1.1\r\n").ok();
    request_str.push_str("Connection: close\r\n").ok();
    request_str.push_str("Host: ").ok();
    // Host would need to be passed separately for TLS - for now omit
    request_str.push_str("\r\n").ok();
    
    if let Some(body) = body {
        request_str.push_str("Content-Type: application/json\r\n").ok();
        request_str.push_str("Content-Length: ").ok();
        let len_str = int_to_heapless_string(body.len());
        request_str.push_str(&len_str).ok();
        request_str.push_str("\r\n").ok();
    }
    
    request_str.push_str("\r\n").ok();
    
    tls.write_all(request_str.as_bytes()).await.map_err(|_| HttpError::WriteFailed)?;
    
    if let Some(body) = body {
        tls.write_all(body).await.map_err(|_| HttpError::WriteFailed)?;
    }
    
    tls.flush().await.map_err(|_| HttpError::WriteFailed)?;

    read_https_response(tls).await
}

async fn read_https_response<S>(
    tls: &mut TlsConnection<'_, S, Aes128GcmSha256>,
) -> Result<(u16, heapless::Vec<u8, 16384>), HttpError>
where
    S: embedded_io_async::Read + embedded_io_async::Write,
{
    let mut status = 0;
    let mut headers_done = false;
    let mut content_length = 0;
    
    let mut buf = [0u8; 2048];
    let mut response_buf = heapless::Vec::<u8, 16384>::new();
    
    loop {
        let n = tls.read(&mut buf).await.map_err(|_| HttpError::ReadFailed)?;
        if n == 0 {
            break;
        }
        
        if response_buf.capacity() - response_buf.len() < n {
            break;
        }
        
        response_buf.extend_from_slice(&buf[..n]).ok();
        
        if !headers_done {
            if let Some(pos) = response_buf.windows(4).position(|w| w == b"\r\n\r\n") {
                let header_str = core::str::from_utf8(&response_buf[..pos]).unwrap_or("");
                
                for line in header_str.lines() {
                    if line.starts_with("HTTP/") {
                        if let Some(code) = line.split_whitespace().nth(1) {
                            status = code.parse().unwrap_or(0);
                        }
                    }
                    if line.to_lowercase().starts_with("content-length:") {
                        content_length = line.split(':').nth(1)
                            .map(|s| s.trim().parse().unwrap_or(0))
                            .unwrap_or(0);
                    }
                    if line.to_lowercase().starts_with("transfer-encoding:") && line.to_lowercase().contains("chunked") {
                        content_length = usize::MAX;
                    }
                }
                
                headers_done = true;
                
                if content_length != usize::MAX && response_buf.len() >= content_length {
                    break;
                }
                if content_length == 0 {
                    break;
                }
            }
        } else {
            if content_length != usize::MAX && response_buf.len() >= content_length {
                break;
            }
        }
    }
    
    Ok((status, response_buf))
}

impl HttpClient for HttpClientImpl {
    type Error = HttpError;
    type Response = ResponseImpl;

    async fn request(
        &mut self,
        _request: &impl HttpRequest,
    ) -> Result<Self::Response, Self::Error> {
        Err(HttpError::NotImplemented)
    }
}

fn int_to_heapless_string(mut n: usize) -> heapless::String<16> {
    let mut s = heapless::String::<16>::new();
    if n == 0 {
        s.push('0').ok();
        return s;
    }
    let mut digits = heapless::Vec::<u8, 16>::new();
    while n > 0 {
        let d = (n % 10) as u8;
        digits.push(b'0' + d).ok();
        n /= 10;
    }
    digits.reverse();
    for &d in &digits {
        s.push(d as char).ok();
    }
    s
}

fn parse_full_url(url: &str) -> Result<(&str, &str, &str, &str), HttpError> {
    let url = url.trim();
    
    let (scheme, rest) = url.split_once("://").ok_or(HttpError::InvalidUrl)?;
    
    let (host_port, path) = match rest.find('/') {
        Some(pos) => (&rest[..pos], &rest[pos..]),
        None => (rest, "/"),
    };
    
    let (host, port) = match host_port.find(':') {
        Some(pos) => (&host_port[..pos], &host_port[pos + 1..]),
        None => (host_port, ""),
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
    WriteFailed,
    ReadFailed,
    BufferTooSmall,
    TlsFailed,
}
