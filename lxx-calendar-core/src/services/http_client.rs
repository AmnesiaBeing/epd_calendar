use core::fmt::Debug;
use embassy_net::tcp::TcpSocket;
use embassy_net::Stack;
use embassy_net::dns::DnsQueryType;
use heapless::String;
use lxx_calendar_common::http::http::{HttpClient, HttpMethod, HttpRequest, HttpResponse};
use reqwless::client::HttpClient as ReqwestHttpClient;
use reqwless::request::Request;

const RX_BUFFER_SIZE: usize = 4096;
const TX_BUFFER_SIZE: usize = 4096;

pub struct HttpClientImpl {
    stack: Stack<'static>,
    client: Option<ReqwestHttpClient<'static>>,
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
            client: None,
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
        let (_scheme, host, _port, path) = parse_full_url(url)?;
        
        let ip_addrs = self.stack
            .dns_query(host, DnsQueryType::A)
            .await
            .map_err(|_| HttpError::DnsFailed)?;
        
        let ip = ip_addrs.first().ok_or(HttpError::DnsFailed)?;

        let mut socket = TcpSocket::new(self.stack, rx_buffer, tx_buffer);
        socket.connect((*ip, 443)).await.map_err(|_| HttpError::ConnectionFailed)?;

        let mut client = ReqwestHttpClient::new(socket);
        
        let tls_config = reqwless::client::TlsConfig::default();
        client.set_tls_config(tls_config).ok();

        let req_method = match method {
            HttpMethod::GET => reqwless::request::Method::GET,
            HttpMethod::POST => reqwless::request::Method::POST,
            HttpMethod::PUT => reqwless::request::Method::PUT,
            HttpMethod::DELETE => reqwless::request::Method::DELETE,
            HttpMethod::PATCH => reqwless::request::Method::PATCH,
        };

        let mut request = Request::new(req_method, url);
        if let Some(body) = body {
            request = request
                .header("Content-Type", "application/json")
                .body(body                .ok()
                .unwrap_or(request);
        }

        let response = client.send)
(request).await.map_err(|_| HttpError::RequestFailed)?;
        
        let status = response.status().as_u16();
        
        let mut body_vec = heapless::Vec::<u8, 16384>::new();
        let mut body_reader = response.body();
        let mut buf = [0u8; 1024];
        loop {
            match body_reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    body_vec.extend_from_slice(&buf[..n]).ok();
                }
                Err(_) => break,
            }
        }
        
        socket.close();
        
        Ok((status, body_vec))
    }
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
    RequestFailed,
}
