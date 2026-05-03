use derive_more::From;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use value_ext::JsonValueExt as _;

use crate::{ModelIden, adapter::AdapterKind};

/// Custom Content Part containing the raw json value of that part from the provider/model
/// `.model_iden` will always be captured if this is part of a response
#[derive(Debug, Clone, Serialize, Deserialize, From)]
pub struct CustomPart {
	pub model_iden: Option<ModelIden>,
	pub data: Value,
}

impl CustomPart {
	pub fn data(&self) -> &Value {
		&self.data
	}

	pub fn adapter_kind(&self) -> Option<AdapterKind> {
		self.model_iden.as_ref().map(|m| m.adapter_kind)
	}

	/// Returns the "type" field from the data, if it exists and is a string.
	pub fn typ(&self) -> Option<&str> {
		self.data.x_get_str("type").ok()
	}
}
