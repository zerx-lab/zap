use bytes::Bytes;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::{Frame, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

type BoxBody = http_body_util::combinators::BoxBody<Bytes, Infallible>;

/// Chunk size for replay responses (matches typical TCP/HTTP chunk boundaries).
const REPLAY_CHUNK_SIZE: usize = 8192;

pub enum Mode {
	Record { backend_url: String, cassette_dir: PathBuf },
	Replay { cassette_dir: PathBuf },
}

pub struct YakbakServer {
	addr: SocketAddr,
	shutdown_tx: Option<oneshot::Sender<()>>,
	join_handle: Option<tokio::task::JoinHandle<()>>,
}

impl YakbakServer {
	pub fn addr(&self) -> SocketAddr {
		self.addr
	}

	/// Base URL, e.g. `http://127.0.0.1:12345/`
	pub fn base_url(&self) -> String {
		format!("http://127.0.0.1:{}/", self.addr.port())
	}

	pub async fn shutdown(&mut self) {
		if let Some(tx) = self.shutdown_tx.take() {
			let _ = tx.send(());
		}
		if let Some(handle) = self.join_handle.take() {
			let _ = handle.await;
		}
	}
}

impl Drop for YakbakServer {
	fn drop(&mut self) {
		if let Some(tx) = self.shutdown_tx.take() {
			let _ = tx.send(());
		}
	}
}

impl YakbakServer {
	pub async fn start(mode: Mode) -> Result<Self, String> {
		let listener = TcpListener::bind("127.0.0.1:0")
			.await
			.map_err(|e| format!("yakbak bind: {e}"))?;
		let addr = listener.local_addr().map_err(|e| format!("yakbak addr: {e}"))?;

		let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

		let join_handle = match mode {
			Mode::Record {
				backend_url,
				cassette_dir,
			} => {
				let state = Arc::new(RecordState {
					backend_url,
					cassette_dir,
					counter: AtomicUsize::new(0),
					client: reqwest::Client::new(),
				});
				tokio::spawn(run_server(listener, shutdown_rx, move |req| {
					let state = state.clone();
					async move { handle_record(req, &state).await }
				}))
			}
			Mode::Replay { cassette_dir } => {
				let state = Arc::new(ReplayState {
					cassette_dir,
					counter: AtomicUsize::new(0),
				});
				tokio::spawn(run_server(listener, shutdown_rx, move |req| {
					let state = state.clone();
					async move { handle_replay(req, &state).await }
				}))
			}
		};

		Ok(YakbakServer {
			addr,
			shutdown_tx: Some(shutdown_tx),
			join_handle: Some(join_handle),
		})
	}
}

/// Returns a 500 error response with the given message.
fn error_response(msg: String) -> Response<BoxBody> {
	eprintln!("[yakbak] ERROR: {msg}");
	Response::builder()
		.status(500)
		.header("content-type", "text/plain")
		.body(Full::new(Bytes::from(msg)).map_err(|never| match never {}).boxed())
		.unwrap()
}

async fn run_server<F, Fut>(listener: TcpListener, mut shutdown_rx: oneshot::Receiver<()>, handler: F)
where
	F: Fn(Request<Incoming>) -> Fut + Send + Sync + Clone + 'static,
	Fut: std::future::Future<Output = Result<Response<BoxBody>, String>> + Send + 'static,
{
	loop {
		tokio::select! {
			accept = listener.accept() => {
				match accept {
					Ok((stream, _)) => {
						let handler = handler.clone();
						tokio::spawn(async move {
							let io = TokioIo::new(stream);
							let svc = service_fn(move |req: Request<Incoming>| {
								let handler = handler.clone();
								async move {
									let resp = match handler(req).await {
										Ok(r) => r,
										Err(e) => error_response(e),
									};
									Ok::<_, Infallible>(resp)
								}
							});
							if let Err(e) = http1::Builder::new().serve_connection(io, svc).await {
								eprintln!("[yakbak] connection error: {e}");
							}
						});
					}
					Err(e) => {
						eprintln!("[yakbak] accept error: {e}");
					}
				}
			}
			_ = &mut shutdown_rx => break,
		}
	}
}

struct RecordState {
	backend_url: String,
	cassette_dir: PathBuf,
	counter: AtomicUsize,
	client: reqwest::Client,
}

async fn handle_record(req: Request<Incoming>, state: &RecordState) -> Result<Response<BoxBody>, String> {
	// -- Extract request parts
	let method = req.method().clone();
	let path_and_query = req
		.uri()
		.path_and_query()
		.map(|pq| pq.as_str().to_string())
		.unwrap_or_else(|| "/".to_string());
	let req_headers: Vec<(String, String)> = req
		.headers()
		.iter()
		.filter(|(name, _)| name.as_str() != "host")
		.map(|(name, value)| (name.to_string(), value.to_str().unwrap_or("").to_string()))
		.collect();
	let body_bytes = req
		.into_body()
		.collect()
		.await
		.map_err(|e| format!("read body: {e}"))?
		.to_bytes();

	// -- Forward to real backend
	let forward_url = format!("{}{}", state.backend_url.trim_end_matches('/'), path_and_query);
	eprintln!("[yakbak] RECORD {method} {forward_url}");

	let mut builder = state.client.request(
		reqwest::Method::from_bytes(method.as_str().as_bytes()).map_err(|e| format!("method: {e}"))?,
		&forward_url,
	);
	for (name, value) in &req_headers {
		builder = builder.header(name.as_str(), value.as_str());
	}
	builder = builder.body(body_bytes.to_vec());

	let response = builder.send().await.map_err(|e| format!("forward: {e}"))?;
	let status = response.status().as_u16();
	let resp_content_type = response
		.headers()
		.get("content-type")
		.and_then(|v| v.to_str().ok())
		.unwrap_or("")
		.to_string();
	let resp_body = response.bytes().await.map_err(|e| format!("read response: {e}"))?;

	// -- Save to cassette
	tokio::fs::create_dir_all(&state.cassette_dir)
		.await
		.map_err(|e| format!("mkdir: {e}"))?;
	let idx = state.counter.fetch_add(1, Ordering::SeqCst);
	let filename = format!("response_{:03}.txt", idx);
	let filepath = state.cassette_dir.join(&filename);
	tokio::fs::write(&filepath, &resp_body)
		.await
		.map_err(|e| format!("write: {e}"))?;
	eprintln!(
		"[yakbak] SAVED {} ({} bytes, status={}, ct={})",
		filepath.display(),
		resp_body.len(),
		status,
		resp_content_type
	);

	// -- Return response to caller
	let mut resp_builder = Response::builder().status(status);
	if !resp_content_type.is_empty() {
		resp_builder = resp_builder.header("content-type", &resp_content_type);
	}
	resp_builder
		.body(Full::new(resp_body).map_err(|never| match never {}).boxed())
		.map_err(|e| format!("build response: {e}"))
}

struct ReplayState {
	cassette_dir: PathBuf,
	counter: AtomicUsize,
}

async fn handle_replay(_req: Request<Incoming>, state: &ReplayState) -> Result<Response<BoxBody>, String> {
	// -- List .txt files in sorted order
	let mut files: Vec<PathBuf> = Vec::new();
	let mut dir = tokio::fs::read_dir(&state.cassette_dir)
		.await
		.map_err(|e| format!("readdir {}: {e}", state.cassette_dir.display()))?;
	while let Some(entry) = dir.next_entry().await.map_err(|e| format!("entry: {e}"))? {
		let path = entry.path();
		if path.extension().and_then(|e| e.to_str()) == Some("txt") {
			files.push(path);
		}
	}
	files.sort();

	// -- Pick the next file
	let idx = state.counter.fetch_add(1, Ordering::SeqCst);
	let file = files.get(idx).ok_or_else(|| {
		format!(
			"REPLAY: no more response files (idx={}, dir={})",
			idx,
			state.cassette_dir.display()
		)
	})?;

	let body = tokio::fs::read(file).await.map_err(|e| format!("read: {e}"))?;
	let body_str = String::from_utf8_lossy(&body);

	// -- Infer content-type from body content
	let content_type = infer_content_type(&body_str);
	eprintln!(
		"[yakbak] REPLAY {} ({} bytes, ct={}, chunk_size={})",
		file.display(),
		body.len(),
		content_type,
		REPLAY_CHUNK_SIZE,
	);

	// -- Stream response in fixed-size chunks (like real HTTP servers do).
	// This uses HTTP/1.1 chunked transfer encoding, which is how real API
	// servers deliver SSE responses. This is important because it means
	// multi-byte UTF-8 characters can be split across chunk boundaries.
	let chunks: Vec<Result<Frame<Bytes>, Infallible>> = body
		.chunks(REPLAY_CHUNK_SIZE)
		.map(|chunk| Ok(Frame::data(Bytes::copy_from_slice(chunk))))
		.collect();
	let stream = futures::stream::iter(chunks);
	let stream_body = StreamBody::new(stream);

	Response::builder()
		.status(200)
		.header("content-type", content_type)
		.body(stream_body.boxed())
		.map_err(|e| format!("build response: {e}"))
}

/// Infer content-type from body text.
fn infer_content_type(body: &str) -> &'static str {
	let trimmed = body.trim_start();
	if trimmed.starts_with('{') || trimmed.starts_with('[') {
		"application/json"
	} else if trimmed.starts_with("event:") || trimmed.starts_with("data:") {
		"text/event-stream"
	} else {
		"text/plain"
	}
}
