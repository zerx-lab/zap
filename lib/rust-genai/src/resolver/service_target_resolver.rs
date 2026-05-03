//! A `ServiceTargetResolver` is responsible for returning the `ServiceTarget`.
//! It allows users to customize or override the service target properties.
//!
//! It can take the following forms:
//! - Contains a fixed service target value,
//! - Contains a `ServiceTargetResolverFn` trait object or closure that will be called to return the `ServiceTarget`.
//! - Contains an `ServiceTargetResolverAsyncFn` trait object or closure that will be called asynchronously to return the `ServiceTarget`.

use crate::ServiceTarget;
use crate::resolver::Result;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// region:    --- ServiceTargetResolver

/// Holder for the `ServiceTargetResolver` function.
#[derive(Debug, Clone)]
pub enum ServiceTargetResolver {
	/// The synchronous `ServiceTargetResolverFn` trait object.
	ResolverFn(Arc<Box<dyn ServiceTargetResolverFn>>),
	/// The asynchronous `ServiceTargetResolverAsyncFn` trait object.
	ResolverAsyncFn(Arc<Box<dyn ServiceTargetResolverAsyncFn>>),
}

impl ServiceTargetResolver {
	/// Create a new synchronous `ServiceTargetResolver` from a resolver function.
	pub fn from_resolver_fn(resolver_fn: impl IntoServiceTargetResolverFn) -> Self {
		ServiceTargetResolver::ResolverFn(resolver_fn.into_resolver_fn())
	}

	/// Create a new asynchronous `ServiceTargetResolver` from an async resolver function.
	pub fn from_resolver_async_fn(resolver_fn: impl IntoServiceTargetResolverAsyncFn) -> Self {
		ServiceTargetResolver::ResolverAsyncFn(resolver_fn.into_resolver_async_fn())
	}
}

impl ServiceTargetResolver {
	/// Resolve the ServiceTarget, calling the appropriate sync or async function.
	pub(crate) async fn resolve(&self, service_target: ServiceTarget) -> Result<ServiceTarget> {
		match self {
			ServiceTargetResolver::ResolverFn(resolver_fn) => resolver_fn.clone().exec_fn(service_target),
			ServiceTargetResolver::ResolverAsyncFn(resolver_fn) => resolver_fn.clone().exec_fn(service_target).await,
		}
	}
}

// endregion: --- ServiceTargetResolver

// region:    --- ServiceTargetResolverFn (Sync)

/// The synchronous `ServiceTargetResolverFn` trait object.
pub trait ServiceTargetResolverFn: Send + Sync {
	/// Execute the `ServiceTargetResolverFn` to get the `ServiceTarget`.
	fn exec_fn(&self, service_target: ServiceTarget) -> Result<ServiceTarget>;

	/// Clone the trait object.
	fn clone_box(&self) -> Box<dyn ServiceTargetResolverFn>;
}

/// `ServiceTargetResolverFn` blanket implementation for any function that matches the resolver function signature.
impl<F> ServiceTargetResolverFn for F
where
	F: FnOnce(ServiceTarget) -> Result<ServiceTarget> + Send + Sync + Clone + 'static,
{
	fn exec_fn(&self, service_target: ServiceTarget) -> Result<ServiceTarget> {
		(self.clone())(service_target)
	}

	fn clone_box(&self) -> Box<dyn ServiceTargetResolverFn> {
		Box::new(self.clone())
	}
}

// Implement Clone for Box<dyn ServiceTargetResolverFn>
impl Clone for Box<dyn ServiceTargetResolverFn> {
	fn clone(&self) -> Box<dyn ServiceTargetResolverFn> {
		self.clone_box()
	}
}

impl std::fmt::Debug for dyn ServiceTargetResolverFn {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "ServiceTargetResolverFn")
	}
}

// endregion: --- ServiceTargetResolverFn (Sync)

// region:    --- IntoServiceTargetResolverFn (Sync)

/// Custom and convenient trait used in the `ServiceTargetResolver::from_resolver_fn` argument.
pub trait IntoServiceTargetResolverFn {
	/// Convert the argument into a `ServiceTargetResolverFn` trait object.
	fn into_resolver_fn(self) -> Arc<Box<dyn ServiceTargetResolverFn>>;
}

impl IntoServiceTargetResolverFn for Arc<Box<dyn ServiceTargetResolverFn>> {
	fn into_resolver_fn(self) -> Arc<Box<dyn ServiceTargetResolverFn>> {
		self
	}
}

// Implement `IntoServiceTargetResolverFn` for closures.
impl<F> IntoServiceTargetResolverFn for F
where
	F: FnOnce(ServiceTarget) -> Result<ServiceTarget> + Send + Sync + Clone + 'static,
{
	fn into_resolver_fn(self) -> Arc<Box<dyn ServiceTargetResolverFn>> {
		Arc::new(Box::new(self))
	}
}

// endregion: --- IntoServiceTargetResolverFn (Sync)

// region:    --- ServiceTargetResolverAsyncFn (Async)

/// The asynchronous `ServiceTargetResolverAsyncFn` trait object.
pub trait ServiceTargetResolverAsyncFn: Send + Sync {
	/// Execute the `ServiceTargetResolverAsyncFn` asynchronously to get the `ServiceTarget`.
	fn exec_fn(&self, service_target: ServiceTarget) -> Pin<Box<dyn Future<Output = Result<ServiceTarget>> + Send>>;
	/// Clone the trait object.
	fn clone_box(&self) -> Box<dyn ServiceTargetResolverAsyncFn>;
}

impl<F> ServiceTargetResolverAsyncFn for F
where
	F: Fn(ServiceTarget) -> Pin<Box<dyn Future<Output = Result<ServiceTarget>> + Send>> + Send + Sync + Clone + 'static,
{
	fn exec_fn(&self, service_target: ServiceTarget) -> Pin<Box<dyn Future<Output = Result<ServiceTarget>> + Send>> {
		self.clone()(service_target)
	}

	fn clone_box(&self) -> Box<dyn ServiceTargetResolverAsyncFn> {
		Box::new(self.clone())
	}
}

impl std::fmt::Debug for dyn ServiceTargetResolverAsyncFn {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "ServiceTargetResolverAsyncFn")
	}
}

impl Clone for Box<dyn ServiceTargetResolverAsyncFn> {
	fn clone(&self) -> Self {
		self.clone_box()
	}
}

// endregion: --- ServiceTargetResolverAsyncFn (Async)

// region:    --- IntoServiceTargetResolverAsyncFn (Async)

/// Custom and convenient trait used in the `ServiceTargetResolver::from_resolver_async_fn` argument.
pub trait IntoServiceTargetResolverAsyncFn {
	/// Convert the argument into an `ServiceTargetResolverAsyncFn` trait object.
	fn into_resolver_async_fn(self) -> Arc<Box<dyn ServiceTargetResolverAsyncFn>>;
}

impl IntoServiceTargetResolverAsyncFn for Arc<Box<dyn ServiceTargetResolverAsyncFn>> {
	fn into_resolver_async_fn(self) -> Arc<Box<dyn ServiceTargetResolverAsyncFn>> {
		self
	}
}

impl<F> IntoServiceTargetResolverAsyncFn for F
where
	F: Fn(ServiceTarget) -> Pin<Box<dyn Future<Output = Result<ServiceTarget>> + Send>> + Send + Sync + Clone + 'static,
{
	fn into_resolver_async_fn(self) -> Arc<Box<dyn ServiceTargetResolverAsyncFn>> {
		Arc::new(Box::new(self))
	}
}

// endregion: --- IntoServiceTargetResolverAsyncFn (Async)
