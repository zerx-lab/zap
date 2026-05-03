use crate::ModelIden;
use crate::resolver::Result;
use std::sync::Arc;

// region:    --- ModelMapper

/// A ModelMapper for mapping a resolved `ModelIden` (i.e. AdapterKind + ModelName) to another one.
/// It must return a `ModelIden` or an appropriate result.
#[derive(Debug, Clone)]
pub enum ModelMapper {
	/// The variant that holds the mapper function
	MapperFn(Arc<Box<dyn ModelMapperFn>>),
}

impl ModelMapper {
	/// Create a new `ModelMapper` from a mapper function.
	pub fn from_mapper_fn(mapper_fn: impl IntoModelMapperFn) -> Self {
		ModelMapper::MapperFn(mapper_fn.into_mapper_fn())
	}
}

impl ModelMapper {
	pub(crate) fn map_model(&self, model_iden: ModelIden) -> Result<ModelIden> {
		match self {
			ModelMapper::MapperFn(mapper_fn) => {
				// Clone the Arc to get a new reference to the Box, then call exec_fn.
				mapper_fn.clone().exec_fn(model_iden)
			}
		}
	}
}

// endregion: --- ModelMapper

// region:    --- ModelMapperFn

/// The `ModelMapperFn` trait object.
pub trait ModelMapperFn: Send + Sync {
	/// Execute the `ModelMapperFn` to obtain the `ModelIden`.
	fn exec_fn(&self, model_iden: ModelIden) -> Result<ModelIden>;

	/// Clone the trait object into a boxed dynamic object.
	fn clone_box(&self) -> Box<dyn ModelMapperFn>;
}

// Implement ModelMapperFn for any `FnOnce`
impl<F> ModelMapperFn for F
where
	F: FnOnce(ModelIden) -> Result<ModelIden> + Send + Sync + Clone + 'static,
{
	fn exec_fn(&self, model_iden: ModelIden) -> Result<ModelIden> {
		(self.clone())(model_iden)
	}

	fn clone_box(&self) -> Box<dyn ModelMapperFn> {
		Box::new(self.clone())
	}
}

// Implement Clone for Box<dyn ModelMapperFn>
impl Clone for Box<dyn ModelMapperFn> {
	fn clone(&self) -> Box<dyn ModelMapperFn> {
		self.clone_box()
	}
}

impl std::fmt::Debug for dyn ModelMapperFn {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "ModelMapperFn")
	}
}

// endregion: --- ModelMapperFn

// region:    --- IntoModelMapperFn

/// Implement IntoModelMapperFn for closures used as arguments in `ModelMapper::from_mapper_fn`.
pub trait IntoModelMapperFn {
	/// Convert the given closure into a `ModelMapperFn` trait object.
	fn into_mapper_fn(self) -> Arc<Box<dyn ModelMapperFn>>;
}

impl IntoModelMapperFn for Arc<Box<dyn ModelMapperFn>> {
	fn into_mapper_fn(self) -> Arc<Box<dyn ModelMapperFn>> {
		self
	}
}

impl<F> IntoModelMapperFn for F
where
	F: FnOnce(ModelIden) -> Result<ModelIden> + Send + Sync + Clone + 'static,
{
	fn into_mapper_fn(self) -> Arc<Box<dyn ModelMapperFn>> {
		Arc::new(Box::new(self))
	}
}

// endregion: --- IntoModelMapperFn
