use std::{net::TcpStream, sync::Arc};

use rustls::{ClientConfig, ClientConnection};

use crate::url::Url;

#[derive(Debug)]
pub struct SseClient {
    url: Url,
    client: ClientConnection,
    tcp_stream: TcpStream,
}
pub enum SseClientError {
    InvalidUrlError(String),
    ClientConnectionError(String),
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
            url,
            client,
            tcp_stream: socket,
        })
    }
    //pub fn post(&mut self)
}
