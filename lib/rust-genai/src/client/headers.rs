use derive_more::From;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A map of HTTP headers (single value per name).
/// Headers can be layered; later values override earlier ones.
///
/// Note: This type keeps a single value per header name, which is the most
/// intuitive behavior for genai. It enables straightforward auth and other
/// overrides.
#[derive(Debug, Default, Clone, From, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Headers {
	inner: HashMap<String, String>,
}

impl Headers {
	/// Merge headers from overlay into self, consuming overlay.
	/// Later values override existing ones.
	/// Use [`Headers::merge_with`] for a borrowed overlay.
	pub fn merge(&mut self, overlay: impl Into<Headers>) {
		let overlay = overlay.into();

		// Insert or replace each single-value override:
		for (k, v) in overlay.inner {
			self.inner.insert(k, v);
		}
	}

	/// Merge headers from overlay into self without consuming it.
	/// Later values override existing ones.
	pub fn merge_with(&mut self, overlay: &Headers) {
		// Insert or replace each single-value override:
		for (k, v) in &overlay.inner {
			self.inner.insert(k.clone(), v.clone());
		}
	}

	/// Apply self on top of target, consuming both, and return the result.
	/// Values in self override those in target.
	pub fn applied_to(self, target: impl Into<Headers>) -> Headers {
		let mut target = target.into();
		for (k, v) in self.inner {
			target.inner.insert(k, v);
		}
		target
	}
}

// region:    --- Froms
impl<K, V> From<(K, V)> for Headers
where
	K: Into<String>,
	V: Into<String>,
{
	fn from((k, v): (K, V)) -> Self {
		let mut inner = HashMap::new();
		inner.insert(k.into(), v.into());
		Headers { inner }
	}
}

impl<K, V> From<Vec<(K, V)>> for Headers
where
	K: Into<String>,
	V: Into<String>,
{
	fn from(vec: Vec<(K, V)>) -> Self {
		let inner = vec.into_iter().map(|(k, v)| (k.into(), v.into())).collect();
		Headers { inner }
	}
}

impl<K, V, const N: usize> From<[(K, V); N]> for Headers
where
	K: Into<String>,
	V: Into<String>,
{
	fn from(arr: [(K, V); N]) -> Self {
		let inner = arr.into_iter().map(|(k, v)| (k.into(), v.into())).collect();
		Headers { inner }
	}
}

// endregion: --- Froms

// region:    --- Iterators

use std::collections::hash_map::{IntoIter, Iter, IterMut};

impl Headers {
	/// Returns an iterator over (name, value) pairs.
	pub fn iter(&self) -> Iter<'_, String, String> {
		self.inner.iter()
	}

	/// Returns a mutable iterator over (name, value) pairs.
	pub fn iter_mut(&mut self) -> IterMut<'_, String, String> {
		self.inner.iter_mut()
	}
}

// IntoIterator for owned Headers
impl IntoIterator for Headers {
	type Item = (String, String);
	type IntoIter = IntoIter<String, String>;

	fn into_iter(self) -> Self::IntoIter {
		self.inner.into_iter()
	}
}

// IntoIterator for &Headers
impl<'a> IntoIterator for &'a Headers {
	type Item = (&'a String, &'a String);
	type IntoIter = Iter<'a, String, String>;

	fn into_iter(self) -> Self::IntoIter {
		self.inner.iter()
	}
}

// IntoIterator for &mut Headers
impl<'a> IntoIterator for &'a mut Headers {
	type Item = (&'a String, &'a mut String);
	type IntoIter = IterMut<'a, String, String>;

	fn into_iter(self) -> Self::IntoIter {
		self.inner.iter_mut()
	}
}

// endregion: --- Iterators
