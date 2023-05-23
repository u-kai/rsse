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
pub trait RequestErrorHandler {
    type Err: std::error::Error;
    fn catch(&self, bad_request: &Request) -> std::result::Result<(), Self::Err>;
}
pub trait ResponseErrorHandler {
    type Err: std::error::Error;
    fn catch(&self, error: &str) -> std::result::Result<(), Self::Err>;
}
pub struct SseHandler<Event, ReqErr, ResErr>
where
    Event: EventHandler,
    ReqErr: RequestErrorHandler,
    ResErr: ResponseErrorHandler,
{
    subscriber: SseSubscriber,
    event_handler: Event,
    request_error_handler: Option<ReqErr>,
    response_error_handler: Option<ResErr>,
}
pub struct NonReqErr {}
impl RequestErrorHandler for NonReqErr {
    type Err = SseHandlerError;
    fn catch(&self, bad_request: &Request) -> std::result::Result<(), Self::Err> {
        Err(SseHandlerError::NonCaughtRequestError {
            request: bad_request.clone(),
        })
    }
}
pub struct NonResErr {}
impl ResponseErrorHandler for NonResErr {
    type Err = SseHandlerError;
    fn catch(&self, error: &str) -> std::result::Result<(), Self::Err> {
        Err(SseHandlerError::NonCaughtResponseError {
            message: error.to_string(),
        })
    }
}

pub type Result<T> = std::result::Result<T, SseHandlerError>;
impl<Event> SseHandler<Event, NonReqErr, NonResErr>
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
            request_error_handler: Some(NonReqErr {}),
            response_error_handler: Some(NonResErr {}),
        })
    }
}
impl<Event, ReqErr, ResErr> SseHandler<Event, ReqErr, ResErr>
where
    Event: EventHandler,
    ReqErr: RequestErrorHandler,
    ResErr: ResponseErrorHandler,
{
    pub fn new(url: &str, event_handler: Event) -> Result<Self> {
        Ok(Self {
            subscriber: SseSubscriber::default(url).map_err(|e| {
                SseHandlerError::SubscriberConstructionError {
                    message: e.to_string(),
                    url: url.to_string(),
                }
            })?,
            event_handler,
            request_error_handler: None,
            response_error_handler: None,
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
                            return self.catch_io_error(&request, e);
                        }
                    }
                    match response_store.evaluate_lines(line.as_str()) {
                        Ok(response) => {
                            if response.is_error() || read_len < 5 {
                                return self.catch_response_error(
                                    &response,
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
                                .map_err(|e| SseHandlerError::SubscribeRequestUserError {
                                    message: e.to_string(),
                                    request: request.clone(),
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
    fn catch_invalid_response_line_error(&self, line: &str, e: SseResponseError) -> Result<()> {
        match self.response_error_handler {
            Some(ref handler) => {
                handler
                    .catch(line)
                    .map_err(|e| SseHandlerError::InvalidResponseLineError {
                        message: e.to_string(),
                        line: line.to_string(),
                    })?;
                Ok(())
            }
            None => {
                return Err(SseHandlerError::InvalidResponseLineError {
                    message: e.to_string(),
                    line: line.to_string(),
                });
            }
        }
    }
    fn catch_response_error(
        &self,
        response: &SseResponse,
        request: &Request,
        line: &str,
    ) -> Result<()> {
        match self.response_error_handler {
            Some(ref handler) => {
                handler
                    .catch(line)
                    .map_err(|e| SseHandlerError::SubscribeResponseUserError {
                        message: e.to_string(),
                        request: request.clone(),
                        response: response.clone(),
                    })?;
                Ok(())
            }
            None => {
                return Err(SseHandlerError::SubscribeResponseError {
                    message: format!("invalid response line: {}", line),
                    request: request.clone(),
                });
            }
        }
    }
    fn catch_io_error(&self, request: &Request, e: std::io::Error) -> Result<()> {
        match self.response_error_handler {
            Some(ref handler) => {
                handler.catch(e.to_string().as_str()).map_err(|e| {
                    SseHandlerError::ReadLineError {
                        message: e.to_string(),
                        request: request.clone(),
                    }
                })?;
                Ok(())
            }
            None => {
                return Err(SseHandlerError::SubscribeResponseError {
                    message: e.to_string(),
                    request: request.clone(),
                });
            }
        }
    }
    fn catch_request_error(&self, request: &Request, e: SseSubscriberError) -> Result<()> {
        match self.request_error_handler {
            Some(ref handler) => {
                handler.catch(&request).map_err(|e| {
                    SseHandlerError::SubscribeRequestUserError {
                        message: e.to_string(),
                        request: request.clone(),
                    }
                })?;
                Ok(())
            }
            None => {
                return Err(SseHandlerError::SubscribeRequestError {
                    message: e.to_string(),
                    request: request.clone(),
                });
            }
        }
    }
    pub fn set_request_error_handler(&mut self, handler: ReqErr) {
        self.request_error_handler = Some(handler);
    }
    pub fn set_response_error_handler(&mut self, handler: ResErr) {
        self.response_error_handler = Some(handler);
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
        request: Request,
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
    SubscribeRequestUserError {
        message: String,
        request: Request,
    },
    SubscribeResponseUserError {
        message: String,
        response: SseResponse,
        request: Request,
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
            Self::SubscribeRequestUserError { message, request } => {
                write!(
                    f,
                    "SseHandlerError::SubscribeRequestUserError{{message:{},request:{:?}}}",
                    message, request
                )
            }
            Self::SubscribeResponseUserError {
                message,
                response,
                request,
            } => {
                write!(
                    f,
                    "SseHandlerError::SubscribeResponseUserError{{message:{},response:{:?},request:{:?}}}",
                    message, response, request
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
            Self::ReadLineError { message, request } => {
                write!(
                    f,
                    "SseHandlerError::ReadLineError{{message:{},request:{:?}}}",
                    message, request
                )
            }
            Self::InvalidResponseLineError { message, line } => {
                write!(
                    f,
                    "SseHandlerError::InvalidResponseLineError{{message:{},line:{:?}}}",
                    message, line
                )
            }
        }
    }
}
impl std::error::Error for SseHandlerError {}
