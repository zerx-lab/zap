use crate::adapter::{AdapterDispatcher, AdapterKind};
use crate::chat::ChatOptions;
use crate::client::{ModelSpec, ServiceTarget};
use crate::embed::EmbedOptions;
use crate::resolver::{AuthData, AuthResolver, Endpoint, ModelMapper, ServiceTargetResolver};
use crate::{Error, ModelIden, Result, WebConfig};

/// Configuration for building and customizing a `Client`.
#[derive(Debug, Default, Clone)]
pub struct ClientConfig {
	pub(super) auth_resolver: Option<AuthResolver>,
	pub(super) service_target_resolver: Option<ServiceTargetResolver>,
	pub(super) model_mapper: Option<ModelMapper>,
	pub(super) web_config: Option<WebConfig>,
	pub(super) chat_options: Option<ChatOptions>,
	pub(super) embed_options: Option<EmbedOptions>,
}

/// Chainable setters related to the ClientConfig.
impl ClientConfig {
	/// Sets the AuthResolver.
	///
	/// Called before `service_target_resolver`; if set, it will receive this value.
	pub fn with_auth_resolver(mut self, auth_resolver: AuthResolver) -> Self {
		self.auth_resolver = Some(auth_resolver);
		self
	}

	/// Sets the ModelMapper.
	///
	/// Called before `service_target_resolver`; if set, it will receive this value.
	pub fn with_model_mapper(mut self, model_mapper: ModelMapper) -> Self {
		self.model_mapper = Some(model_mapper);
		self
	}

	/// Sets the ServiceTargetResolver.
	///
	/// Final step before execution; allows full control over the resolved endpoint, auth, and model identifier.
	pub fn with_service_target_resolver(mut self, service_target_resolver: ServiceTargetResolver) -> Self {
		self.service_target_resolver = Some(service_target_resolver);
		self
	}

	/// Sets default ChatOptions for chat requests.
	pub fn with_chat_options(mut self, options: ChatOptions) -> Self {
		self.chat_options = Some(options);
		self
	}

	/// Sets default EmbedOptions for embed requests.
	pub fn with_embed_options(mut self, options: EmbedOptions) -> Self {
		self.embed_options = Some(options);
		self
	}

	/// Sets the HTTP client configuration (reqwest).
	pub fn with_web_config(mut self, web_config: WebConfig) -> Self {
		self.web_config = Some(web_config);
		self
	}

	/// Returns the WebConfig, if set.
	pub fn web_config(&self) -> Option<&WebConfig> {
		self.web_config.as_ref()
	}
}

/// Getters for the fields of ClientConfig (as references).
impl ClientConfig {
	/// Returns the AuthResolver, if set.
	pub fn auth_resolver(&self) -> Option<&AuthResolver> {
		self.auth_resolver.as_ref()
	}

	/// Returns the ServiceTargetResolver, if set.
	pub fn service_target_resolver(&self) -> Option<&ServiceTargetResolver> {
		self.service_target_resolver.as_ref()
	}

	/// Returns the ModelMapper, if set.
	pub fn model_mapper(&self) -> Option<&ModelMapper> {
		self.model_mapper.as_ref()
	}

	/// Returns the default ChatOptions, if set.
	pub fn chat_options(&self) -> Option<&ChatOptions> {
		self.chat_options.as_ref()
	}

	/// Returns the default EmbedOptions, if set.
	pub fn embed_options(&self) -> Option<&EmbedOptions> {
		self.embed_options.as_ref()
	}
}

/// Resolvers
impl ClientConfig {
	/// Resolves auth and endpoint for the given adapter kind.
	///
	/// Used by `Client::all_model_names()` where no specific model name is available.
	pub(crate) async fn resolve_adapter_config(&self, adapter_kind: AdapterKind) -> Result<(AuthData, Endpoint)> {
		let model = ModelIden::new(adapter_kind, "");
		let auth = self.run_auth_resolver(model).await?;
		let endpoint = AdapterDispatcher::default_endpoint(adapter_kind);
		Ok((auth, endpoint))
	}

