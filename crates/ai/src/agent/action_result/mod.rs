mod convert;

use std::{fmt::Display, ops::Range, time::SystemTime};

use itertools::Itertools as _;
use serde::{Deserialize, Serialize};
use warp_core::command::ExitCode;
use warp_multi_agent_api::apply_file_diffs_result::success::UpdatedFileContent;
use warp_terminal::model::BlockId;

use crate::{
    agent::FileLocations,
    document::{AIDocumentId, AIDocumentVersion},
};

#[derive(Debug, Clone, PartialEq)]
pub enum AIAgentActionResultType {
    /// The output of a requested command.
    RequestCommandOutput(RequestCommandOutputResult),

    /// The result of sending some input to a long-running command.
    WriteToLongRunningShellCommand(WriteToLongRunningShellCommandResult),

    /// The output of a requested file edits.
    RequestFileEdits(RequestFileEditsResult),

    /// The output of a read files action.
    ReadFiles(ReadFilesResult),

    /// The output of a grep action.
    Grep(GrepResult),

    /// The output of a file glob action.
    FileGlob(FileGlobResult),

    /// The output of a file glob V2 action.
    FileGlobV2(FileGlobV2Result),

    /// The output of reading an MCP resource.
    ReadMCPResource(ReadMCPResourceResult),

    /// The output of calling an MCP tool.
    CallMCPTool(CallMCPToolResult),

    /// The output of reading a skill.
    ReadSkill(ReadSkillResult),

    /// The output of suggesting a new conversation.
    SuggestNewConversation(SuggestNewConversationResult),

    /// The result of suggesting a prompt.
    SuggestPrompt(SuggestPromptResult),

    OpenCodeReview,

    InitProject,

    /// The output of a read documents action.
    ReadDocuments(ReadDocumentsResult),

    /// The output of an edit documents action.
    EditDocuments(EditDocumentsResult),

    /// The output of a create documents action.
    CreateDocuments(CreateDocumentsResult),

    /// The output of reading shell command output.
    ReadShellCommandOutput(ReadShellCommandOutputResult),

    /// The result of inserting code review comments.
    InsertReviewComments(InsertReviewCommentsResult),

    /// The output of transferring shell command control to the user.
    TransferShellCommandControlToUser(TransferShellCommandControlToUserResult),
    /// The result of asking the user a question.
    AskUserQuestion(AskUserQuestionResult),
}

impl AIAgentActionResultType {
    /// Returns the effective command string for command-related results, if any.
    ///
    /// This is used by UIs (e.g. requested command views) that want to display the
    /// final executed command rather than the original suggestion.
    pub fn command_str(&self) -> Option<&str> {
        match self {
            AIAgentActionResultType::RequestCommandOutput(
                RequestCommandOutputResult::Completed { command, .. },
            )
            | AIAgentActionResultType::RequestCommandOutput(
                RequestCommandOutputResult::LongRunningCommandSnapshot { command, .. },
            )
            | AIAgentActionResultType::ReadShellCommandOutput(
                ReadShellCommandOutputResult::CommandFinished { command, .. },
            )
            | AIAgentActionResultType::ReadShellCommandOutput(
                ReadShellCommandOutputResult::LongRunningCommandSnapshot { command, .. },
            ) => Some(command.as_str()),
            _ => None,
        }
    }
}

