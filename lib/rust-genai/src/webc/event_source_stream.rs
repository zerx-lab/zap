use crate::error::BoxError;
use crate::webc::WebStream;
use futures::Stream;
use reqwest::RequestBuilder;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Simple EventSource stream implementation that uses WebStream as a foundation.
pub struct EventSourceStream {
	inner: WebStream,
	opened: bool,
}

#[derive(Debug)]
pub enum Event {
	Open,
	Message(Message),
}

#[derive(Debug)]
pub struct Message {
	pub event: String,
	pub data: String,
}

impl EventSourceStream {
	pub fn new(reqwest_builder: RequestBuilder) -> Self {
		// Standard EventSource uses \n\n as event separator
		Self {
			inner: WebStream::new_with_delimiter(reqwest_builder, "\n\n"),
			opened: false,
		}
	}
}

impl Stream for EventSourceStream {
	type Item = Result<Event, BoxError>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();

		// -- 1. Handle initial "Open" event
		if !this.opened {
			this.opened = true;
			return Poll::Ready(Some(Ok(Event::Open)));
		}

		// -- 2. Poll the inner WebStream for next event block
		loop {
			let nx = Pin::new(&mut this.inner).poll_next(cx);

			match nx {
				Poll::Ready(Some(Ok(raw_event))) => {
					let mut event = "message".to_string();
					let mut data = String::new();
					for line in raw_event.lines() {
						let line = line.trim();
						// Skip empty lines or comments (starting with :)
						if line.is_empty() || line.starts_with(':') {
							continue;
						}

						if let Some(e) = line.strip_prefix("event:") {
							event = e.trim().to_string();
						} else if let Some(d) = line.strip_prefix("data:") {
							if !data.is_empty() {
								data.push('\n');
							}
							data.push_str(d.trim());
						}
					}

					// If no data found in this block, poll for the next one
					if data.is_empty() {
						continue;
					}

					return Poll::Ready(Some(Ok(Event::Message(Message { event, data }))));
				}
				Poll::Ready(Some(Err(e))) => {
					return Poll::Ready(Some(Err(e)));
				}
				Poll::Ready(None) => return Poll::Ready(None),
				Poll::Pending => return Poll::Pending,
			}
		}
	}
}
