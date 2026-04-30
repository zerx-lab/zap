use warp_completer::util::parse_current_commands_and_tokens;

use crate::{Context, test_utils::CompletionContext};

use super::*;

async fn mock_parsed_input_token(buffer_text: String) -> ParsedTokensSnapshot {
    let completion_context = CompletionContext::new();
    parse_current_commands_and_tokens(buffer_text, &completion_context).await
}

#[test]
fn test_input_detection() {
    futures::executor::block_on(async move {
        let classifier = HeuristicClassifier;

        let mut context = Context {
            current_input_type: InputType::AI,
            is_agent_follow_up: false,
        };

        let token = mock_parsed_input_token("cargo --version".to_string()).await;
        assert_eq!(
            classifier.detect_input_type(token, &context).await,
            InputType::Shell
        );

        // We have to override the first token description here given the mocked completion
        // parser will parse the first token always as commands.
        //
        // Mock the case where cargo is not installed. We should still parse this as Shell input.
        let mut token = mock_parsed_input_token("cargo --version".to_string()).await;
        token.parsed_tokens[0].token_description = None;
        assert_eq!(
            classifier.detect_input_type(token, &context).await,
            InputType::Shell
        );

        let mut token = mock_parsed_input_token("rvm install 3.3".to_string()).await;
        token.parsed_tokens[0].token_description = None;
        assert_eq!(
            classifier.detect_input_type(token, &context).await,
            InputType::Shell
        );

        // Short queries with NL should be parsed as AI input when already in AI input.
        let mut token = mock_parsed_input_token("Explain this".to_string()).await;
        token.parsed_tokens[0].token_description = None;
        assert_eq!(
            classifier.detect_input_type(token.clone(), &context).await,
            InputType::AI
        );

        context.current_input_type = InputType::Shell;

        // Typing "fix this" after an error block is a common use case.
        let mut token = mock_parsed_input_token("fix this".to_string()).await;
        token.parsed_tokens[0].token_description = None;
        assert_eq!(
            classifier.detect_input_type(token, &context).await,
            InputType::AI,
        );

        // Short queries with punctuation should be parsed as AI input.
        let token = mock_parsed_input_token("What went wrong?".to_string()).await;
        assert_eq!(
            classifier.detect_input_type(token, &context).await,
            InputType::AI
        );
        // Short queries with contractions should be parsed as AI input.
        let mut token = mock_parsed_input_token("What's the reason".to_string()).await;
        token.parsed_tokens[0].token_description = None;
        assert_eq!(
            classifier.detect_input_type(token, &context).await,
            InputType::AI
        );

        // Short queries with quotations should be parsed as AI input.
        let mut token =
            mock_parsed_input_token("The message is \"utils::future ... ok\"".to_string()).await;
        token.parsed_tokens[0].token_description = None;
        assert_eq!(
            classifier.detect_input_type(token, &context).await,
            InputType::AI
        );

        // String tokens with special shell syntax should not be treated as negative NL signal.
        let mut token = mock_parsed_input_token("The type is \"<>\"".to_string()).await;
        token.parsed_tokens[0].token_description = None;
        assert_eq!(
            classifier.detect_input_type(token, &context).await,
            InputType::AI
        );
    });
}

#[test]
fn test_cjk_input_detection() {
    futures::executor::block_on(async move {
        let classifier = HeuristicClassifier;

        // 默认从 Shell 模式触发(更严格场景,验证 CJK 仍能切到 AI)。
        let context = Context {
            current_input_type: InputType::Shell,
            is_agent_follow_up: false,
        };

        // 单个汉字也判 AI(默认逻辑会因 token 数 < 2 被判 Shell)。
        let token = mock_parsed_input_token("帮我列出当前目录文件".to_string()).await;
        assert_eq!(
            classifier.detect_input_type(token, &context).await,
            InputType::AI
        );

        // 中英混合,只要含 CJK 就走 AI。
        let token = mock_parsed_input_token("用 cargo build 编译这个项目".to_string()).await;
        assert_eq!(
            classifier.detect_input_type(token, &context).await,
            InputType::AI
        );

        // 中文标点(全角逗号、句号、问号)也命中。
        let token = mock_parsed_input_token("这是什么?".to_string()).await;
        assert_eq!(
            classifier.detect_input_type(token, &context).await,
            InputType::AI
        );

        // 日文(平假名 + 片假名)。
        let token = mock_parsed_input_token("ファイルを表示してください".to_string()).await;
        assert_eq!(
            classifier.detect_input_type(token, &context).await,
            InputType::AI
        );

        // 韩文。
        let token = mock_parsed_input_token("파일 목록을 보여줘".to_string()).await;
        assert_eq!(
            classifier.detect_input_type(token, &context).await,
            InputType::AI
        );

        // 纯英文 shell 命令不受影响。
        let token = mock_parsed_input_token("ls -la".to_string()).await;
        assert_eq!(
            classifier.detect_input_type(token, &context).await,
            InputType::Shell
        );
    });
}
