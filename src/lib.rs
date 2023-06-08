use std::fmt::{Debug, Display};

use event_handler::SseHandler;
use response::SseResponse;
use subscriber::SubscriberBuilder;

mod connector;
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
        Ok(Self { builder, handler })
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
    pub fn handle_event(mut self) -> Result<SseResult<T>> {
        let mut subscriber = self.builder.build();
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
