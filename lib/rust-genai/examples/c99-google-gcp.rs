use gcp_auth::{CustomServiceAccount, TokenProvider};
use genai::Client;
use genai::ModelIden;
use genai::chat::printer::print_chat_stream;
use genai::chat::{ChatMessage, ChatRequest};
use genai::resolver::{AuthData, AuthResolver};
use std::pin::Pin;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

// With the Vertex adapter, use the `vertex::` namespace prefix.
// The adapter handles URL construction and publisher routing automatically.
const GEMINI_MODEL: &str = "vertex::gemini-2.5-flash";
const CLAUDE_MODEL: &str = "vertex::claude-sonnet-4-6";

/// This example shows how to use the Vertex AI adapter with an AuthResolver
/// that obtains OAuth2 tokens from a GCP service account.
///
/// The `vertex::` adapter handles URL construction (region, project, publisher)
/// and wire format dispatch (Gemini vs Anthropic) automatically.
/// You only need to supply a Bearer token via the AuthResolver.
///
/// For an alternative approach without the Vertex adapter, you can use
/// `AuthData::RequestOverride` with the Gemini adapter to manually construct
/// the full Vertex URL and auth headers. See the git history of this file
/// for that pattern, which also works as a general escape hatch for any
/// custom endpoint/auth scenario.
///
/// Required env vars:
///   - GCP_SERVICE_ACCOUNT: JSON content of the service account key
///   - VERTEX_PROJECT_ID: Your GCP project ID
///   - VERTEX_LOCATION: GCP region (uses "global" location if not set)
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::new("genai=debug"))
		// .with_max_level(tracing::Level::DEBUG) // To enable all sub-library tracing
		.init();

	let gcp_env_name: Arc<str> = "GCP_SERVICE_ACCOUNT".into();

	// -- Create the Async Auth resolver that fetches OAuth2 tokens from the service account
	let resolve_fn = move |_model: ModelIden| -> Pin<
		Box<dyn Future<Output = Result<Option<AuthData>, genai::resolver::Error>> + Send + 'static>,
	> {
		let gcp_env_name = gcp_env_name.clone();
		Box::pin(async move {
			let gcp_json = std::env::var(&*gcp_env_name).map_err(|_err| genai::resolver::Error::ApiKeyEnvNotFound {
				env_name: gcp_env_name.to_string(),
			})?;
			let account = CustomServiceAccount::from_json(&gcp_json)
				.map_err(|e| genai::resolver::Error::Custom(e.to_string()))?;
			let scopes: &[&str] = &["https://www.googleapis.com/auth/cloud-platform"];
			let token = account
				.token(scopes)
				.await
				.map_err(|e| genai::resolver::Error::Custom(e.to_string()))?;
			Ok(Some(AuthData::from_single(token.as_str())))
		})
	};

	// -- Create the AuthResolver
	let auth_resolver = AuthResolver::from_resolver_async_fn(resolve_fn);

	// -- Create Chat Client
	let client = Client::builder().with_auth_resolver(auth_resolver).build();

	// -- Example 1: Gemini on Vertex AI
	println!("--- Gemini on Vertex AI ---");
	let chat_request = ChatRequest::default().with_system("Answer in one sentence");
	let chat_request = chat_request.append_message(ChatMessage::user("Why is the sky blue?"));
	let stream = client.exec_chat_stream(GEMINI_MODEL, chat_request, None).await?;
	print_chat_stream(stream, None).await?;

	// -- Example 2: Claude on Vertex AI (Model Garden)
	println!("\n--- Claude on Vertex AI (Model Garden) ---");
	let chat_request = ChatRequest::default().with_system("Answer in one sentence");
	let chat_request = chat_request.append_message(ChatMessage::user("Why is the sky blue?"));
	let stream = client.exec_chat_stream(CLAUDE_MODEL, chat_request, None).await?;
	print_chat_stream(stream, None).await?;

	Ok(())
}
