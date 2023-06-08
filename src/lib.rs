use std::fmt::{Debug, Display};

use event_handler::SseHandler;
use response::SseResponse;
use subscriber::SubscriberBuilder;

mod event_handler;
mod request_builder;
mod response;
mod subscriber;
mod url;

#[derive(Debug, Clone, Copy)]
pub enum SseResult<T> {
    Finished(T),
    Continue,
    Retry,
}
pub trait EventHandler<T> {
    type Err: std::error::Error;
    fn handle(&self, event: &str) -> std::result::Result<SseResult<T>, Self::Err>;
    fn finished(&self) -> std::result::Result<SseResult<T>, Self::Err>;
}
pub trait ErrorHandler<T> {
    type Err: std::error::Error;
    fn catch(&self, error: SseHandlerError) -> std::result::Result<SseResult<T>, Self::Err>;
}

pub struct SseClient<Event, Err, T>
where
    Event: EventHandler<T>,
    Err: ErrorHandler<T>,
{
    proxy_url: Option<String>,
    builder: SubscriberBuilder,
    handler: SseHandler<Event, Err, T>,
}

#[macro_export]
macro_rules! debug {
    ($args:ident) => {
        if let Ok(_) = std::env::var("RUST_LOG") {
            println!("{} = {:#?}", stringify!($args), $args)
        }
    };
}
impl<Event, Err, T> SseClient<Event, Err, T>
where
    Event: EventHandler<T>,
    Err: ErrorHandler<T>,
{
    pub fn new(url: &str, event_handler: Event, error_handler: Err) -> Result<Self> {
        let builder = SubscriberBuilder::new(url);
        let handler = event_handler::SseHandler::new(event_handler, error_handler);
        let mut this = Self {
            builder,
            handler,
            proxy_url: None,
        };

        match (std::env::var("HTTP_PROXY"), std::env::var("HTTPS_PROXY")) {
            (Ok(http_proxy), _) => this.proxy_url = Some(http_proxy),
            (_, Ok(https_proxy)) => this.proxy_url = Some(https_proxy),
            _ => {}
        };
        Ok(this)
    }
    pub fn set_proxy_url(mut self, proxy_url: &str) -> Self {
        self.proxy_url = Some(proxy_url.to_string());
        self
    }
    pub fn bearer_auth(self, token: &str) -> Self {
        let builder = self.builder.bearer_auth(token);
        Self { builder, ..self }
    }
    pub fn post(self) -> Self {
        let builder = self.builder.post();
        Self { builder, ..self }
    }
    pub fn header(self, key: &str, value: &str) -> Self {
        let builder = self.builder.header(key, value);
        Self { builder, ..self }
    }
    pub fn json<S: serde::Serialize>(self, json: S) -> Self {
        let builder = self.builder.json(json);
        Self { builder, ..self }
    }
    pub fn handle_event(self) -> Result<SseResult<T>> {
        let mut subscriber = if let Some(proxy_url) = self.proxy_url {
            self.builder
                .connect_proxy(proxy_url.as_str())
                .map_err(|e| {
                    SseClientError::SseHandlerError(SseHandlerError::SubscriberConstructionError {
                        message: format!("Failed to connect proxy: {}", e),
                        url: proxy_url,
                    })
                })?
        } else {
            self.builder.build()
        };
        let reader = subscriber
            .subscribe_stream()
            .map_err(|e| SseClientError::SseSubscriberError(e))?;
        self.handler
            .handle_event(reader)
            .map_err(|e| SseClientError::SseHandlerError(e))
    }
}
impl<Event, T> SseClient<Event, NonCaughtError, T>
where
    Event: EventHandler<T>,
{
    pub fn without_error_handlers(url: &str, event_handler: Event) -> Result<Self> {
        Self::new(url, event_handler, NonCaughtError {})
    }
}
pub struct NonCaughtError {}
impl<T> ErrorHandler<T> for NonCaughtError {
    type Err = SseHandlerError;
    fn catch(&self, error: SseHandlerError) -> std::result::Result<SseResult<T>, Self::Err> {
        Err(error)
    }
}

#[derive(Debug)]
pub enum SseClientError {
    SseHandlerError(SseHandlerError),
    SseSubscriberError(subscriber::SseSubscriberError),
}
impl Display for SseClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SseClientError::SseHandlerError(e) => {
                write!(f, "SseClientError::SseHandlerError({})", e)
            }
            SseClientError::SseSubscriberError(e) => {
                write!(f, "SseClientError::SseSubscriberError({})", e)
            }
        }
    }
}
impl std::error::Error for SseClientError {}

type Result<T> = std::result::Result<T, SseClientError>;

