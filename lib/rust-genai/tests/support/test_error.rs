//! TestError provides better control over error text formatting (by implementing Debug),
//! making errors more readable and legible.
//!

use std::error::Error;

pub type TestResult<T> = core::result::Result<T, TestError>;

pub struct TestError {
	inner: Box<dyn Error>,
}

/// From Strin
impl From<String> for TestError {
	fn from(value: String) -> Self {
		TestError {
			inner: Box::new(std::io::Error::other(value)),
		}
	}
}
impl From<&str> for TestError {
	fn from(value: &str) -> Self {
		TestError {
			inner: Box::new(std::io::Error::other(value.to_string())),
		}
	}
}

macro_rules! impl_test_error_from {
    ($($ty:path),* $(,)?) => {
        $(
            impl From<$ty> for TestError {
                fn from(value: $ty) -> Self {
                    TestError { inner: Box::new(value) }
                }
            }
        )*
    };
}

impl_test_error_from!(
	//
	simple_fs::Error,
	genai::Error,
	value_ext::JsonValueExtError,
	std::io::Error
);

impl std::fmt::Display for TestError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}", self.inner)
	}
}

impl std::fmt::Debug for TestError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}", self.inner)
	}
}

impl std::error::Error for TestError {}
