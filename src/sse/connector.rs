use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    sync::Arc,
};

use rustls::{ClientConfig, ClientConnection};

use crate::{
    http::{
        body::HttpBody, header::HttpHeader, response::HttpResponse, status_line::HttpStatusLine,
    },
    request::Request,
    url::Url,
};

use super::response::SseResponse;
pub type Result<T> = std::result::Result<T, SseConnectionError>;
pub struct SseTlsConnector {
    url: Url,
}

impl SseTlsConnector {
    pub fn new(url: impl Into<Url>) -> Self {
        Self { url: url.into() }
    }
}

impl SseConnector for SseTlsConnector {
    type Socket = TlsSocket;
    fn connect(&mut self, req: &Request) -> Result<SseConnection<Self::Socket>> {
        let mut socket = TlsSocket::new(&self.url).map_err(|e| SseConnectionError::IOError(e))?;
        let conn = SseConnection::new(socket);
        Ok(conn)
    }
}

pub trait SseConnector {
    type Socket: Socket;
    fn connect(&mut self, req: &Request) -> Result<SseConnection<Self::Socket>>;
}

pub trait Socket {
    fn read_line(&mut self) -> std::result::Result<Option<String>, std::io::Error>;
    fn write(&mut self, data: &[u8]) -> std::result::Result<(), std::io::Error>;
}

pub struct TlsSocket {
    tls_stream: rustls::StreamOwned<ClientConnection, TcpStream>,
}
impl TlsSocket {
    pub fn new(url: &Url) -> std::result::Result<Self, std::io::Error> {
        let tcp_stream = TcpStream::connect(url.host())?;
        let client = default_client_connection(url);
        let tls_stream = rustls::StreamOwned::new(client, tcp_stream);
        Ok(Self { tls_stream })
    }
}
impl Socket for TlsSocket {
    fn read_line(&mut self) -> std::result::Result<Option<String>, std::io::Error> {
        let mut buf = String::new();
        let mut reader = BufReader::new(&mut self.tls_stream);
        let size = reader.read_line(&mut buf)?;
        if size == 0 {
            Ok(None)
        } else {
            Ok(Some(buf))
        }
    }
    fn write(&mut self, data: &[u8]) -> std::result::Result<(), std::io::Error> {
        self.tls_stream.write(data)?;
        Ok(())
    }
}

fn default_client_connection(url: &Url) -> rustls::ClientConnection {
    let ip = url.host().try_into().unwrap();
    let config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(default_root_store())
        .with_no_client_auth();
    ClientConnection::new(Arc::new(config), ip).unwrap()
}
fn default_root_store() -> rustls::RootCertStore {
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

#[derive(Debug, Clone)]
pub struct SseConnection<S: Socket> {
    conn: S,
}
impl<S: Socket> SseConnection<S> {
    pub fn new(conn: S) -> Self {
        Self { conn }
    }
    pub fn read(&mut self) -> Result<ConnectedSseResponse> {
        while let Some(line) = self
            .conn
            .read_line()
            .map_err(|e| SseConnectionError::IOError(e))?
        {
            if let Ok(http_status) = HttpStatusLine::from_str(&line) {
                if !http_status.is_error() {
                    continue;
                };
                return Err(self.http_error(http_status));
            };
            // sse_response is look like header, so check sse_response first
            if let Ok(sse_response) = SseResponse::from_line(line.as_str()) {
                return Ok(ConnectedSseResponse::Progress(sse_response));
            };
            if let Ok(_header) = HttpHeader::from_line(line.as_str()) {
                continue;
            };
        }
        Ok(ConnectedSseResponse::Done)
    }
    fn http_error(&mut self, http_status: HttpStatusLine) -> SseConnectionError {
        let mut header = HttpHeader::new();
        let mut body = HttpBody::new();
        while let Some(line) = self.conn.read_line().map_or(None, |r| r) {
            if let Ok(add_header) = HttpHeader::from_line(line.as_str()) {
                header.concat(add_header);
                continue;
            };
            let add_body = HttpBody::from_line(line.as_str());
            body.concat(add_body)
        }
        SseConnectionError::HttpError(HttpResponse::new(http_status, header, body))
    }
}
#[derive(Debug, PartialEq, Clone)]
pub enum ConnectedSseResponse {
    Progress(SseResponse),
    Done,
}

#[derive(Debug)]
pub enum SseConnectionError {
    IOError(std::io::Error),
    HttpError(HttpResponse),
}

impl std::fmt::Display for SseConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SseConnectionError::IOError(err) => {
                write!(f, "SseConnectionError: {}", err.to_string())
            }
            Self::HttpError(err) => write!(f, "SseConnectionError: {}", err.to_string()),
        }
    }
}
impl std::error::Error for SseConnectionError {}

#[cfg(test)]
mod tests {

    use crate::{
        http::{body::HttpBody, response::HttpResponse},
        request::RequestBuilder,
        sse::connector::fakes::FakeTcpConnection,
    };

