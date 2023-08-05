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
    debug,
    request::{Request, RequestBuilder},
    url::Url,
};

#[derive(Debug)]
pub struct SseSubscriber {
    client: ClientConnection,
    tcp_stream: TcpStream,
    req: Request,
}
#[derive(Debug)]
pub enum SseSubscriberError {
    InvalidUrlError(String),
    ClientConnectionError(String),
    TcpStreamConnectionError(String),
    ReadLineError(String),
    ProxyUrlError(String),
    ProxyConnectionError(String),
    WriteAllError { message: String, request: Request },
}
impl Display for SseSubscriberError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProxyConnectionError(e) => write!(f, "ProxyConnectionError: {}", e),
            Self::InvalidUrlError(e) => write!(f, "InvalidUrlError: {}", e),
            Self::ProxyUrlError(e) => write!(f, "ProxyUrlError: {}", e),
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
    pub fn subscribe_stream(&mut self) -> Result<BufReader<Stream<ClientConnection, TcpStream>>> {
        let mut tls_stream = rustls::Stream::new(&mut self.client, &mut self.tcp_stream);
        tls_stream
            .write_all(self.req.bytes())
            .map_err(|e| SseSubscriberError::WriteAllError {
                message: format!("error : {:#?}\n", e.to_string(),),
                request: self.req.clone(),
            })?;
        debug!(tls_stream);
        let reader = BufReader::new(tls_stream);
        Ok(reader)
    }
}

#[derive(Debug)]
pub struct SubscriberBuilder {
    root_store: rustls::RootCertStore,
    request_builder: RequestBuilder,
}

impl SubscriberBuilder {
    pub fn new(url: &str) -> Self {
        Self {
            root_store: Self::default_ca(),
            request_builder: RequestBuilder::new(url),
        }
    }
    pub fn add_ca(mut self, ca_path: impl AsRef<Path>) -> Result<Self> {
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
        Ok(self)
    }
    pub fn get(mut self) -> Self {
        self.request_builder = self.request_builder.get();
        self
    }
    pub fn json<T: serde::Serialize>(mut self, s: T) -> Self {
        self.request_builder = self.request_builder.json(s);
        self
    }
    pub fn post(mut self) -> Self {
        self.request_builder = self.request_builder.post();
        self
    }
    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.request_builder = self.request_builder.header(key, value);
        self
    }
    pub fn bearer_auth(mut self, token: &str) -> Self {
        self.request_builder = self.request_builder.bearer_auth(token);
        self
    }
    pub fn connect_proxy(self, proxy_url: &str) -> Result<SseSubscriber> {
        let proxy_url = Url::from_str(proxy_url)
            .map_err(|e| SseSubscriberError::InvalidUrlError(e.to_string()))?;

        // CONNECT method
        let request = self.request_builder.connect_request();
        // socket for connect proxy
        let mut socket = TcpStream::connect(proxy_url.to_addr_str())
            .map_err(|e| SseSubscriberError::TcpStreamConnectionError(e.to_string()))?;
        // connect to proxy
        socket.write_all(request.bytes()).unwrap();
        let mut buf = vec![0; 4096];
        let request = self.request_builder.build();
        let url = request.url();
        while socket.read(&mut buf).unwrap() > 0 {
            let proxy_response = String::from_utf8(buf.clone()).unwrap();
            debug!(proxy_response);
            if proxy_response.contains("Established") {
                let client = Self::make_client(self.root_store, url);
                return Ok(SseSubscriber {
                    client,
                    tcp_stream: socket,
                    req: request,
                });
            }
        }
        Err(SseSubscriberError::ProxyConnectionError(
            "proxy connection error".to_string(),
        ))
    }

    pub fn build(self) -> SseSubscriber {
        let request = self.request_builder.build();
        let client = Self::make_client(self.root_store, request.url());
        let tcp_stream = TcpStream::connect(request.url().to_addr_str())
            .map_err(|e| SseSubscriberError::TcpStreamConnectionError(e.to_string()))
            .unwrap();
        SseSubscriber {
            client,
            tcp_stream,
            req: request,
        }
    }
    fn default_ca() -> rustls::RootCertStore {
        let mut root_store = rustls::RootCertStore::empty();
        root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
            rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        }));
        root_store
    }
    fn make_client(root_store: RootCertStore, url: &Url) -> ClientConnection {
        let ip = url.host().try_into().unwrap();
        let config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store.clone())
            .with_no_client_auth();
        ClientConnection::new(Arc::new(config), ip).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufRead;

    use super::*;
    #[test]
    #[ignore = "dockerを利用したproxyのテスト"]
    fn test_connection_proxy() {
        let mut subscriber = SubscriberBuilder::new("https://www.google.com")
            .connect_proxy("http://localhost:8080")
            .unwrap();

        let mut reader = subscriber.subscribe_stream().unwrap();
        let mut buf = String::new();
        while reader.read_line(&mut buf).unwrap() > 0 {
            if buf.contains("OK") {
                assert!(true);
                return;
            }
            buf.clear();
        }
        assert!(false);
    }
}
