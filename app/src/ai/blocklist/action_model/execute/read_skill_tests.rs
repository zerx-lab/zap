use super::*;
use crate::ai::agent::task::TaskId;
use crate::ai::agent::AIAgentActionResultType;
use crate::ai::agent::ReadSkillRequest;
use crate::ai::agent::ReadSkillResult;
use crate::ai::agent::{AIAgentAction, AIAgentActionId, AIAgentActionType};
use crate::ai::blocklist::action_model::AIConversationId;
use crate::ai::skills::SkillManager;
use crate::warp_managed_paths_watcher::WarpManagedPathsWatcher;
use ai::agent::action_result::AnyFileContent;
use ai::skills::{parse_skill, SkillReference};
use repo_metadata::{
    repositories::DetectedRepositories, watcher::DirectoryWatcher, RepoMetadataModel,
};
use std::fs;
use std::io::Write;
use tempfile::TempDir;
use warpui::App;
use watcher::HomeDirectoryWatcher;

fn initialize_app(app: &mut App) {
    app.add_singleton_model(DirectoryWatcher::new);
    app.add_singleton_model(|_| DetectedRepositories::default());
    app.add_singleton_model(RepoMetadataModel::new);
    app.add_singleton_model(HomeDirectoryWatcher::new_for_test);
    app.add_singleton_model(WarpManagedPathsWatcher::new_for_testing);
    app.add_singleton_model(SkillManager::new);
}

fn create_test_skill_file(dir: &TempDir, name: &str, description: &str) -> std::path::PathBuf {
    let skill_content = format!(
        r#"---
name: {}
description: {}
---

# {}

## Instructions
Test instructions for this skill.

## Examples
Example usage of the skill.
"#,
        name, description, name
    );

    let skill_dir = dir.path().join(format!(".claude/skills/{}", name));
    fs::create_dir_all(&skill_dir).unwrap();
    let skill_path = skill_dir.join("SKILL.md");
    let mut file = fs::File::create(&skill_path).unwrap();
    file.write_all(skill_content.as_bytes()).unwrap();
    file.flush().unwrap();

    skill_path
}

#[test]
fn test_read_skill_executor_success() {
    let temp_dir = TempDir::new().unwrap();
    let skill_path = create_test_skill_file(&temp_dir, "test-skill", "A test skill");

    App::test((), |mut app| async move {
        initialize_app(&mut app);

        // Populate SkillManager cache with the test skill
        let parsed_skill = parse_skill(&skill_path).expect("Failed to parse test skill");
        SkillManager::handle(&app).update(&mut app, |manager, _ctx| {
            manager.add_skill_for_testing(parsed_skill);
        });

        let executor_handle = app.add_model(|_| ReadSkillExecutor::new());

        let action = AIAgentAction {
            id: AIAgentActionId::from("test-action-id".to_string()),
            action: AIAgentActionType::ReadSkill(ReadSkillRequest {
                skill: SkillReference::Path(skill_path.clone()),
            }),
            task_id: TaskId::new("test-task-id".to_string()),
            requires_result: false,
        };

        let input = ExecuteActionInput {
            action: &action,
            conversation_id: AIConversationId::new(),
        };

        executor_handle.update(&mut app, |executor, ctx| {
            let result: AnyActionExecution = executor.execute(input, ctx).into();

            match result {
                AnyActionExecution::Sync(AIAgentActionResultType::ReadSkill(
                    ReadSkillResult::Success { content },
                )) => {
                    assert_eq!(content.file_name, skill_path.to_string_lossy().to_string());
                }
                _ => panic!("Successfully read skill file; should return ReadSkillResult::Success"),
            }
        });
    });
}

#[test]
fn test_read_skill_executor_file_not_found() {
    let temp_dir = TempDir::new().unwrap();
    // Don't create the SKILL.md file
    let skill_path = temp_dir.path().join("SKILL.md");

    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let executor_handle = app.add_model(|_| ReadSkillExecutor::new());

        let action = AIAgentAction {
            id: AIAgentActionId::from("test-action-id".to_string()),
            action: AIAgentActionType::ReadSkill(ReadSkillRequest {
                skill: SkillReference::Path(skill_path),
            }),
            task_id: TaskId::new("test-task-id".to_string()),
            requires_result: false,
        };

        let input = ExecuteActionInput {
            action: &action,
            conversation_id: AIConversationId::new(),
        };

        executor_handle.update(&mut app, |executor, ctx| {
            let result: AnyActionExecution = executor.execute(input, ctx).into();

            match result {
                AnyActionExecution::Sync(AIAgentActionResultType::ReadSkill(
                    ReadSkillResult::Error(error_msg),
                )) => {
                    // Should contain an error about file not found or I/O error
                    assert!(!error_msg.is_empty());
                }
                _ => panic!(
                    "Nonexistent SKILL.md file at given path; should return ReadSkillResult::Error"
                ),
            }
        });
    });
}

