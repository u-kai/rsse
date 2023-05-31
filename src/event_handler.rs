use std::{
    io::{BufRead, BufReader, Read},
    marker::PhantomData,
};

use crate::{
    debug,
    request_builder::Request,
    response::{SseResponse, SseResponseError, SseResponseStore},
    ErrorHandler, EventHandler, SseHandlerError, SseResult,
};

pub struct SseHandler<Event, Er, T>
where
    Event: EventHandler<T>,
    Er: ErrorHandler<T>,
{
    event_handler: Event,
    error_handler: Er,
    _p: PhantomData<T>,
}

pub type Result<T> = std::result::Result<T, SseHandlerError>;
impl<Event, Er, T> SseHandler<Event, Er, T>
where
    Event: EventHandler<T>,
    Er: ErrorHandler<T>,
{
    pub fn new(event_handler: Event, error_handler: Er) -> Self {
        Self {
            event_handler,
            error_handler,
            _p: PhantomData {},
        }
    }
    pub fn handle_event<R: Read>(
        &self,
        mut reader: BufReader<R>,
        request: Request,
    ) -> Result<SseResult<T>> {
        let mut response_store = SseResponseStore::new();
        let mut read_len = 1;
        let mut line = String::new();
        while read_len > 0 {
            match reader.read_line(&mut line) {
                Ok(len) => read_len = len,
                Err(e) => {
                    return self.return_or_retry(
                        self.catch_io_error(line.as_str(), &request, e),
                        reader,
                        request,
                    );
                }
            }
            let response_line = line.as_str();
            debug!(response_line);
            match response_store.evaluate_lines(line.as_str()) {
                Ok(response) => {
                    if response.is_error() && read_len <= 5 {
                        let result =
                            self.catch_http_response_error(response, &request, line.as_str());
                        return self.return_or_retry(result, reader, request);
                    }
                    let Some(event) = response.new_event() else {
                        line.clear();
                        continue;
                    };
                    let result = self.event_handler.handle(event).map_err(|e| {
                        SseHandlerError::UserError {
                            message: e.to_string(),
                        }
                    })?;
                    match result {
                        SseResult::Finished(a) => {
                            return Ok(SseResult::Finished(a));
                        }
                        SseResult::Continue => {
                            line.clear();
                            continue;
                        }
                        SseResult::Retry => {
                            todo!()
                        }
                    };
                }
                Err(e) => {
                    let result = self.catch_invalid_response_line_error(line.as_str(), e);
                    return self.return_or_retry(result, reader, request);
                }
            }
        }
        //Ok(SseResult::Finished)
        self.event_handler
            .finished()
            .map_err(|e| SseHandlerError::UserError {
                message: e.to_string(),
            })
    }
    fn return_or_retry<R: Read>(
        &self,
        result: Result<SseResult<T>>,
        reader: BufReader<R>,
        request: Request,
    ) -> Result<SseResult<T>> {
        match result {
            Ok(SseResult::Retry) => self.handle_event(reader, request),
            _ => result,
        }
    }
    fn catch_http_response_error(
        &self,
        response: &SseResponse,
        request: &Request,
        line: &str,
    ) -> Result<SseResult<T>> {
        let error = SseHandlerError::HttpResponseError {
            message: format!("http response error"),
            read_line: line.to_string(),
            request: request.clone(),
            response: response.clone(),
        };
        self.catch(error)
    }
    fn catch_invalid_response_line_error(
        &self,
        line: &str,
        e: SseResponseError,
    ) -> Result<SseResult<T>> {
        let error = SseHandlerError::InvalidResponseLineError {
            message: e.to_string(),
            line: line.to_string(),
        };
        self.catch(error)
    }
    fn catch_io_error(
        &self,
        read_line: &str,
        request: &Request,
        e: std::io::Error,
    ) -> Result<SseResult<T>> {
        let error = SseHandlerError::ReadLineError {
            message: e.to_string(),
            read_line: read_line.to_string(),
            request: request.clone(),
        };
        self.catch(error)
    }
    fn catch(&self, error: SseHandlerError) -> Result<SseResult<T>> {
        self.error_handler
            .catch(error)
            .map_err(|e| SseHandlerError::UserError {
                message: e.to_string(),
            })
    }
}
