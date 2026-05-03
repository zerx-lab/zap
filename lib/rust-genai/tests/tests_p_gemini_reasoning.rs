mod support;

use crate::support::{Check, TestResult, common_tests};

// "gemini-2.5-flash", "gemini-2.5-pro-preview"
const MODEL: &str = "gemini-2.5-flash"; // can add "-medium" .. suffix

// NOTE: For now just single test to make sure reasoning token get captured.

#[tokio::test]
async fn test_chat_simple_ok() -> TestResult<()> {
	// NOTE: At this point, gemini 2.5 does not seems to give back reasoning content.
	//       But it should have REASONING_USAGE
	common_tests::common_test_chat_simple_ok(MODEL, Some(Check::REASONING_USAGE)).await
}
