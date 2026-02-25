pub mod http {
    use core::fmt::Debug;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum HttpMethod {
        GET,
        POST,
        PUT,
        DELETE,
        PATCH,
    }

    pub trait HttpRequest: Debug + Send {
        fn method(&self) -> HttpMethod;
        fn url(&self) -> &str;
        fn headers(&self) -> Option<&[(&str, &str)]>;
        fn body(&self) -> Option<&[u8]>;
    }

    pub trait HttpResponse: Debug + Send {
        fn status(&self) -> u16;
        fn headers(&self) -> Option<&[(&str, &str)]>;
        fn body(&self) -> &[u8];
    }

    pub trait HttpClient: Debug + Send {
        type Error: Debug;

        async fn request(
            &mut self,
            request: &impl HttpRequest,
        ) -> Result<Self::Response, Self::Error>;

        type Response: HttpResponse;
    }
}

pub mod jwt {
    use heapless::String;

    pub trait JwtSigner: Send + Sync {
        fn sign_with_time(
            &self,
            payload: &str,
            timestamp_secs: i64,
        ) -> Result<String<256>, JwtError>;
        fn verify(&self, token: &str) -> Result<(), JwtError>;
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum JwtError {
        InvalidSignature,
        Expired,
        InvalidFormat,
        EncodingError,
    }
}
