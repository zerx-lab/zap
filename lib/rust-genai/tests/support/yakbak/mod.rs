//! Yakbak — lightweight HTTP record/replay for integration testing.
//!
//! - **Record mode**: proxies requests to a real backend, saves response bodies as `.txt` files.
//! - **Replay mode**: serves `.txt` files from a cassette directory in lexicographic order.
//!
//! No manifest files needed — content-type is inferred from the response body.

mod server;

pub use server::*;

use super::TestResult;
use genai::resolver::{AuthData, AuthResolver, Endpoint, ServiceTargetResolver};
use genai::{Client, ServiceTarget};

/// Build a genai `Client` that talks to a yakbak replay server.
///
/// Returns `(client, server)` — keep `server` alive for the duration of the test.
pub async fn replay_client(provider: &str, scenario: &str) -> TestResult<(Client, YakbakServer)> {
	let cassette_dir = format!("tests/data/yakbak/{provider}/{scenario}");
	let server = YakbakServer::start(Mode::Replay {
		cassette_dir: cassette_dir.into(),
	})
	.await
	.map_err(|e| format!("yakbak start failed: {e}"))?;

	let base_url = server.base_url();
	let client = Client::builder()
		.with_auth_resolver(AuthResolver::from_resolver_fn(
			|_| -> Result<Option<AuthData>, genai::resolver::Error> {
				Ok(Some(AuthData::from_single("yakbak-fake-key")))
			},
		))
		.with_service_target_resolver(ServiceTargetResolver::from_resolver_fn(
			move |st: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
				Ok(ServiceTarget {
					endpoint: Endpoint::from_owned(base_url.clone()),
					..st
				})
			},
		))
		.build();

	Ok((client, server))
}

/// Build a genai `Client` that talks through a yakbak record proxy to a real backend.
///
/// Returns `(client, server)` — call `server.shutdown().await` when done to flush cassettes.
pub async fn record_client(provider: &str, scenario: &str, backend_url: &str) -> TestResult<(Client, YakbakServer)> {
	let cassette_dir = format!("tests/data/yakbak/{provider}/{scenario}");
	let server = YakbakServer::start(Mode::Record {
		backend_url: backend_url.to_string(),
		cassette_dir: cassette_dir.into(),
	})
	.await
	.map_err(|e| format!("yakbak start failed: {e}"))?;

	let base_url = server.base_url();
	let client = Client::builder()
		.with_service_target_resolver(ServiceTargetResolver::from_resolver_fn(
			move |st: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
				Ok(ServiceTarget {
					endpoint: Endpoint::from_owned(base_url.clone()),
					..st
				})
			},
		))
		.build();

	Ok((client, server))
}
