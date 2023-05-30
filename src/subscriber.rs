use std::{
    fmt::Display,
    fs::File,
    io::{BufReader, Read, Write},
    net::TcpStream,
    path::Path,
    sync::Arc,
};

use rustls::{Certificate, ClientConfig, ClientConnection, RootCertStore, Stream};
use rustls_pemfile::{read_one, Item};

use crate::{
    request_builder::{Request, RequestBuilder},
    url::Url,
};

#[derive(Debug)]
pub struct SseSubscriber {
    url: Url,
    client: Option<ClientConnection>,
    root_store: RootCertStore,
    tcp_stream: TcpStream,
}
#[derive(Debug)]
pub enum SseSubscriberError {
    InvalidUrlError(String),
    ClientConnectionError(String),
    TcpStreamConnectionError(String),
    ReadLineError(String),
    WriteAllError { message: String, request: Request },
}
impl Display for SseSubscriberError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUrlError(e) => write!(f, "InvalidUrlError: {}", e),
            Self::ClientConnectionError(e) => write!(f, "ClientConnectionError: {}", e),
            Self::ReadLineError(e) => write!(f, "ReadLineError: {}", e),
            Self::TcpStreamConnectionError(e) => write!(f, "TcpStreamConnectionError: {}", e),
            Self::WriteAllError { message, .. } => write!(f, "WriteAllError: {}", message),
        }
    }
}
impl std::error::Error for SseSubscriberError {}

type Result<T> = std::result::Result<T, SseSubscriberError>;

impl SseSubscriber {
    pub fn default(url: &str) -> Result<Self> {
        let url =
            Url::from_str(url).map_err(|e| SseSubscriberError::InvalidUrlError(e.to_string()))?;
        let mut root_store = rustls::RootCertStore::empty();
        root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
            rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        }));
        let socket = TcpStream::connect(url.to_addr_str())
            .map_err(|e| SseSubscriberError::TcpStreamConnectionError(e.to_string()))?;
        Ok(Self {
            url: url,
            client: None,
            root_store: root_store,
            tcp_stream: socket,
        })
    }
    pub fn add_ca(&mut self, ca_path: impl AsRef<Path>) -> Result<()> {
        let file = File::open(ca_path)
            .map_err(|e| SseSubscriberError::ClientConnectionError(e.to_string()))?;
        let mut reader = BufReader::new(file);
        match read_one(&mut reader).unwrap().unwrap() {
            Item::X509Certificate(cert) => {
                let cert = Certificate(cert);
                self.root_store.add(&cert).unwrap();
            }
            _ => println!("error"),
        };
        Ok(())
    }
    pub fn with_proxy(proxy_url: &str, url: &str) -> Result<Self> {
        let mut root_store = rustls::RootCertStore::empty();
        root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
            rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        }));
        let url =
            Url::from_str(url).map_err(|e| SseSubscriberError::InvalidUrlError(e.to_string()))?;
        let mut socket = TcpStream::connect(proxy_url)
            .map_err(|e| SseSubscriberError::TcpStreamConnectionError(e.to_string()))?;
        let request = RequestBuilder::new(url.clone()).connect().build();
        socket.write_all(request.bytes()).unwrap();
        let mut buf = vec![0; 4096];
        while socket.read(&mut buf).unwrap() > 0 {
            let response = String::from_utf8(buf.clone()).unwrap();
            if response.contains("200 OK") {
                break;
            }
        }
        Ok(Self {
            url: url,
            client: None,
            root_store: root_store,
            tcp_stream: socket,
        })
    }
    fn set_client(&mut self) {
        let ip = self.url.host().try_into().unwrap();
        let config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(self.root_store.clone())
            .with_no_client_auth();
        self.client = Some(ClientConnection::new(Arc::new(config), ip).unwrap());
    }
    pub fn subscribe_stream<'a>(
        &'a mut self,
        request: &Request,
    ) -> Result<BufReader<Stream<'a, ClientConnection, TcpStream>>> {
        if let Ok(cert) = std::env::var("CA_BUNDLE") {
            self.add_ca(cert)?;
        }
        self.set_client();
        let req = request.bytes();
        let mut tls_stream =
            rustls::Stream::new(self.client.as_mut().unwrap(), &mut self.tcp_stream);
        tls_stream
            .write_all(req)
            .map_err(|e| SseSubscriberError::WriteAllError {
                message: format!("error : {:#?}\n", e.to_string(),),
                request: request.clone(),
            })?;
        let reader = BufReader::new(tls_stream);
        Ok(reader)
    }
}