    use super::*;
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct ChatRequest {
        model: OpenAIModel,
        messages: Vec<Message>,
        stream: bool,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct Message {
        role: Role,
        content: String,
    }
    #[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
    pub enum Role {
        User,
        Assistant,
    }
    impl Role {
        fn into_str(&self) -> &'static str {
            match self {
                Self::User => "user",
                Self::Assistant => "assistant",
            }
        }
    }
    impl serde::Serialize for Role {
        fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let role: &str = self.into_str();
            serializer.serialize_str(role)
        }
    }
    #[derive(Debug, Clone, serde::Deserialize)]
    pub enum OpenAIModel {
        Gpt3Dot5Turbo,
    }
    impl serde::Serialize for OpenAIModel {
        fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
        where
            S: serde::ser::Serializer,
        {
            serializer.serialize_str(self.into_str())
        }
    }

    impl OpenAIModel {
        pub fn into_str(&self) -> &'static str {
            match self {
                Self::Gpt3Dot5Turbo => "gpt-3.5-turbo",
            }
        }
    }
    impl Into<&'static str> for OpenAIModel {
        fn into(self) -> &'static str {
            self.into_str()
        }
    }
    #[test]
    #[ignore = "実際の通信を行うため"]
    fn chatgptにtlsで通信する() {
        let url = "https://api.openai.com/v1/chat/completions";
        let req = RequestBuilder::new(url)
            .post()
            .bearer_auth(std::env::var("OPENAI_API_KEY").unwrap().as_str())
            .json(&ChatRequest {
                model: OpenAIModel::Gpt3Dot5Turbo,
                messages: vec![Message {
                    role: Role::User,
                    content: "Hello".to_string(),
                }],
                stream: true,
            })
            .build();
        let mut tls_connector = SseTlsConnector::new(req.url().clone());
        let mut conn = tls_connector.connect(&req).unwrap();
        println!("conn {:#?}", conn.read().unwrap());
        assert!(false);
    }
    #[test]
    fn sse_connectionはデータを接続相手から受け取りsseのレスポンスを返す() {
        let mut fake = FakeTcpConnection::new();
        fake.set_response("HTTP/1.1 200 OK\n\n");
        fake.set_response("Content-Type: text/event-stream\n\n");
        fake.set_response("\n\n");
        fake.set_response("data: Hello, World!\n\n");
        fake.set_response("data: Good Bye World\n\n");

        let mut sut = SseConnection::new(fake);

        let result = sut.read().unwrap();
        assert_eq!(
            result,
            ConnectedSseResponse::Progress(SseResponse::Data("Hello, World!".to_string()))
        );

        let result = sut.read().unwrap();
        assert_eq!(
            result,
            ConnectedSseResponse::Progress(SseResponse::Data("Good Bye World".to_string()))
        );

        let done = sut.read().unwrap();
        assert_eq!(done, ConnectedSseResponse::Done);
    }
    #[test]
    fn http_errorの場合はhttp_responseをそのままerrorに包んで返す() {
        let mut fake = FakeTcpConnection::new();
        fake.set_response("HTTP/1.1 404 Not Found\n\n");
        fake.set_response("Content-Type: text/event-stream\n\n");

        let mut sut = SseConnection::new(fake);
        let Err(SseConnectionError::HttpError(result)) = sut.read() else {
            panic!("expected Err, but got Ok");
        };

        assert_eq!(
            result,
            HttpResponse::new(
                HttpStatusLine::from_str("HTTP/1.1 404 Not Found").unwrap(),
                HttpHeader::from_line("Content-Type: text/event-stream").unwrap(),
                HttpBody::from_line("")
            )
        );
    }
}
#[cfg(test)]
pub(crate) mod fakes {
    use super::{Socket, SseConnection, SseConnectionError};

    pub struct FakeSseConnector {
        connected_times: usize,
        pub connection: SseConnection<FakeTcpConnection>,
    }
    impl FakeSseConnector {
        pub fn new() -> Self {
            Self {
                connection: SseConnection::new(FakeTcpConnection::new()),
                connected_times: 0,
            }
        }
        pub fn set_response(&mut self, response: &str) {
            self.connection.conn.set_response(response);
        }
        pub fn connected_times(&self) -> usize {
            self.connected_times
        }
    }
    impl super::SseConnector for FakeSseConnector {
        type Socket = FakeTcpConnection;
        fn connect(
            &mut self,
            _req: &super::Request,
        ) -> std::result::Result<SseConnection<FakeTcpConnection>, SseConnectionError> {
            self.connected_times += 1;
            Ok(self.connection.clone())
        }
    }
    #[derive(Debug, Clone)]
    pub struct FakeTcpConnection {
        responses: Vec<String>,
    }
    impl FakeTcpConnection {
        pub fn new() -> Self {
            Self {
                responses: Vec::new(),
            }
        }
        pub fn set_response(&mut self, response: &str) {
            self.responses.push(response.to_string());
        }
    }
    impl Socket for FakeTcpConnection {
        fn read_line(&mut self) -> std::result::Result<Option<String>, std::io::Error> {
            if self.responses.is_empty() {
                return Ok(None);
            }
            Ok(Some(self.responses.remove(0)))
        }
        fn write(&mut self, _data: &[u8]) -> std::result::Result<(), std::io::Error> {
            Ok(())
        }
    }
}
