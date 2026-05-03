//! An `AuthResolver` is responsible for returning the `AuthData` (typically containing the `api_key`).
//! It can take the following forms:
//! - Configured with a custom environment name,
//! - Contains a fixed auth value,
//! - Contains an `AuthResolverFn` trait object or closure that will be called to return the `AuthData`.
//!
//! Note: `AuthData` is typically a single value but can be multiple for future adapters (e.g., AWS Bedrock).

use crate::ModelIden;
use crate::resolver::{AuthData, Result};
use std::pin::Pin;
use std::sync::Arc;

// region:    --- AuthResolver

/// Holder for the `AuthResolver` function.
#[derive(Debug, Clone)]
pub enum AuthResolver {
	/// The `AuthResolverFn` trait object.
	ResolverFn(Arc<Box<dyn AuthResolverFn>>),
	ResolverAsyncFn(Arc<Box<dyn AuthResolverAsyncFn>>),
}

impl AuthResolver {
	/// Create a new `AuthResolver` from a resolver function.
	pub fn from_resolver_fn(resolver_fn: impl IntoAuthResolverFn) -> Self {
		AuthResolver::ResolverFn(resolver_fn.into_resolver_fn())
	}

	pub fn from_resolver_async_fn(resolver_fn: impl IntoAuthResolverAsyncFn) -> Self {
		AuthResolver::ResolverAsyncFn(resolver_fn.into_async_auth_resolver())
	}
}

impl AuthResolver {
	pub(crate) async fn resolve(&self, model_iden: ModelIden) -> Result<Option<AuthData>> {
		match self {
			AuthResolver::ResolverFn(resolver_fn) => resolver_fn.clone().exec_fn(model_iden),
			AuthResolver::ResolverAsyncFn(resolver_fn) => resolver_fn.exec_fn(model_iden).await,
		}
	}
	// pub(crate) async fn resolve_or_default(&self, ())
}

// endregion: --- AuthResolver

// region:    --- AuthResolverAsyncFn

pub trait AuthResolverAsyncFn: Send + Sync {
	fn exec_fn(&self, model_iden: ModelIden) -> Pin<Box<dyn Future<Output = Result<Option<AuthData>>> + Send>>;
	fn clone_box(&self) -> Box<dyn AuthResolverAsyncFn>;
}

impl<F> AuthResolverAsyncFn for F
where
	F: Fn(ModelIden) -> Pin<Box<dyn Future<Output = Result<Option<AuthData>>> + Send>> + Send + Sync + Clone + 'static,
{
	fn exec_fn(&self, model_iden: ModelIden) -> Pin<Box<dyn Future<Output = Result<Option<AuthData>>> + Send>> {
		self(model_iden)
	}

	fn clone_box(&self) -> Box<dyn AuthResolverAsyncFn> {
		Box::new(self.clone())
	}
}

impl std::fmt::Debug for dyn AuthResolverAsyncFn {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "AuthResolverAsyncFn")
	}
}

impl Clone for Box<dyn AuthResolverAsyncFn> {
	fn clone(&self) -> Self {
		self.clone_box()
	}
}

pub trait IntoAuthResolverAsyncFn {
	fn into_async_auth_resolver(self) -> Arc<Box<dyn AuthResolverAsyncFn>>;
}

impl IntoAuthResolverAsyncFn for Arc<Box<dyn AuthResolverAsyncFn>> {
	fn into_async_auth_resolver(self) -> Arc<Box<dyn AuthResolverAsyncFn>> {
		self
	}
}

impl<F> IntoAuthResolverAsyncFn for F
where
	F: Fn(ModelIden) -> Pin<Box<dyn Future<Output = Result<Option<AuthData>>> + Send>> + Send + Sync + Clone + 'static,
{
	fn into_async_auth_resolver(self) -> Arc<Box<dyn AuthResolverAsyncFn>> {
		Arc::new(Box::new(self))
	}
}

// endregion: --- AuthResolverAsyncFn

// region:    --- AuthResolverFn
/// The `AuthResolverFn` trait object.
pub trait AuthResolverFn: Send + Sync {
	/// Execute the `AuthResolverFn` to get the `AuthData`.
	fn exec_fn(&self, model_iden: ModelIden) -> Result<Option<AuthData>>;

	/// Clone the trait object.
	fn clone_box(&self) -> Box<dyn AuthResolverFn>;
}

/// `AuthResolverFn` blanket implementation for any function that matches the `AuthResolver` function signature.
impl<F> AuthResolverFn for F
where
	F: FnOnce(ModelIden) -> Result<Option<AuthData>> + Send + Sync + Clone + 'static,
{
	fn exec_fn(&self, model_iden: ModelIden) -> Result<Option<AuthData>> {
		(self.clone())(model_iden)
	}

	fn clone_box(&self) -> Box<dyn AuthResolverFn> {
		Box::new(self.clone())
	}
}

// Implement Clone for Box<dyn AuthResolverFn>
impl Clone for Box<dyn AuthResolverFn> {
	fn clone(&self) -> Box<dyn AuthResolverFn> {
		self.clone_box()
	}
}

impl std::fmt::Debug for dyn AuthResolverFn {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "AuthResolverFn")
	}
}

// endregion: --- AuthResolverFn

// region:    --- IntoAuthResolverFn

/// Custom and convenient trait used in the `AuthResolver::from_resolver_fn` argument.
pub trait IntoAuthResolverFn {
	/// Convert the argument into an `AuthResolverFn` trait object.
	fn into_resolver_fn(self) -> Arc<Box<dyn AuthResolverFn>>;
}

impl IntoAuthResolverFn for Arc<Box<dyn AuthResolverFn>> {
	fn into_resolver_fn(self) -> Arc<Box<dyn AuthResolverFn>> {
		self
	}
}

// Implement `IntoAuthResolverFn` for closures.
impl<F> IntoAuthResolverFn for F
where
	F: FnOnce(ModelIden) -> Result<Option<AuthData>> + Send + Sync + Clone + 'static,
{
	fn into_resolver_fn(self) -> Arc<Box<dyn AuthResolverFn>> {
		Arc::new(Box::new(self))
	}
}

// endregion: --- IntoAuthResolverFn
