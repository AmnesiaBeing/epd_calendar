use core::fmt::Debug;
use embassy_net::Stack;
use heapless::String;

pub struct ReqwlessHttpClient {
    stack: Stack<'static>,
}

impl Debug for ReqwlessHttpClient {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "ReqwlessHttpClient")
    }
}

impl ReqwlessHttpClient {
    pub fn new(stack: Stack<'static>) -> Self {
        Self { stack }
    }
}

pub struct RequestImpl {
    method: Method,
    url: String<256>,
    headers: heapless::Vec<(&'static str, heapless::String<256>), 4>,
    body: Option<heapless::Vec<u8, 4096>>,
}

#[derive(Debug, Clone, Copy)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

impl RequestImpl {
    pub fn new(method: Method, url: &str) -> Self {
        let url: String<256> = String::try_from(url).unwrap_or_default();
        Self {
            method,
            url,
            headers: heapless::Vec::new(),
            body: None,
        }
    }

    pub fn with_header(mut self, key: &'static str, value: &str) -> Self {
        let value: heapless::String<256> = String::try_from(value).unwrap_or_default();
        self.headers.push((key, value)).ok();
        self
    }

    pub fn with_body(mut self, body: &[u8]) -> Self {
        let mut body_vec = heapless::Vec::new();
        body_vec.extend_from_slice(body).ok();
        self.body = Some(body_vec);
        self
    }
}

impl Debug for RequestImpl {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RequestImpl")
            .field("method", &self.method)
            .field("url", &self.url)
            .finish()
    }
}

pub struct ResponseImpl {
    status: u16,
    body: heapless::Vec<u8, 8192>,
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

impl Debug for ResponseImpl {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ResponseImpl")
            .field("status", &self.status)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpError {
    NotImplemented,
    RequestFailed,
    ReadFailed,
}
