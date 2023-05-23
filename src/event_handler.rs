use std::{fmt::Display, io::BufRead};

use crate::{
    request_builder::Request,
    response::{SseResponse, SseResponseError, SseResponseStore},
    subscriber::{SseSubscriber, SseSubscriberError},
};

pub struct SseFinished(bool);
impl SseFinished {
    pub fn finish() -> Self {
        Self(true)
    }
    pub fn r#continue() -> Self {
        Self(false)
    }
}
impl Into<SseFinished> for bool {
    fn into(self) -> SseFinished {
        SseFinished(self)
    }
}
pub trait EventHandler {
    type Err: std::error::Error;
    fn handle(&self, event: &str) -> std::result::Result<SseFinished, Self::Err>;
}
pub trait ErrorHandler {
    type Err: std::error::Error;
    fn catch(&self, error: SseHandlerError) -> std::result::Result<(), Self::Err>;
}
pub struct SseHandler<Event, Er>
where
    Event: EventHandler,
    Er: ErrorHandler,
{
    subscriber: SseSubscriber,
    event_handler: Event,
    error_handler: Er,
}
pub struct NonCaughtError {}
impl ErrorHandler for NonCaughtError {
    type Err = SseHandlerError;
    fn catch(&self, error: SseHandlerError) -> std::result::Result<(), Self::Err> {
        Err(error)
    }
}

pub type Result<T> = std::result::Result<T, SseHandlerError>;
impl<Event> SseHandler<Event, NonCaughtError>
where
    Event: EventHandler,
{
    pub fn without_error_handlers(url: &str, event_handler: Event) -> Result<Self> {
        Ok(Self {
            subscriber: SseSubscriber::default(url).map_err(|e| {
                SseHandlerError::SubscriberConstructionError {
                    message: e.to_string(),
                    url: url.to_string(),
                }
            })?,
            event_handler,
            error_handler: NonCaughtError {},
        })
    }
}
impl<Event, Er> SseHandler<Event, Er>
where
    Event: EventHandler,
    Er: ErrorHandler,
{
    pub fn new(url: &str, event_handler: Event, error_handler: Er) -> Result<Self> {
        Ok(Self {
            subscriber: SseSubscriber::default(url).map_err(|e| {
                SseHandlerError::SubscriberConstructionError {
                    message: e.to_string(),
                    url: url.to_string(),
                }
            })?,
            event_handler,
            error_handler,
        })
    }
    pub fn handle_subscribe_event(&mut self, request: Request) -> Result<()> {
        match self.subscriber.subscribe_stream(&request) {
            Ok(mut reader) => {
                let mut response_store = SseResponseStore::new();
                let mut read_len = 1;
                let mut line = String::new();
                while read_len > 0 {
                    match reader.read_line(&mut line) {
                        Ok(len) => read_len = len,
                        Err(e) => {
                            return self.catch_io_error(line.as_str(), &request, e);
                        }
                    }
                    match response_store.evaluate_lines(line.as_str()) {
                        Ok(response) => {
                            if response.is_error() && read_len <= 5 {
                                return self.catch_http_response_error(
                                    response,
                                    &request,
                                    line.as_str(),
                                );
                            }
                            let Some(event) = response.new_event() else {
                                line.clear();
                                continue;
                            };
                            if self
                                .event_handler
                                .handle(event)
                                .map_err(|e| SseHandlerError::SubscribeUserError {
                                    message: e.to_string(),
                                })?
                                .0
                            {
                                return Ok(());
                            };
                        }
                        Err(e) => {
                            return self.catch_invalid_response_line_error(line.as_str(), e);
                        }
                    }
                }
                line.clear();
            }
            Err(e) => {
                return self.catch_request_error(&request, e);
            }
        }
        Ok(())
    }
    fn catch_http_response_error(
        &self,
        response: &SseResponse,
        request: &Request,
        line: &str,
    ) -> Result<()> {
        let error = SseHandlerError::HttpResponseError {
            message: format!("http response error"),
            read_line: line.to_string(),
            request: request.clone(),
            response: response.clone(),
        };
        self.catch(error)
    }
    fn catch_invalid_response_line_error(&self, line: &str, e: SseResponseError) -> Result<()> {
        let error = SseHandlerError::InvalidResponseLineError {
            message: e.to_string(),
            line: line.to_string(),
        };
        self.catch(error)
    }
    fn catch_io_error(&self, read_line: &str, request: &Request, e: std::io::Error) -> Result<()> {
        let error = SseHandlerError::ReadLineError {
            message: e.to_string(),
            read_line: read_line.to_string(),
            request: request.clone(),
        };
        self.catch(error)
    }
    fn catch_request_error(&self, request: &Request, e: SseSubscriberError) -> Result<()> {
        let error = SseHandlerError::SubscribeRequestError {
            message: e.to_string(),
            request: request.clone(),
        };
        self.catch(error)
    }
    fn catch(&self, error: SseHandlerError) -> Result<()> {
        self.error_handler
            .catch(error)
            .map_err(|e| SseHandlerError::SubscribeUserError {
                message: e.to_string(),
            })
    }
}

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
    SubscribeUserError {
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
            Self::SubscribeUserError { message } => {
                write!(
                    f,
                    "SseHandlerError::SubscribeUserError{{message:{}}}",
                    message,
                )
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