impl Display for AIAgentActionResultType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AIAgentActionResultType::RequestCommandOutput(result) => result.fmt(f),
            AIAgentActionResultType::WriteToLongRunningShellCommand(result) => result.fmt(f),
            AIAgentActionResultType::RequestFileEdits(result) => result.fmt(f),
            AIAgentActionResultType::ReadFiles(result) => result.fmt(f),
            AIAgentActionResultType::Grep(result) => result.fmt(f),
            AIAgentActionResultType::FileGlob(result) => result.fmt(f),
            AIAgentActionResultType::FileGlobV2(result) => result.fmt(f),
            AIAgentActionResultType::ReadMCPResource(result) => result.fmt(f),
            AIAgentActionResultType::CallMCPTool(result) => result.fmt(f),
            AIAgentActionResultType::ReadSkill(result) => result.fmt(f),
            AIAgentActionResultType::SuggestNewConversation(result) => result.fmt(f),
            AIAgentActionResultType::SuggestPrompt(result) => result.fmt(f),
            AIAgentActionResultType::ReadDocuments(result) => result.fmt(f),
            AIAgentActionResultType::EditDocuments(result) => result.fmt(f),
            AIAgentActionResultType::CreateDocuments(result) => result.fmt(f),
            AIAgentActionResultType::ReadShellCommandOutput(result) => result.fmt(f),
            AIAgentActionResultType::InsertReviewComments(result) => result.fmt(f),
            AIAgentActionResultType::TransferShellCommandControlToUser(result) => result.fmt(f),
            AIAgentActionResultType::AskUserQuestion(result) => result.fmt(f),
            AIAgentActionResultType::OpenCodeReview | AIAgentActionResultType::InitProject => {
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RequestCommandOutputResult {
    Completed {
        block_id: BlockId,
        command: String,
        output: String,
        exit_code: ExitCode,
    },
    LongRunningCommandSnapshot {
        block_id: BlockId,
        command: String,
        grid_contents: String,
        cursor: String,
        is_alt_screen_active: bool,
    },
    /// A running command canceled via ctrl-c
    /// would have Completed result with exit code 130.
    CancelledBeforeExecution,
    /// The command was denied because it was present on the denylist.
    Denylisted { command: String },
}

impl RequestCommandOutputResult {
    pub fn is_successful(&self) -> bool {
        match self {
            Self::Completed { exit_code, .. } => exit_code.was_successful(),
            Self::LongRunningCommandSnapshot { .. } => true,
            Self::CancelledBeforeExecution | Self::Denylisted { .. } => false,
        }
    }

    pub fn failed(&self) -> bool {
        match self {
            Self::Completed { exit_code, .. } => !exit_code.was_successful(),
            Self::Denylisted { .. } => true,
            Self::CancelledBeforeExecution | Self::LongRunningCommandSnapshot { .. } => false,
        }
    }
}

impl Display for RequestCommandOutputResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestCommandOutputResult::Completed {
                command,
                output,
                exit_code,
                ..
            } => {
                write!(
                    f,
                    "Command '{}' completed with exit code {}:\n{}",
                    command,
                    exit_code.value(),
                    output
                )
            }
            RequestCommandOutputResult::LongRunningCommandSnapshot { command, .. } => {
                write!(f, "Command '{command}' is long-running")
            }
            RequestCommandOutputResult::CancelledBeforeExecution => {
                write!(f, "Command output cancelled")
            }
            RequestCommandOutputResult::Denylisted { .. } => {
                write!(f, "Command output was on denylist")
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum ShellCommandError {
    BlockNotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum WriteToLongRunningShellCommandResult {
    Snapshot {
        block_id: BlockId,
        grid_contents: String,
        cursor: String,
        is_alt_screen_active: bool,
        is_preempted: bool,
    },
    CommandFinished {
        block_id: BlockId,
        output: String,
        exit_code: ExitCode,
    },
    Cancelled,
    Error(ShellCommandError),
}

impl Display for WriteToLongRunningShellCommandResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Snapshot { .. } => {
                write!(f, "Sent snapshot of long-running shell command to agent")
            }
            Self::CommandFinished {
                output, exit_code, ..
            } => write!(
                f,
                "Long-running shell command finished with exit code {}:\n{output}",
                exit_code.value()
            ),
            Self::Cancelled => write!(f, "Writing to long-running shell command cancelled"),
            Self::Error(e) => write!(f, "Write to long-running shell command failed: {e:?}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum SuggestNewConversationResult {
    Accepted { message_id: String },
    Rejected,
    Cancelled,
}

impl Display for SuggestNewConversationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Accepted { message_id } => {
                write!(
                    f,
                    "Suggest new conversation accepted for message {message_id}"
                )
            }
            Self::Rejected => {
                write!(f, "Suggest new conversation rejected for message")
            }
            Self::Cancelled => write!(f, "Suggest new conversation cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum AnyFileContent {
    StringContent(String),
    BinaryContent(Vec<u8>),
}

impl AnyFileContent {
    pub fn len(&self) -> usize {
        match self {
            Self::StringContent(content) => content.len(),
            Self::BinaryContent(content) => content.len(),
        }
    }

    pub fn line_count(&self) -> usize {
        match self {
            Self::StringContent(content) => content.lines().count(),
            Self::BinaryContent(_) => 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::StringContent(content) => content.is_empty(),
            Self::BinaryContent(content) => content.is_empty(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct FileContext {
    pub file_name: String,
    pub content: AnyFileContent,
    pub line_range: Option<Range<usize>>,
    pub last_modified: Option<SystemTime>,
    pub line_count: usize,
}

impl FileContext {
    // create a new FileContext and autocalculate number of lines in the given file
    pub fn new(
        file_name: String,
        content: AnyFileContent,
        line_range: Option<Range<usize>>,
        last_modified: Option<SystemTime>,
    ) -> Self {
        let string_content = if let AnyFileContent::StringContent(content) = content.clone() {
            content
        } else {
            return Self {
                file_name,
                content,
                line_range,
                last_modified,
                line_count: 0,
            };
        };

        let line_count = string_content.lines().count();

        Self {
            file_name,
            content: AnyFileContent::StringContent(string_content),
            line_range,
            last_modified,
            line_count,
        }
    }
}

impl Display for FileContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.line_range {
            None => write!(f, "{}", self.file_name),
            Some(range) => write!(f, "{} ({}-{})", self.file_name, range.start, range.end),
        }
    }
}

impl From<&FileContext> for FileLocations {
    fn from(context: &FileContext) -> Self {
        FileLocations {
            name: context.file_name.clone(),
            lines: context.line_range.clone().into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ReadFilesResult {
    Success { files: Vec<FileContext> },
    Error(String),
    Cancelled,
}

impl Display for ReadFilesResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadFilesResult::Success { files } => {
                write!(f, "Read files: {}", files.iter().format(", "))
            }
            ReadFilesResult::Error(error) => write!(f, "Read files error: {error}"),
            ReadFilesResult::Cancelled => write!(f, "Read files cancelled"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DocumentContext {
    pub document_id: AIDocumentId,
    pub document_version: AIDocumentVersion,
    pub content: String,
    pub line_ranges: Vec<Range<usize>>,
}

impl Display for DocumentContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line_ranges.is_empty() {
            return write!(f, "Document {}", self.document_id);
        }

        let line_ranges = self
            .line_ranges
            .iter()
            .map(|range| format!("{}-{}", range.start, range.end))
            .collect_vec();
        write!(
            f,
            "Document {} ({})",
            self.document_id,
            line_ranges.join(", ")
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ReadDocumentsResult {
    Success { documents: Vec<DocumentContext> },
    Error(String),
    Cancelled,
}

impl Display for ReadDocumentsResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadDocumentsResult::Success { documents } => {
                write!(f, "Read documents: {}", documents.iter().format(", "))
            }
            ReadDocumentsResult::Error(error) => write!(f, "Read documents error: {error}"),
            ReadDocumentsResult::Cancelled => write!(f, "Read documents cancelled"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum EditDocumentsResult {
    Success {
        updated_documents: Vec<DocumentContext>,
    },
    Error(String),
    Cancelled,
}

impl Display for EditDocumentsResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditDocumentsResult::Success { updated_documents } => {
                write!(
                    f,
                    "Edited documents: {}",
                    updated_documents.iter().format(", ")
                )
            }
            EditDocumentsResult::Error(error) => write!(f, "Edit documents error: {error}"),
            EditDocumentsResult::Cancelled => write!(f, "Edit documents cancelled"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CreateDocumentsResult {
    Success {
        created_documents: Vec<DocumentContext>,
    },
    Error(String),
    Cancelled,
}

impl Display for CreateDocumentsResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreateDocumentsResult::Success { created_documents } => {
                write!(
                    f,
                    "Created documents: {}",
                    created_documents.iter().format(", ")
                )
            }
            CreateDocumentsResult::Error(error) => write!(f, "Create documents error: {error}"),
            CreateDocumentsResult::Cancelled => write!(f, "Create documents cancelled"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ReadShellCommandOutputResult {
    CommandFinished {
        command: String,
        block_id: BlockId,
        output: String,
        exit_code: ExitCode,
    },
    LongRunningCommandSnapshot {
        command: String,
        block_id: BlockId,
        grid_contents: String,
        cursor: String,
        is_alt_screen_active: bool,
        is_preempted: bool,
    },
    Cancelled,
    Error(ShellCommandError),
}

impl Display for ReadShellCommandOutputResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadShellCommandOutputResult::CommandFinished {
                output, exit_code, ..
            } => {
                write!(
                    f,
                    "Shell command output finished with exit code{}:\n{output}",
                    exit_code.value()
                )
            }
            ReadShellCommandOutputResult::LongRunningCommandSnapshot { .. } => {
                write!(f, "Sent snapshot of long-running shell command to agent")
            }
            ReadShellCommandOutputResult::Cancelled => {
                write!(f, "Read shell command output cancelled")
            }
            ReadShellCommandOutputResult::Error(e) => {
                write!(f, "Read shell command output failed: {e:?}")
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RequestFileEditsResult {
    Success {
        diff: String,
        updated_files: Vec<UpdatedFileContext>,
        deleted_files: Vec<String>,
        lines_added: usize,
        lines_removed: usize,
    },
    Cancelled,
    /// Diff application failed.
    DiffApplicationFailed {
        error: String,
    },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UpdatedFileContext {
    pub was_edited_by_user: bool,
    pub file_context: FileContext,
}

impl Display for UpdatedFileContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "user_edited {}, file {}",
            self.was_edited_by_user, self.file_context
        )
    }
}

impl From<UpdatedFileContext> for Vec<UpdatedFileContent> {
    fn from(value: UpdatedFileContext) -> Self {
        // Note: This method only makes sense for FileContexts that have a string content.
        // TODO: How do we gracefully fail binary files here?
        let file_content: Vec<warp_multi_agent_api::FileContent> = value.file_context.into();

        file_content
            .into_iter()
            .map(|content| UpdatedFileContent {
                was_edited_by_user: value.was_edited_by_user,
                file: Some(content),
            })
            .collect()
    }
}

impl Display for RequestFileEditsResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestFileEditsResult::Success {
                diff,
                updated_files,
                ..
            } => {
                write!(
                    f,
                    "File edits completed:\n\tDiff:\n{diff}\n\tUpdatedFiles: [{}]",
                    updated_files.iter().format(", ")
                )
            }
            RequestFileEditsResult::Cancelled => write!(f, "File edits cancelled"),
            RequestFileEditsResult::DiffApplicationFailed { error } => {
                write!(f, "File edits failed: {error}")
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SuggestPromptResult {
    Accepted { query: String },
    Cancelled,
}

impl Display for SuggestPromptResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SuggestPromptResult::Accepted { query } => {
                write!(f, "Suggest prompt accepted: {query}")
            }
            SuggestPromptResult::Cancelled => write!(f, "Suggest prompt cancelled"),
        }
    }
}

impl AIAgentActionResultType {
    /// A user visible description of what the result contains.
    /// Used to display error messages when the content is too large for
    /// the LLM context window.
    pub fn description(&self) -> &str {
        match self {
            AIAgentActionResultType::RequestCommandOutput(_) => {
                "The output of your last command executed by Agent Mode"
            }
            AIAgentActionResultType::WriteToLongRunningShellCommand(_) => {
                "A snapshot of the command currently being executed by Agent Mode"
            }
            AIAgentActionResultType::RequestFileEdits(_) => {
                "The diff from editing the last file in Agent Mode"
            }
            AIAgentActionResultType::ReadFiles(_) => "The requested file content",
            AIAgentActionResultType::Grep(_) => "The results of the grep operation",
            AIAgentActionResultType::FileGlob(_) => "The results of the file glob operation",
            AIAgentActionResultType::FileGlobV2(_) => "The results of the file glob operation",
            AIAgentActionResultType::CallMCPTool(_) => "The MCP tool call",
            AIAgentActionResultType::ReadSkill(_) => "The results of reading a skill from file",
            AIAgentActionResultType::ReadMCPResource(_) => "The MCP resource",
            AIAgentActionResultType::SuggestNewConversation(_) => {
                "Your decision on whether to start a new conversation"
            }
            AIAgentActionResultType::SuggestPrompt(_) => "The suggested prompt",
            AIAgentActionResultType::OpenCodeReview => "Open code review",
            AIAgentActionResultType::InsertReviewComments(_) => "Insert code review comments",
            AIAgentActionResultType::InitProject => "Initialize project",
            AIAgentActionResultType::ReadDocuments(_) => "The requested document content",
            AIAgentActionResultType::EditDocuments(_) => "The edited document content",
            AIAgentActionResultType::CreateDocuments(_) => "The newly created documents",
            AIAgentActionResultType::ReadShellCommandOutput(_) => "The shell command output",
            AIAgentActionResultType::TransferShellCommandControlToUser(_) => {
                "The result of transferring shell command control to user"
            }
            AIAgentActionResultType::AskUserQuestion(_) => {
                "The user's answers to clarifying questions"
            }
        }
    }

    pub fn is_successful(&self) -> bool {
        match self {
            Self::RequestCommandOutput(r) => r.is_successful(),
            Self::RequestFileEdits(RequestFileEditsResult::Success { .. })
            | Self::ReadFiles(ReadFilesResult::Success { .. })
            | Self::Grep(GrepResult::Success { .. })
            | Self::FileGlob(FileGlobResult::Success { .. })
            | Self::FileGlobV2(FileGlobV2Result::Success { .. })
            | Self::ReadMCPResource(ReadMCPResourceResult::Success { .. })
            | Self::CallMCPTool(CallMCPToolResult::Success { .. })
            | Self::SuggestNewConversation(SuggestNewConversationResult::Accepted { .. })
            | Self::SuggestPrompt(SuggestPromptResult::Accepted { .. })
            | Self::ReadDocuments(ReadDocumentsResult::Success { .. })
            | Self::EditDocuments(EditDocumentsResult::Success { .. })
            | Self::CreateDocuments(CreateDocumentsResult::Success { .. })
            | Self::ReadShellCommandOutput(
                ReadShellCommandOutputResult::CommandFinished { .. }
                | ReadShellCommandOutputResult::LongRunningCommandSnapshot { .. },
            )
            | Self::InsertReviewComments(InsertReviewCommentsResult::Success { .. })
            | Self::OpenCodeReview
            | Self::ReadSkill(ReadSkillResult::Success { .. })
            | Self::TransferShellCommandControlToUser(
                TransferShellCommandControlToUserResult::Snapshot { .. }
                | TransferShellCommandControlToUserResult::CommandFinished { .. },
            ) => true,
            Self::AskUserQuestion(AskUserQuestionResult::Success { .. }) => true,
            _ => false,
        }
    }

    pub fn is_failed(&self) -> bool {
        match self {
            Self::RequestCommandOutput(r) => r.failed(),
            Self::RequestFileEdits(RequestFileEditsResult::DiffApplicationFailed { .. })
            | Self::ReadFiles(ReadFilesResult::Error(_))
            | Self::Grep(GrepResult::Error(_))
            | Self::FileGlob(FileGlobResult::Error(_))
            | Self::FileGlobV2(FileGlobV2Result::Error(_))
            | Self::ReadMCPResource(ReadMCPResourceResult::Error(_))
            | Self::CallMCPTool(CallMCPToolResult::Error(_))
            | Self::ReadDocuments(ReadDocumentsResult::Error(_))
            | Self::EditDocuments(EditDocumentsResult::Error(_))
            | Self::CreateDocuments(CreateDocumentsResult::Error(_))
            | Self::InsertReviewComments(InsertReviewCommentsResult::Error { .. })
            | Self::AskUserQuestion(AskUserQuestionResult::Error(_))
            | Self::TransferShellCommandControlToUser(
                TransferShellCommandControlToUserResult::Error(_),
            ) => true,
            _ => false,
        }
    }

    pub fn is_cancelled(&self) -> bool {
        match self {
            Self::RequestCommandOutput(RequestCommandOutputResult::CancelledBeforeExecution) => {
                true
            }
            Self::RequestCommandOutput(RequestCommandOutputResult::Completed {
                exit_code, ..
            }) if exit_code.value() == 130 => true,
            Self::RequestFileEdits(RequestFileEditsResult::Cancelled)
            | Self::ReadFiles(ReadFilesResult::Cancelled)
            | Self::Grep(GrepResult::Cancelled)
            | Self::FileGlob(FileGlobResult::Cancelled)
            | Self::FileGlobV2(FileGlobV2Result::Cancelled)
            | Self::ReadMCPResource(ReadMCPResourceResult::Cancelled)
            | Self::CallMCPTool(CallMCPToolResult::Cancelled)
            | Self::SuggestNewConversation(SuggestNewConversationResult::Cancelled)
            | Self::SuggestPrompt(SuggestPromptResult::Cancelled)
            | Self::ReadDocuments(ReadDocumentsResult::Cancelled)
            | Self::EditDocuments(EditDocumentsResult::Cancelled)
            | Self::CreateDocuments(CreateDocumentsResult::Cancelled)
            | Self::ReadShellCommandOutput(ReadShellCommandOutputResult::Cancelled)
            | Self::InsertReviewComments(InsertReviewCommentsResult::Cancelled)
            | Self::TransferShellCommandControlToUser(
                TransferShellCommandControlToUserResult::Cancelled,
            )
            | Self::WriteToLongRunningShellCommand(
                WriteToLongRunningShellCommandResult::Cancelled,
            )
            | Self::ReadSkill(ReadSkillResult::Cancelled)
            // SkippedByAutoApprove is intentionally excluded: the agent should continue.
            | Self::AskUserQuestion(AskUserQuestionResult::Cancelled) => true,
            _ => false,
        }
    }

    pub fn is_cancelled_during_requested_command_execution(&self) -> bool {
        matches!(self, Self::RequestCommandOutput(RequestCommandOutputResult::Completed {
            exit_code, ..
        }) if exit_code.value() == 130)
    }

    /// Returns `true` if this completion of this action result should trigger a follow-up request.
    pub fn should_trigger_request_upon_completion(&self) -> bool {
        !self.is_cancelled()
    }

    pub fn is_requested_command(&self) -> bool {
        matches!(self, AIAgentActionResultType::RequestCommandOutput(_))
    }

    pub fn is_call_mcp_tool(&self) -> bool {
        matches!(self, AIAgentActionResultType::CallMCPTool(_))
    }

    /// Returns `true` if this result will cause the server to route the next
    /// turn to a subagent (e.g. the CLI subagent) rather than the orchestrator.
    /// LRC snapshot variants are the current indicators of this.
    pub fn triggers_server_subagent(&self) -> bool {
        matches!(
            self,
            Self::RequestCommandOutput(
                RequestCommandOutputResult::LongRunningCommandSnapshot { .. }
            ) | Self::WriteToLongRunningShellCommand(
                WriteToLongRunningShellCommandResult::Snapshot { .. }
            ) | Self::ReadShellCommandOutput(
                ReadShellCommandOutputResult::LongRunningCommandSnapshot { .. }
            ) | Self::TransferShellCommandControlToUser(
                TransferShellCommandControlToUserResult::Snapshot { .. }
            )
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrepResult {
    Success { matched_files: Vec<GrepFileMatch> },
    Error(String),
    Cancelled,
}

impl Display for GrepResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrepResult::Success { matched_files } => {
                write!(
                    f,
                    "Grep found matches in: [{}]",
                    matched_files.iter().format(", ")
                )
            }
            GrepResult::Error(error) => write!(f, "Grep error: {error}"),
            GrepResult::Cancelled => write!(f, "Grep cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GrepFileMatch {
    /// The absolute path to the file that was matched.
    pub file_path: String,
    /// The lines that matched the query.
    pub matched_lines: Vec<GrepLineMatch>,
}

impl Display for GrepFileMatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} on lines [{}]",
            self.file_path,
            self.matched_lines.iter().format(", ")
        )
    }
}

/// Info about a line that matched the grep query. This only contains the line
/// number for now, but can be extended in the future to include more info, e.g.
/// line contents.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GrepLineMatch {
    /// The line number of the line that matched the query.
    pub line_number: usize,
}

impl Display for GrepLineMatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.line_number)
    }
}

/// The result of a file globbing operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileGlobResult {
    Success { matched_files: String },
    Error(String),
    Cancelled,
}

impl Display for FileGlobResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileGlobResult::Success { matched_files } => {
                write!(f, "File glob completed: {matched_files}")
            }
            FileGlobResult::Error(error) => write!(f, "File glob error: {error}"),
            FileGlobResult::Cancelled => write!(f, "File glob cancelled"),
        }
    }
}

// The result of a v2 file globbing operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileGlobV2Result {
    Success {
        matched_files: Vec<FileGlobV2Match>,
        warnings: Option<String>,
    },
    Error(String),
    Cancelled,
}

