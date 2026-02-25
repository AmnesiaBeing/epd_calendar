use core::fmt::Debug;
use embassy_net::Stack;
use heapless::String;
use lxx_calendar_common::http::http::{HttpClient, HttpMethod, HttpRequest, HttpResponse};
use reqwless::client::HttpClient as ReqwlessClient;
use reqwless::request::Method as ReqwlessMethod;

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

impl HttpClient for ReqwlessHttpClient {
    type Error = HttpError;
    type Response = ResponseImpl;

    async fn request(
        &mut self,
        request: &impl HttpRequest,
    ) -> Result<Self::Response, Self::Error> {
        let method = match request.method() {
            HttpMethod::GET => Method::GET,
            HttpMethod::POST => Method::POST,
            HttpMethod::PUT => Method::PUT,
            HttpMethod::DELETE => Method::DELETE,
            HttpMethod::PATCH => Method::PATCH,
        };

        let mut client = ReqwlessClient::new(&self.stack, "reqwless-client");

        let mut request_builder = client.request(method, request.url());

        // TODO: implement headers
        // if let Some(headers) = request.headers() {
        //     for (key, value) in headers {
        //         request_builder = request_builder.header(*key, *value);
        //     }
        // }

        if let Some(body) = request.body() {
            request_builder = request_builder.body(body);
        }

        let response: reqwless::response::Response<4096> = request_builder.send().await.map_err(|_| HttpError::RequestFailed)?;

        let status = response.status().to_u16();
        let mut body = heapless::Vec::<u8, 8192>::new();
        response.body().read_to_end::<heapless::Vec<u8, 8192>>(&mut body).await.map_err(|_| HttpError::ReadFailed)?;

        Ok(ResponseImpl::new(status, &body))
    }
}

pub struct RequestImpl {
    method: ReqwlessMethod,
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
}

impl RequestImpl {
    pub fn new(method: Method, url: &str) -> Self {
        let method = match method {
            Method::Get => ReqwlessMethod::GET,
            Method::Post => ReqwlessMethod::POST,
            Method::Put => ReqwlessMethod::PUT,
            Method::Delete => ReqwlessMethod::DELETE,
        };
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

impl HttpRequest for RequestImpl {
    fn method(&self) -> HttpMethod {
        match self.method {
            ReqwlessMethod::GET => HttpMethod::GET,
            ReqwlessMethod::POST => HttpMethod::POST,
            ReqwlessMethod::PUT => HttpMethod::PUT,
            ReqwlessMethod::DELETE => HttpMethod::DELETE,
            _ => HttpMethod::GET,
        }
    }

    fn url(&self) -> &str {
        &self.url
    }

    fn headers(&self) -> Option<&[(&str, &str)]> {
        None // TODO: implement headers
    }

    fn body(&self) -> Option<&[u8]> {
        self.body.as_ref().map(|b| b.as_slice())
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

impl HttpResponse for ResponseImpl {
    fn status(&self) -> u16 {
        self.status
    }

    fn headers(&self) -> Option<&[(&str, &str)]> {
        None // For simplicity, not implementing headers parsing
    }

    fn body(&self) -> &[u8] {
        &self.body
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpError {
    NotImplemented,
    RequestFailed,
    ReadFailed,
}
