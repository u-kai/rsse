use std::{
    fmt::Display,
    io::{BufReader, Write},
    net::TcpStream,
    sync::Arc,
};

use rustls::{ClientConfig, ClientConnection, Stream};

use crate::{request_builder::Request, url::Url};

#[derive(Debug)]
pub struct SseSubscriber {
    client: ClientConnection,
    tcp_stream: TcpStream,
}
#[derive(Debug)]
pub enum SseSubscriberError {
    InvalidUrlError(String),
    ClientConnectionError(String),
    TcpStreamConnectionError(String),
    ReadLineError(String),
    WriteAllError(String),
}
impl Display for SseSubscriberError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUrlError(e) => write!(f, "InvalidUrlError: {}", e),
            Self::ClientConnectionError(e) => write!(f, "ClientConnectionError: {}", e),
            Self::ReadLineError(e) => write!(f, "ReadLineError: {}", e),
            Self::TcpStreamConnectionError(e) => write!(f, "TcpStreamConnectionError: {}", e),
            Self::WriteAllError(e) => write!(f, "WriteAllError: {}", e),
        }
    }
}
impl std::error::Error for SseSubscriberError {}

type Result<T> = std::result::Result<T, SseSubscriberError>;

impl SseSubscriber {
    pub fn default(url: &str) -> Result<Self> {
        let url =
            Url::from_str(url).map_err(|e| SseSubscriberError::InvalidUrlError(e.to_string()))?;
        let ip = url.host().try_into().map_err(|_| {
            SseSubscriberError::InvalidUrlError(format!(
                "{} can not be resolved to an IPv4 or IPv6 address",
                url.to_string()
            ))
        })?;
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
        let client = ClientConnection::new(Arc::new(config), ip)
            .map_err(|e| SseSubscriberError::ClientConnectionError(e.to_string()))?;
        let socket = TcpStream::connect(url.to_addr_str())
            .map_err(|e| SseSubscriberError::TcpStreamConnectionError(e.to_string()))?;
        Ok(Self {
            client,
            tcp_stream: socket,
        })
    }
    pub fn subscribe_stream<'a>(
        &'a mut self,
        request: Request,
    ) -> Result<BufReader<Stream<'a, ClientConnection, TcpStream>>> {
        let req = request.bytes();
        let mut tls_stream = rustls::Stream::new(&mut self.client, &mut self.tcp_stream);
        tls_stream.write_all(req).map_err(|e| {
            SseSubscriberError::WriteAllError(format!("error : {:#?}\nrequest : {:#?}", e, request))
        })?;
        let reader = BufReader::new(tls_stream);
        Ok(reader)
    }
}
