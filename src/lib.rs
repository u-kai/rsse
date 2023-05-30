use std::fmt::Display;

use event_handler::SseHandler;
use request_builder::{Request, RequestBuilder};
use response::SseResponse;
use subscriber::SseSubscriber;

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
    subscriber: SseSubscriber,
    handler: SseHandler<Event, Err, T>,
    request_builder: RequestBuilder,
}

impl<Event, Err, T> SseClient<Event, Err, T>
where
    Event: EventHandler<T>,
    Err: ErrorHandler<T>,
{
    pub fn new(url: &str, event_handler: Event, error_handler: Err) -> Result<Self> {
        let subscriber = if std::env::var("HTTP_PROXY").is_ok() {
            SseSubscriber::with_proxy(std::env::var("HTTP_PROXY").unwrap().as_str(), url)
                .map_err(|e| SseClientError::SseSubscriberError(e))?
        } else {
            SseSubscriber::default(url).map_err(|e| SseClientError::SseSubscriberError(e))?
        };
        let handler = event_handler::SseHandler::new(event_handler, error_handler);
        let request_builder = RequestBuilder::new(url);
        Ok(Self {
            subscriber,
            handler,
            request_builder,
        })
    }
    pub fn bearer_auth(self, token: &str) -> Self {
        let request_builder = self.request_builder.bearer_auth(token);
        Self {
            request_builder,
            ..self
        }
    }
    pub fn post(self) -> Self {
        let request_builder = self.request_builder.post();
        Self {
            request_builder,
            ..self
        }
    }
    pub fn json<S: serde::Serialize>(self, json: S) -> Self {
        let request_builder = self.request_builder.json(json);
        Self {
            request_builder,
            ..self
        }
    }
    pub fn handle_event(mut self) -> Result<SseResult<T>> {
        let request = self.request_builder.build();
        let reader = self
            .subscriber
            .subscribe_stream(&request)
            .map_err(|e| SseClientError::SseSubscriberError(e))?;
        self.handler
            .handle_event(reader, request)
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
        request: Request,
    },
    HttpResponseError {
        message: String,
        read_line: String,
        request: Request,
        response: SseResponse,
    },
    SubscriberConstructionError {
        message: String,
        url: String,
    },
    SubscribeRequestError {
        message: String,
        request: Request,
    },
    SubscribeResponseError {
        message: String,
        request: Request,
    },
    UserError {
        message: String,
    },
    NonCaughtRequestError {
        request: Request,
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
            Self::SubscribeRequestError { message, request } => {
                write!(
                    f,
                    "SseHandlerError::SubscribeRequestError{{message:{},request:{:?}}}",
                    message, request
                )
            }
            Self::SubscribeResponseError { message, request } => {
                write!(
                    f,
                    "SseHandlerError::SubscribeResponseError{{message:{},request:{:?}}}",
                    message, request
                )
            }
            Self::NonCaughtRequestError { request } => {
                write!(
                    f,
                    "SseHandlerError::NonCaughtRequestError{{request:{:?}}}",
                    request
                )
            }
            Self::NonCaughtResponseError { message } => {
                write!(
                    f,
                    "SseHandlerError::NonCaughtResponseError{{message:{}}}",
                    message
                )
            }
            Self::ReadLineError {
                read_line,
                message,
                request,
            } => {
                write!(
                    f,
                    "SseHandlerError::ReadLineError{{message:{},read_line:{},request:{:?}}}",
                    message, read_line, request
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
                request,
                response,
            } => {
                write!(
                    f,
                    "SseHandlerError::HttpResponseError{{message:{},read_line:{},request:{:?},response:{:?}}}",
                    message, read_line, request, response
                )
            }
        }
    }
}
impl std::error::Error for SseHandlerError {}