/// Issue #99 兜底:cache 未命中时,若 SkillReference::Path 指向合法形状的 skill 文件,
/// 直接读盘并成功返回(走 Async 分支)。
#[test]
fn test_read_skill_executor_fallback_reads_disk_on_cache_miss() {
    let temp_dir = TempDir::new().unwrap();
    let skill_path = create_test_skill_file(&temp_dir, "fallback-skill", "Read from disk");

    App::test((), |mut app| async move {
        initialize_app(&mut app);
        // 注意:不调用 add_skill_for_testing,模拟 cache miss。
        let executor_handle = app.add_model(|_| ReadSkillExecutor::new());

        let action = AIAgentAction {
            id: AIAgentActionId::from("fallback-action".to_string()),
            action: AIAgentActionType::ReadSkill(ReadSkillRequest {
                skill: SkillReference::Path(skill_path.clone()),
            }),
            task_id: TaskId::new("fallback-task".to_string()),
            requires_result: false,
        };

        let input = ExecuteActionInput {
            action: &action,
            conversation_id: AIConversationId::new(),
        };

        let execution = executor_handle.update(&mut app, |executor, ctx| {
            let result: AnyActionExecution = executor.execute(input, ctx).into();
            result
        });

        let AnyActionExecution::Async {
            execute_future,
            on_complete,
        } = execution
        else {
            panic!("Cache miss with valid skill path should produce Async execution");
        };

        let async_result = execute_future.await;
        let result = app.update(|ctx| on_complete(async_result, ctx));

        match result {
            AIAgentActionResultType::ReadSkill(ReadSkillResult::Success { content }) => {
                assert_eq!(content.file_name, skill_path.to_string_lossy().to_string());
                let body = match &content.content {
                    AnyFileContent::StringContent(s) => s.clone(),
                    AnyFileContent::BinaryContent(_) => {
                        panic!("SKILL.md should be parsed as text")
                    }
                };
                assert!(body.contains("fallback-skill"));
            }
            other => panic!("Fallback should return Success, got: {other:?}"),
        }
    });
}

/// Issue #99 兜底失败路径:cache 未命中时,若路径形状合法但磁盘上文件不存在
///(例如校验后被删的竞态),Async 分支的 parse_skill 失败,on_complete 应返回 Error。
#[test]
fn test_read_skill_executor_fallback_returns_error_when_file_missing() {
    let temp_dir = TempDir::new().unwrap();
    // 路径形状合法,但 SKILL.md 从未被创建。
    let skill_path = temp_dir
        .path()
        .join(".agents/skills/missing-skill/SKILL.md");

    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let executor_handle = app.add_model(|_| ReadSkillExecutor::new());

        let action = AIAgentAction {
            id: AIAgentActionId::from("missing-action".to_string()),
            action: AIAgentActionType::ReadSkill(ReadSkillRequest {
                skill: SkillReference::Path(skill_path),
            }),
            task_id: TaskId::new("missing-task".to_string()),
            requires_result: false,
        };

        let input = ExecuteActionInput {
            action: &action,
            conversation_id: AIConversationId::new(),
        };

        let execution = executor_handle.update(&mut app, |executor, ctx| {
            let result: AnyActionExecution = executor.execute(input, ctx).into();
            result
        });

        let AnyActionExecution::Async {
            execute_future,
            on_complete,
        } = execution
        else {
            panic!("Legal-shaped skill path should still produce Async execution before disk check");
        };

        let async_result = execute_future.await;
        let result = app.update(|ctx| on_complete(async_result, ctx));

        match result {
            AIAgentActionResultType::ReadSkill(ReadSkillResult::Error(msg)) => {
                assert!(msg.starts_with("Skill not found"));
            }
            other => panic!("Missing file should resolve to Error, got: {other:?}"),
        }
    });
}

