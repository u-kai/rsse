use std::{
    fmt::Display,
    io::{BufReader, Write},
    net::TcpStream,
    sync::Arc,
};

use rustls::{ClientConfig, ClientConnection, Stream};

use crate::{request_builder::Request, url::Url};

#[derive(Debug)]
pub struct SseClient {
    client: ClientConnection,
    tcp_stream: TcpStream,
}
#[derive(Debug)]
pub enum SseClientError {
    InvalidUrlError(String),
    ClientConnectionError(String),
    TcpStreamConnectionError(String),
    ReadLineError(String),
}
impl Display for SseClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUrlError(e) => write!(f, "InvalidUrlError: {}", e),
            Self::ClientConnectionError(e) => write!(f, "ClientConnectionError: {}", e),
            Self::ReadLineError(e) => write!(f, "ReadLineError: {}", e),
            Self::TcpStreamConnectionError(e) => write!(f, "TcpStreamConnectionError: {}", e),
        }
    }
}
impl std::error::Error for SseClientError {}

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
            .map_err(|e| SseClientError::TcpStreamConnectionError(e.to_string()))?;
        Ok(Self {
            client,
            tcp_stream: socket,
        })
    }
    pub fn stream_reader<'a>(
        &'a mut self,
        request: Request,
    ) -> Result<BufReader<Stream<'a, ClientConnection, TcpStream>>> {
        let req = request.bytes();
        let mut tls_stream = rustls::Stream::new(&mut self.client, &mut self.tcp_stream);
        tls_stream.write_all(req).map_err(|e| {
            SseClientError::ClientConnectionError(format!(
                "error : {:#?}\nrequest : {:#?}",
                e, request
            ))
        })?;
        let reader = BufReader::new(tls_stream);
        Ok(reader)
    }
}