impl Display for FileGlobV2Result {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileGlobV2Result::Success {
                matched_files,
                warnings,
            } => {
                write!(
                    f,
                    "File glob V2 completed: [{}] warnings: {:?}",
                    matched_files.iter().format(", "),
                    warnings
                )
            }
            FileGlobV2Result::Error(error) => write!(f, "File glob V2 error: {error}"),
            FileGlobV2Result::Cancelled => write!(f, "File glob V2 cancelled"),
        }
    }
}

// A match of a single file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileGlobV2Match {
    pub file_path: String,
}

impl Display for FileGlobV2Match {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.file_path)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CallMCPToolResult {
    Success { result: rmcp::model::CallToolResult },
    Error(String),
    Cancelled,
}

impl Display for CallMCPToolResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallMCPToolResult::Success { result } => {
                write!(
                    f,
                    "MCP tool call completed: [{result:?}]",
                    // results.iter().format(", ")
                )
            }
            CallMCPToolResult::Error(error) => write!(f, "MCP tool call error: {error}"),
            CallMCPToolResult::Cancelled => write!(f, "MCP tool call cancelled"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReadMCPResourceResult {
    Success {
        resource_contents: Vec<rmcp::model::ResourceContents>,
    },
    Error(String),
    Cancelled,
}

impl Display for ReadMCPResourceResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadMCPResourceResult::Success { resource_contents } => {
                write!(f, "MCP resource read completed: [{resource_contents:?}]",)
            }
            ReadMCPResourceResult::Error(error) => write!(f, "MCP resource error: {error}"),
            ReadMCPResourceResult::Cancelled => write!(f, "MCP resource read cancelled"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReadSkillResult {
    Success { content: FileContext },
    Error(String),
    Cancelled,
}

impl Display for ReadSkillResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadSkillResult::Success { content } => {
                write!(f, "Skill read successfully: {}", content.file_name)
            }
            ReadSkillResult::Error(error) => write!(f, "Skill read error: {error}"),
            ReadSkillResult::Cancelled => write!(f, "Skill read cancelled"),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertReviewCommentsResult {
    Success { repo_path: String },
    Error { repo_path: String, message: String },
    Cancelled,
}

impl Display for InsertReviewCommentsResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InsertReviewCommentsResult::Success { repo_path } => {
                write!(f, "Inserted code review comments for {repo_path}")
            }
            InsertReviewCommentsResult::Error { repo_path, message } => {
                write!(
                    f,
                    "Error inserting code review comments for {repo_path}: {message}"
                )
            }
            InsertReviewCommentsResult::Cancelled => {
                write!(f, "Cancelled inserting code review comments")
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum TransferShellCommandControlToUserResult {
    Snapshot {
        block_id: BlockId,
        grid_contents: String,
        cursor: String,
        is_alt_screen_active: bool,
        is_preempted: bool,
    },
    CommandFinished {
        block_id: BlockId,
        output: String,
        exit_code: ExitCode,
    },
    Cancelled,
    Error(ShellCommandError),
}

impl Display for TransferShellCommandControlToUserResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Snapshot { .. } => {
                write!(f, "Transferred control to user, sent snapshot")
            }
            Self::CommandFinished {
                output, exit_code, ..
            } => write!(
                f,
                "Command finished while user had control, exit code {}:\n{output}",
                exit_code.value()
            ),
            Self::Cancelled => write!(f, "Transfer shell command control to user cancelled"),
            Self::Error(e) => write!(f, "Transfer shell command control to user failed: {e:?}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AskUserQuestionAnswerItem {
    Answered {
        question_id: String,
        selected_options: Vec<String>,
        other_text: String,
    },
    Skipped {
        question_id: String,
    },
}

impl AskUserQuestionAnswerItem {
    pub fn is_skipped(&self) -> bool {
        matches!(self, Self::Skipped { .. })
    }

    pub fn display_text(&self) -> String {
        match self {
            Self::Answered {
                selected_options,
                other_text,
                ..
            } => {
                let mut parts = selected_options.clone();
                if !other_text.is_empty() {
                    parts.push(format!("Other: {other_text}"));
                }
                parts.join(", ")
            }
            Self::Skipped { .. } => "Skipped".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AskUserQuestionResult {
    Success {
        answers: Vec<AskUserQuestionAnswerItem>,
    },
    Error(String),
    Cancelled,
    /// The question was skipped automatically because the conversation is in auto-approve mode.
    SkippedByAutoApprove {
        question_ids: Vec<String>,
    },
}

impl Display for AskUserQuestionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AskUserQuestionResult::Success { answers } => {
                write!(
                    f,
                    "Ask user question completed with {} answer(s)",
                    answers.len()
                )
            }
            AskUserQuestionResult::Error(msg) => write!(f, "Ask user question error: {msg}"),
            AskUserQuestionResult::Cancelled => write!(f, "Ask user question cancelled"),
            AskUserQuestionResult::SkippedByAutoApprove { question_ids } => {
                write!(
                    f,
                    "Ask user question skipped (auto-approve) with {} skipped question(s)",
                    question_ids.len()
                )
            }
        }
    }
}
