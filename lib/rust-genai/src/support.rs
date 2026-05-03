use serde::{Deserialize, Deserializer};

// region:    --- Serde Support

pub fn zero_as_none<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
	D: Deserializer<'de>,
	T: Deserialize<'de> + PartialEq + Default,
{
	// Attempt to deserialize as the inner type T
	let value = Option::<T>::deserialize(deserializer)?;

	// If the value is Some(0) or the default for the type, return None
	match value {
		Some(val) if val == T::default() => Ok(None),
		other => Ok(other),
	}
}

// endregion: --- Serde Support

// region:    --- Text Support

pub fn combine_text_with_empty_line(combined: &mut String, text: &str) {
	if !combined.is_empty() {
		if combined.ends_with('\n') {
			combined.push('\n');
		} else if !combined.is_empty() {
			combined.push_str("\n\n");
		}
	}
	// Do not add any empty line if previous content is empty

	combined.push_str(text);
}

// endregion: --- Text Support
