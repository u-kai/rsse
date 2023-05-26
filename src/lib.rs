use event_handler::SseResult;
use request_builder::RequestBuilder;
use subscriber::SseSubscriber;

pub mod event_handler;
pub mod request_builder;
mod response;
pub mod subscriber;
mod url;

pub struct SseClient<Event, Err>
where
    Event: event_handler::EventHandler,
    Err: event_handler::ErrorHandler,
{
    subscriber: SseSubscriber,
    handler: event_handler::SseHandler<Event, Err>,
    request_builder: RequestBuilder,
}

impl<Event, Err> SseClient<Event, Err>
where
    Event: event_handler::EventHandler,
    Err: event_handler::ErrorHandler,
{
    pub fn new(url: &str, event_handler: Event, error_handler: Err) -> Result<Self> {
        let subscriber =
            SseSubscriber::default(url).map_err(|e| SseClientError::SseSubscriberError(e))?;
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
    pub fn json<T: serde::Serialize>(self, json: T) -> Self {
        let request_builder = self.request_builder.json(json);
        Self {
            request_builder,
            ..self
        }
    }
    pub fn handle_event(mut self) -> Result<SseResult> {
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

#[derive(Debug)]
pub enum SseClientError {
    //RequestBuilderError(request_builder::RequestBuilderError),
    SseHandlerError(event_handler::SseHandlerError),
    SseSubscriberError(subscriber::SseSubscriberError),
}

type Result<T> = std::result::Result<T, SseClientError>;