/// BYOP `read_skill` 工具用 name 调用时:
/// `from_args` 把 name 装进 `SkillReference::SkillPath(name)`,
/// executor 端 cache miss 后按 name 反查命中并 Sync Success 返回。
#[test]
fn test_read_skill_executor_resolves_by_name() {
    let temp_dir = TempDir::new().unwrap();
    let skill_path = create_test_skill_file(&temp_dir, "byop-named-skill", "Lookup by name");

    App::test((), |mut app| async move {
        initialize_app(&mut app);

        let parsed_skill = parse_skill(&skill_path).expect("Failed to parse test skill");
        SkillManager::handle(&app).update(&mut app, |manager, _ctx| {
            manager.add_skill_for_testing(parsed_skill);
        });

        let executor_handle = app.add_model(|_| ReadSkillExecutor::new());

        // 模拟 BYOP from_args:把 name 当作 path 传入。
        let action = AIAgentAction {
            id: AIAgentActionId::from("name-lookup-action".to_string()),
            action: AIAgentActionType::ReadSkill(ReadSkillRequest {
                skill: SkillReference::Path(std::path::PathBuf::from("byop-named-skill")),
            }),
            task_id: TaskId::new("name-lookup-task".to_string()),
            requires_result: false,
        };

        let input = ExecuteActionInput {
            action: &action,
            conversation_id: AIConversationId::new(),
        };

        executor_handle.update(&mut app, |executor, ctx| {
            let result: AnyActionExecution = executor.execute(input, ctx).into();
            match result {
                AnyActionExecution::Sync(AIAgentActionResultType::ReadSkill(
                    ReadSkillResult::Success { content },
                )) => {
                    assert_eq!(content.file_name, skill_path.to_string_lossy().to_string());
                }
                _ => panic!("Lookup by name should succeed via Sync Success"),
            }
        });
    });
}

/// 未知 name(不在 SkillManager 索引中)走完所有 fallback 后:
/// `name_candidate` 命中但 `find_skill_by_name` 返回 None,继续到 fs fallback —
/// 此处路径形状不合法(纯 name 不含 `/`),直接 Sync Error。
#[test]
fn test_read_skill_executor_rejects_unknown_name() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let executor_handle = app.add_model(|_| ReadSkillExecutor::new());

        let action = AIAgentAction {
            id: AIAgentActionId::from("unknown-name-action".to_string()),
            action: AIAgentActionType::ReadSkill(ReadSkillRequest {
                skill: SkillReference::Path(std::path::PathBuf::from("no-such-skill")),
            }),
            task_id: TaskId::new("unknown-name-task".to_string()),
            requires_result: false,
        };

        let input = ExecuteActionInput {
            action: &action,
            conversation_id: AIConversationId::new(),
        };

        executor_handle.update(&mut app, |executor, ctx| {
            let result: AnyActionExecution = executor.execute(input, ctx).into();
            match result {
                AnyActionExecution::Sync(AIAgentActionResultType::ReadSkill(
                    ReadSkillResult::Error(msg),
                )) => {
                    assert!(msg.starts_with("Skill not found"), "msg={msg}");
                }
                _ => panic!("Unknown name should resolve to Sync Error"),
            }
        });
    });
}

/// Issue #99 安全门:cache 未命中时,若路径不匹配 skill 文件形状,
/// 直接走 Sync Error 分支,不触发任何磁盘读取。
#[test]
fn test_read_skill_executor_rejects_non_skill_path_on_cache_miss() {
    let temp_dir = TempDir::new().unwrap();
    // 一个不在 `.<provider>/skills/<name>/SKILL.md` 结构里的随机 markdown 文件。
    // 即使该文件存在,fallback 也不应读取它 —— extract_skill_parent_directory 会拒绝。
    let non_skill_path = temp_dir.path().join("random.md");
    fs::write(&non_skill_path, "not a skill").unwrap();

    App::test((), |mut app| async move {
        initialize_app(&mut app);
        let executor_handle = app.add_model(|_| ReadSkillExecutor::new());

        let action = AIAgentAction {
            id: AIAgentActionId::from("non-skill-action".to_string()),
            action: AIAgentActionType::ReadSkill(ReadSkillRequest {
                skill: SkillReference::Path(non_skill_path),
            }),
            task_id: TaskId::new("non-skill-task".to_string()),
            requires_result: false,
        };

        let input = ExecuteActionInput {
            action: &action,
            conversation_id: AIConversationId::new(),
        };

        executor_handle.update(&mut app, |executor, ctx| {
            let result: AnyActionExecution = executor.execute(input, ctx).into();
            match result {
                AnyActionExecution::Sync(AIAgentActionResultType::ReadSkill(
                    ReadSkillResult::Error(msg),
                )) => {
                    assert!(msg.starts_with("Skill not found"));
                }
                _ => panic!(
                    "Non-skill path on cache miss should return Sync Error, not Async fallback"
                ),
            }
        });
    });
}