	/// Resolves a ServiceTarget for the given model.
	///
	/// Applies the ModelMapper (if any), resolves auth (via AuthResolver or adapter default),
	/// selects the adapter's default endpoint, then applies the ServiceTargetResolver (if any).
	///
	/// Errors with Error::Resolver if any resolver step fails.
	pub async fn resolve_service_target(&self, model: ModelIden) -> Result<ServiceTarget> {
		// -- Resolve the Model first
		let model = self.run_model_mapper(model.clone())?;

		// -- Get the auth
		let auth = self.run_auth_resolver(model.clone()).await?;

		// -- Get the default endpoint
		// For now, just get the default endpoint; the `resolve_target` will allow overriding it.
		let endpoint = AdapterDispatcher::default_endpoint(model.adapter_kind);

		// -- Create the default service target
		let service_target = ServiceTarget {
			model: model.clone(),
			auth,
			endpoint,
		};

		// -- Resolve the service target
		let service_target = self.run_service_target_resolver(service_target).await?;

		Ok(service_target)
	}

	/// Resolves a [`ModelIden`] to a [`ModelIden`] via the [`ModelMapper`] (if any).
	fn run_model_mapper(&self, model: ModelIden) -> Result<ModelIden> {
		match self.model_mapper() {
			Some(model_mapper) => model_mapper.map_model(model.clone()),
			None => Ok(model.clone()),
		}
		.map_err(|resolver_error| Error::Resolver {
			model_iden: model.clone(),
			resolver_error,
		})
	}

	/// Resolves a [`ModelIden`] to an [`AuthData`] via the [`AuthResolver`] (if any).
	async fn run_auth_resolver(&self, model: ModelIden) -> Result<AuthData> {
		match self.auth_resolver() {
			Some(auth_resolver) => {
				let auth_data = auth_resolver
					.resolve(model.clone())
					.await
					.map_err(|err| Error::Resolver {
						model_iden: model.clone(),
						resolver_error: err,
					})?
					// default the resolver resolves to nothing
					.unwrap_or_else(|| AdapterDispatcher::default_auth(model.adapter_kind));

				Ok(auth_data)
			}
			None => Ok(AdapterDispatcher::default_auth(model.adapter_kind)),
		}
	}

	/// Resolves a [`ServiceTarget`] via the [`ServiceTargetResolver`] (if any).
	async fn run_service_target_resolver(&self, service_target: ServiceTarget) -> Result<ServiceTarget> {
		let model = service_target.model.clone();

		match self.service_target_resolver() {
			Some(service_target_resolver) => {
				service_target_resolver
					.resolve(service_target)
					.await
					.map_err(|resolver_error| Error::Resolver {
						model_iden: model,
						resolver_error,
					})
			}
			None => Ok(service_target),
		}
	}

	/// Resolves a [`ModelSpec`] to a [`ServiceTarget`].
	///
	/// The resolution behavior depends on the variant:
	///
	/// - [`ModelSpec::Name`]: Infers adapter from name, then applies full resolution
	///   (model mapper, auth resolver, service target resolver).
	///
	/// - [`ModelSpec::Iden`]: Skips adapter inference, applies full resolution.
	///
	/// - [`ModelSpec::Target`]: Returns the target directly, running only the service target resolver.
	pub async fn resolve_model_spec(&self, spec: ModelSpec) -> Result<ServiceTarget> {
		match spec {
			ModelSpec::Name(name) => {
				let adapter_kind = AdapterKind::from_model(&name)?;
				let model = ModelIden::new(adapter_kind, name);
				self.resolve_service_target(model).await
			}
			ModelSpec::Iden(model) => self.resolve_service_target(model).await,
			ModelSpec::Target(target) => self.run_service_target_resolver(target).await,
		}
	}
}
