use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    sync::Arc,
};

use rustls::{ClientConfig, ClientConnection};

use crate::{request_builder::RequestBuilder, url::Url};

#[derive(Debug)]
pub struct SseClient {
    client: ClientConnection,
    tcp_stream: TcpStream,
    request_builder: RequestBuilder,
}
#[derive(Debug)]
pub enum SseClientError {
    InvalidUrlError(String),
    ClientConnectionError(String),
    ReadLineError(String),
}

type Result<T> = std::result::Result<T, SseClientError>;

impl SseClient {
    pub fn default(url: &str) -> Result<Self> {
        let url = Url::from_str(url).map_err(|e| SseClientError::InvalidUrlError(e.to_string()))?;
        let mut root_store = rustls::RootCertStore::empty();
        root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
            rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        }));
        let config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        let client = ClientConnection::new(Arc::new(config), url.host().try_into().unwrap())
            .map_err(|e| SseClientError::ClientConnectionError(e.to_string()))?;
        let socket = TcpStream::connect(url.to_addr_str())
            .map_err(|e| SseClientError::ClientConnectionError(e.to_string()))?;
        Ok(Self {
            client,
            tcp_stream: socket,
            request_builder: RequestBuilder::new(url),
        })
    }
    pub fn post(mut self) -> Self {
        self.request_builder = self.request_builder.post();
        self
    }
    pub fn bearer_auth(mut self, token: &str) -> Self {
        self.request_builder = self.request_builder.bearer_auth(token);
        self
    }
    pub fn json_body<T: serde::Serialize>(mut self, body: T) -> Self {
        self.request_builder = self.request_builder.json(body);
        self
    }
    pub fn read_stream(mut self, line_handler: impl Fn(&str) -> Result<()>) -> Result<()> {
        let req = self.request_builder.to_request();
        let mut tls_stream = rustls::Stream::new(&mut self.client, &mut self.tcp_stream);
        tls_stream
            .write_all(req.as_bytes())
            .map_err(|e| SseClientError::ClientConnectionError(e.to_string()))?;
        let mut reader = BufReader::new(tls_stream);
        let mut line = String::new();
        while reader
            .read_line(&mut line)
            .map_err(|e| SseClientError::ReadLineError(e.to_string()))?
            > 0
        {
            line_handler(&line)?;
            line.clear();
        }
        Ok(())
    }
    pub fn read_stream_data(mut self, data_handler: impl Fn(&str) -> Result<()>) -> Result<()> {
        let req = self.request_builder.to_request();
        let mut tls_stream = rustls::Stream::new(&mut self.client, &mut self.tcp_stream);
        tls_stream
            .write_all(req.as_bytes())
            .map_err(|e| SseClientError::ClientConnectionError(e.to_string()))?;
        let mut reader = BufReader::new(tls_stream);
        let mut line = String::new();
        while reader
            .read_line(&mut line)
            .map_err(|e| SseClientError::ReadLineError(e.to_string()))?
            > 0
        {
            if line.starts_with("data:") {
                let data = line.trim_start_matches("data:").trim();
                data_handler(data)?;
            }
            line.clear();
        }
        Ok(())
    }
}
