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
pub struct SseTlsConnector {}

impl SseTlsConnector {
    pub fn new() -> Self {
        Self {}
    }
    fn default_client_connection(url: &Url) -> rustls::ClientConnection {
        let ip = url.host().try_into().unwrap();
        let config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(Self::default_root_store())
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
}

impl SseConnector for SseTlsConnector {
    type Socket = TlsSocket;
    fn connect(&mut self, req: &Request) -> Result<SseConnection<Self::Socket>> {
        let url = req.url();
        let tcp_stream =
            TcpStream::connect(url.to_addr_str()).map_err(|e| SseConnectionError::IOError(e))?;
        let client = Self::default_client_connection(url);
        let mut tls_stream = rustls::StreamOwned::new(client, tcp_stream);
        tls_stream
            .write_all(req.bytes())
            .map_err(|e| SseConnectionError::IOError(e))?;
        let socket = TlsSocket::new(tls_stream).map_err(|e| SseConnectionError::IOError(e))?;
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
}

#[derive(Debug)]
pub struct TlsSocket {
    reader: BufReader<rustls::StreamOwned<ClientConnection, TcpStream>>,
}
impl TlsSocket {
    pub fn new(
        tls_stream: rustls::StreamOwned<ClientConnection, TcpStream>,
    ) -> std::result::Result<Self, std::io::Error> {
        let reader = BufReader::new(tls_stream);
        Ok(Self { reader })
    }
}
impl Socket for TlsSocket {
    fn read_line(&mut self) -> std::result::Result<Option<String>, std::io::Error> {
        let mut buf = String::new();
        let size = self.reader.read_line(&mut buf)?;
        if size == 0 {
            Ok(None)
        } else {
            Ok(Some(buf))
        }
    }
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
        sse::connector::{
            chatgpt::{chatgpt_key, evaluate_chatgpt_response, message, ChatGptRes, URL},
            fakes::FakeTcpConnection,
        },
    };

    use super::*;
    #[test]
    #[ignore = "実際の通信を行うため"]
    fn chatgptにtlsで通信する() {
        let req = RequestBuilder::new(URL)
            .post()
            .bearer_auth(&chatgpt_key())
            .json(message("hello"))
            .build();
        let mut tls_connector = SseTlsConnector::new();
        let mut conn = tls_connector.connect(&req).unwrap();
        let mut result = conn.read();
        let mut flag = false;
        while let Ok(res) = &result {
            let chatgpt_res = evaluate_chatgpt_response(res);
            match chatgpt_res {
                ChatGptRes::Done => {
                    println!("done");
                    flag = true;
                    break;
                }
                ChatGptRes::Data(data) => {
                    println!("progress: {}", data);
                    result = conn.read();
                    continue;
                }
                ChatGptRes::Err => {
                    flag = false;
                    println!("err");
                    break;
                }
            }
        }
        assert!(flag);
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
    }
}
#[cfg(test)]
pub mod chatgpt {
    use crate::sse::{response::SseResponse, subscriber::HandleProgress};

    use super::ConnectedSseResponse;

    pub struct GptHandler {
        flag: bool,
    }
    impl GptHandler {
        pub fn new() -> Self {
            GptHandler { flag: false }
        }
        pub fn is_success(&self) -> bool {
            self.flag
        }
    }
    impl crate::sse::subscriber::SseMutHandler<(), ()> for GptHandler {
        fn handle(&mut self, res: SseResponse) -> HandleProgress<()> {
            let res = evaluate_chatgpt_sse_response(&res);
            match res {
                ChatGptRes::Done => {
                    println!("done");
                    self.flag = true;
                    HandleProgress::Done
                }
                ChatGptRes::Data(data) => {
                    println!("progress: {}", data);
                    HandleProgress::Progress
                }
                ChatGptRes::Err => {
                    self.flag = false;
                    println!("error");
                    HandleProgress::Err(())
                }
            }
        }
        fn result(&self) -> std::result::Result<(), ()> {
            Ok(())
        }
    }
    pub enum ChatGptRes {
        Done,
        Data(String),
        Err,
    }
    pub fn evaluate_chatgpt_sse_response(res: &SseResponse) -> ChatGptRes {
        match res {
            SseResponse::Retry(_) => ChatGptRes::Err,
            SseResponse::Data(data) => {
                if "[DONE]".contains(data) {
                    ChatGptRes::Done
                } else {
                    ChatGptRes::Data(data.to_string())
                }
            }
            SseResponse::Id(id) => ChatGptRes::Data(id.to_string()),
            SseResponse::Event(event) => ChatGptRes::Data(event.to_string()),
            _ => ChatGptRes::Err,
        }
    }
    pub fn evaluate_chatgpt_response(res: &ConnectedSseResponse) -> ChatGptRes {
        match res {
            ConnectedSseResponse::Done => ChatGptRes::Done,
            ConnectedSseResponse::Progress(SseResponse::Data(data)) => {
                if "[DONE]".contains(data) {
                    ChatGptRes::Done
                } else {
                    ChatGptRes::Data(data.to_string())
                }
            }
            _ => ChatGptRes::Err,
        }
    }

    pub const URL: &'static str = "https://api.openai.com/v1/chat/completions";
    pub fn message(mes: &str) -> ChatRequest {
        ChatRequest {
            model: OpenAIModel::Gpt3Dot5Turbo,
            messages: vec![Message {
                role: Role::User,
                content: mes.to_string(),
            }],
            stream: true,
        }
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct ChatRequest {
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
    pub fn chatgpt_key() -> String {
        std::env::var("OPENAI_API_KEY").unwrap()
    }
}