#[derive(Debug)]
pub enum SseHandlerError {
    InvalidResponseLineError {
        message: String,
        line: String,
    },
    ReadLineError {
        message: String,
        read_line: String,
    },
    HttpResponseError {
        message: String,
        read_line: String,
        response: SseResponse,
    },
    SubscriberConstructionError {
        message: String,
        url: String,
    },
    SubscribeRequestError {
        message: String,
    },
    SubscribeResponseError {
        message: String,
    },
    UserError {
        message: String,
    },
    NonCaughtRequestError {
        message: String,
    },
    NonCaughtResponseError {
        message: String,
    },
}
impl Display for SseHandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SseHandlerError::SubscriberConstructionError { message, url } => {
                write!(
                    f,
                    "SseHandlerError::SubscriberConstructionError{{message:{},url:{}}}",
                    message, url
                )
            }
            Self::SubscribeRequestError { message } => {
                write!(
                    f,
                    "SseHandlerError::SubscribeRequestError{{message:{}}}",
                    message,
                )
            }
            Self::SubscribeResponseError { message } => {
                write!(
                    f,
                    "SseHandlerError::SubscribeResponseError{{message:{}}}",
                    message,
                )
            }
            Self::NonCaughtRequestError { message } => {
                write!(
                    f,
                    "SseHandlerError::NonCaughtRequestError{{message:{:?}}}",
                    message
                )
            }
            Self::NonCaughtResponseError { message } => {
                write!(
                    f,
                    "SseHandlerError::NonCaughtResponseError{{message:{}}}",
                    message
                )
            }
            Self::ReadLineError { read_line, message } => {
                write!(
                    f,
                    "SseHandlerError::ReadLineError{{message:{},read_line:{}}}",
                    message, read_line,
                )
            }
            Self::InvalidResponseLineError { message, line } => {
                write!(
                    f,
                    "SseHandlerError::InvalidResponseLineError{{message:{},line:{}}}",
                    message, line,
                )
            }
            Self::UserError { message } => {
                write!(f, "SseHandlerError::UserError{{message:{}}}", message,)
            }
            Self::HttpResponseError {
                message,
                read_line,
                response,
            } => {
                write!(
                    f,
                    "SseHandlerError::HttpResponseError{{message:{},read_line:{},response:{:?}}}",
                    message, read_line, response
                )
            }
        }
    }
}
impl std::error::Error for SseHandlerError {}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::EventHandler;

    #[test]
    #[ignore = "requires a valid token"]
    fn proxy_gpt_test() {
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
        #[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
        pub struct Chat {
            pub choices: Vec<ChatChoices>,
            pub created: usize,
            pub id: String,
            pub model: String,
            pub object: String,
        }
        #[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
        pub struct ChatChoices {
            pub delta: ChatChoicesDelta,
            pub finish_reason: serde_json::Value,
            pub index: usize,
        }

        #[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
        pub struct ChatChoicesDelta {
            pub content: Option<String>,
        }

        struct Handler {}
        impl EventHandler<()> for Handler {
            type Err = std::io::Error;
            fn handle(&self, event: &str) -> std::result::Result<SseResult<()>, Self::Err> {
                let chat = serde_json::from_str::<Chat>(event);
                match chat {
                    Ok(chat) => {
                        if let Some(choice) = chat.choices.first() {
                            if let Some(content) = &choice.delta.content {
                                println!("{}", content);
                            }
                        }
                    }
                    Err(e) => {
                        if event == "[DONE]" {
                            return Ok(SseResult::Finished(()));
                        }
                        println!("{:?}", e);
                    }
                }
                Ok(SseResult::Continue)
            }
            fn finished(&self) -> std::result::Result<SseResult<()>, Self::Err> {
                Ok(SseResult::Finished(()))
            }
        }

        struct ErrHandler {
            count: RefCell<usize>,
        }
        impl ErrorHandler<()> for ErrHandler {
            type Err = std::io::Error;
            fn catch(
                &self,
                error: SseHandlerError,
            ) -> std::result::Result<SseResult<()>, Self::Err> {
                println!("{:?}", error);
                if *self.count.borrow_mut() + 1 > 3 {
                    return Ok(SseResult::Finished(()));
                }
                println!("retry ");
                *self.count.borrow_mut() += 1;
                Ok(SseResult::Retry)
            }
        }

        let url = "https://api.openai.com/v1/chat/completions";
        let message = String::from("hello");
        let result = SseClient::new(
            url,
            Handler {},
            ErrHandler {
                count: RefCell::new(0),
            },
        )
        .unwrap()
        .set_proxy_url("http://localhost:8080")
        .bearer_auth(std::env::var("OPENAI_API_KEY").unwrap().as_str())
        .post()
        .json(ChatRequest {
            stream: true,
            model: OpenAIModel::Gpt3Dot5Turbo,
            messages: vec![Message {
                role: Role::User,
                content: message,
            }],
        })
        .handle_event()
        .unwrap();
        println!("{:#?}", result);
        assert!(true);
    }
}
