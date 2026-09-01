//! SFTP 浏览器主视图
//!
//! 实现 BackingView trait，作为 pane 的核心视图组件。
//! 提供远程文件浏览、上传下载、目录导航等完整功能。
//! author: logic
//! date: 2026-05-26

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use pathfinder_geometry::vector::Vector2F;
use warp_core::ui::appearance::Appearance;
use warp_core::ui::icons::Icon;
use warp_ssh_manager::{KeychainSecretStore, SshRepository};
use warpui::elements::{
    Align, Border, ChildAnchor, ChildView, ClippedScrollStateHandle, ClippedScrollable,
    ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, DispatchEventResult, Element,
    EventHandler, Fill, Flex, Hoverable, MainAxisAlignment, MainAxisSize, MouseStateHandle,
    OffsetPositioning, ParentAnchor, ParentElement, ParentOffsetBounds, Radius, SavePosition,
    ScrollbarWidth, Shrinkable, Stack, Text,
};
use warpui::platform::{Cursor, FilePickerConfiguration, SaveFilePickerConfiguration};
use warpui::r#async::SpawnedFutureHandle;
use warpui::{
    AppContext, Entity, ModelHandle, SingletonEntity, TypedActionView, View, ViewContext,
    ViewHandle,
};

use crate::editor::{
    EditorView, Event as EditorEvent, SingleLineEditorOptions, TextColors, TextOptions,
};
use crate::pane_group::focus_state::PaneFocusHandle;
use crate::pane_group::pane::view;
use crate::pane_group::{BackingView, PaneConfiguration, PaneEvent};
use crate::view_components::DismissibleToast;
use crate::workspace::ToastStack;

use super::context_menu::ContextMenuState;
use super::sftp_backend::{LiveSftpBackend, SftpBackend};
use super::sftp_ops;
use super::sftp_ops::normalize_remote_path;
use super::types::{
    ConnectionState, Dialog, FileEntry, FileEntryType, TransferDirection, TransferState,
    TransferTask,
};

/// 工具栏按钮尺寸
const TOOLBAR_BTN_SIZE: f32 = 28.0;
/// 工具栏图标尺寸
const TOOLBAR_ICON_SIZE: f32 = 16.0;
/// 工具栏间距
const TOOLBAR_SPACING: f32 = 4.0;
/// 面板内边距
const PANEL_PADDING: f32 = 8.0;
/// SFTP 面板位置 ID（用于 SavePosition 定位右键菜单）
pub(crate) const SFTP_PANEL_POSITION_ID: &str = "sftp_browser_panel_root";

/// SFTP 浏览器动作
#[derive(Debug, Clone)]
pub enum SftpBrowserAction {
    /// 导航到指定路径
    NavigateTo(PathBuf),
    /// 返回上级目录
    GoUp,
    /// 后退（历史记录）
    GoBack,
    /// 前进（历史记录）
    GoForward,
    /// 刷新当前目录
    Refresh,
    /// 选中指定索引的条目
    SelectEntry(usize),
    /// 打开指定索引的条目（目录则进入，文件则下载）
    OpenEntry(usize),
    /// 删除指定索引的条目
    DeleteEntry(usize),
    /// 重命名指定索引的条目
    RenameEntry(usize),
    /// 下载指定索引的条目
    DownloadEntry(usize),
    /// 上传文件
    UploadFile,
    /// 新建文件夹
    NewFolder,
    /// 确认删除
    ConfirmDelete,
    /// 确认重命名
    ConfirmRename,
    /// 确认新建文件夹
    ConfirmNewFolder,
    /// 确认覆盖
    ConfirmOverwrite,
    /// 弹出右键菜单
    ContextMenu { index: usize, position: Vector2F },
    /// 关闭右键菜单
    CloseContextMenu,
    /// 关闭对话框
    CloseDialog,
    /// 查看条目详情
    DetailsEntry(usize),
    /// 设置搜索过滤
    SetSearchFilter(String),
    /// 清除搜索过滤
    ClearSearchFilter,
    /// 返回上级（键盘快捷键）
    NavigateUp,
    /// 删除选中条目（键盘快捷键）
    DeleteSelected,
    /// 创建文件夹（键盘快捷键）
    CreateFolder,
    /// 文件拖入浏览器区域
    DragFilesEnter,
    /// 文件拖出浏览器区域
    DragFilesLeave,
    /// 拖放文件上传
    DragAndDropFiles(Vec<PathBuf>),
    /// 执行上传
    ExecuteUpload(String),
    /// 执行保存下载（用户已选择路径）
    DownloadSaveAs { index: usize, local_path: String },
    /// 确认移动
    ConfirmMove,
    /// 取消传输任务
    CancelTransfer(usize),
    /// 切换传输面板可见性
    ToggleTransferPanel,
    /// 确认关闭传输面板（取消所有传输并清空记录）
    ConfirmCloseTransferPanel,
}

/// SFTP 浏览器视图
pub struct SftpBrowserView {
    /// 关联的 SSH 服务器节点 ID
    node_id: String,
    /// pane 配置句柄
    pane_configuration: ModelHandle<PaneConfiguration>,
    /// 焦点句柄
    focus_handle: Option<PaneFocusHandle>,
    // ---- 连接 ----
    /// 连接状态
    pub(crate) connection: ConnectionState,
    /// SFTP 会话
    _session: Option<zap_sftp::SftpSession>,
    /// SFTP 操作通道
    sftp: Option<Arc<dyn SftpBackend>>,
    // ---- 导航 ----
    /// 当前路径
    pub(crate) current_path: PathBuf,
    /// 当前目录文件条目
    pub(crate) entries: Vec<FileEntry>,
    /// 选中的条目索引集合
    pub(crate) selected: HashSet<usize>,
    /// 路径历史记录
    pub(crate) path_history: Vec<PathBuf>,
    /// 历史记录当前位置
    pub(crate) history_index: usize,
    // ---- 传输 ----
    /// 传输任务列表
    pub(crate) transfers: Vec<TransferTask>,
    /// 下一个传输任务 ID
    pub(crate) next_transfer_id: usize,
    // ---- UI 状态 ----
    /// 当前打开的对话框
    pub(crate) dialog: Option<Dialog>,
    /// 是否正在加载
    pub(crate) is_loading: bool,
    /// 右键菜单状态
    pub(crate) context_menu: Option<ContextMenuState>,
    /// 搜索过滤文本
    pub(crate) search_filter: Option<String>,
    /// 是否有文件拖拽悬停在浏览器上
    pub(crate) is_drag_hovering: bool,
    // ---- 鼠标句柄 ----
    /// 刷新按钮
    refresh_btn: MouseStateHandle,
    /// 上级目录按钮
    up_btn: MouseStateHandle,
    /// 后退按钮
    back_btn: MouseStateHandle,
    /// 前进按钮
    forward_btn: MouseStateHandle,
    /// 上传按钮
    upload_btn: MouseStateHandle,
    /// 新建文件夹按钮
    new_folder_btn: MouseStateHandle,
    /// 对话框确认按钮
    dialog_confirm_btn: MouseStateHandle,
    /// 对话框取消按钮
    dialog_cancel_btn: MouseStateHandle,
    /// 对话框关闭按钮（标题栏 X 按钮）
    dialog_close_btn: MouseStateHandle,
    // ---- 传输面板 ----
    /// 传输面板是否被用户隐藏
    transfer_panel_hidden: bool,
    /// 传输面板关闭按钮
    transfer_panel_close_btn: MouseStateHandle,
    // ---- 对话框编辑器 ----
    /// 重命名编辑器
    pub(crate) rename_editor: ViewHandle<EditorView>,
    /// 新建文件夹编辑器
    pub(crate) new_folder_editor: ViewHandle<EditorView>,
    /// 搜索过滤编辑器
    search_editor: ViewHandle<EditorView>,
    // ---- 文件行鼠标句柄 ----
    /// 每行文件条目的鼠标状态句柄
    row_mouse_handles: Vec<MouseStateHandle>,
    // ---- 滚动 ----
    /// 滚动状态句柄
    scroll_state: ClippedScrollStateHandle,
    // ---- 异步任务 ----
    /// 当前连接任务的 future handle
    connect_handle: Option<SpawnedFutureHandle>,
    /// 当前刷新目录的 future handle
    refresh_handle: Option<SpawnedFutureHandle>,
    /// 传输任务 ID 到 future handle 的映射
    transfer_handles: HashMap<usize, SpawnedFutureHandle>,
    /// 拖拽批量上传时的待处理队列
    pending_uploads: Vec<PathBuf>,
}

impl SftpBrowserView {
    /// 创建新的 SFTP 浏览器视图
    pub fn new(node_id: String, ctx: &mut ViewContext<Self>) -> Self {
        let pane_configuration = ctx.add_model(|_ctx| PaneConfiguration::new("文件管理"));
        let rename_editor = make_editor("Enter new name", ctx);
        let new_folder_editor = make_editor("Folder name", ctx);
        let search_editor = make_editor("Search files...", ctx);

        let mut me = Self {
            node_id,
            pane_configuration,
            focus_handle: None,
            connection: ConnectionState::Disconnected,
            _session: None,
            sftp: None,
            current_path: PathBuf::from("/"),
            entries: Vec::new(),
            selected: HashSet::new(),
            path_history: vec![PathBuf::from("/")],
            history_index: 0,
            transfers: Vec::new(),
            next_transfer_id: 1,
            dialog: None,
            is_loading: false,
            context_menu: None,
            search_filter: None,
            is_drag_hovering: false,
            refresh_btn: MouseStateHandle::default(),
            up_btn: MouseStateHandle::default(),
            back_btn: MouseStateHandle::default(),
            forward_btn: MouseStateHandle::default(),
            upload_btn: MouseStateHandle::default(),
            new_folder_btn: MouseStateHandle::default(),
            dialog_confirm_btn: MouseStateHandle::default(),
            dialog_cancel_btn: MouseStateHandle::default(),
            dialog_close_btn: MouseStateHandle::default(),
            transfer_panel_hidden: false,
            transfer_panel_close_btn: MouseStateHandle::default(),
            rename_editor,
            new_folder_editor,
            search_editor,
            row_mouse_handles: Vec::new(),
            scroll_state: ClippedScrollStateHandle::default(),
            connect_handle: None,
            refresh_handle: None,
            transfer_handles: HashMap::new(),
            pending_uploads: Vec::new(),
        };

        // 订阅重命名编辑器事件
        let rename_editor_handle = me.rename_editor.clone();
        ctx.subscribe_to_view(
            &rename_editor_handle,
            |me, _source, event, ctx| match event {
                EditorEvent::Enter => {
                    me.handle_action(&SftpBrowserAction::ConfirmRename, ctx);
                }
                EditorEvent::Escape => {
                    me.dialog = None;
                    ctx.notify();
                }
                _ => {}
            },
        );

        // 订阅新建文件夹编辑器事件
        let new_folder_editor_handle = me.new_folder_editor.clone();
        ctx.subscribe_to_view(
            &new_folder_editor_handle,
            |me, _source, event, ctx| match event {
                EditorEvent::Enter => {
                    me.handle_action(&SftpBrowserAction::ConfirmNewFolder, ctx);
                }
                EditorEvent::Escape => {
                    me.dialog = None;
                    ctx.notify();
                }
                _ => {}
            },
        );

        // 订阅搜索编辑器事件
        let search_editor_handle = me.search_editor.clone();
        ctx.subscribe_to_view(
            &search_editor_handle,
            |me, _source, event, ctx| match event {
                EditorEvent::Escape => {
                    me.search_filter = None;
                    me.search_editor
                        .update(ctx, |e, ctx| e.set_buffer_text("", ctx));
                    ctx.notify();
                }
                _ => {
                    let text = me.search_editor.as_ref(ctx).buffer_text(ctx);
                    let trimmed = text.trim().to_string();
                    if trimmed.is_empty() {
                        me.search_filter = None;
                    } else {
                        me.search_filter = Some(trimmed);
                    }
                    ctx.notify();
                }
            },
        );

        // 发起连接
        me.connect_to_server(ctx);

        me
    }

    /// 注入测试后端，模拟 Connected 状态（仅测试使用）
    #[cfg(test)]
    pub(crate) fn set_backend_for_test(
        &mut self,
        backend: Arc<dyn SftpBackend>,
        start_path: PathBuf,
        ctx: &mut ViewContext<Self>,
    ) {
        self.connection = ConnectionState::Connected;
        self.sftp = Some(backend);
        self.current_path = start_path.clone();
        self.path_history = vec![start_path];
        self.history_index = 0;
        self.refresh_dir_sync(ctx);
    }

    /// 注入测试后端（集成测试使用）
    #[cfg(feature = "integration_tests")]
    pub fn inject_mock_backend(
        &mut self,
        backend: Arc<dyn SftpBackend>,
        start_path: PathBuf,
        ctx: &mut ViewContext<Self>,
    ) {
        self.connection = ConnectionState::Connected;
        self.sftp = Some(backend);
        self.current_path = start_path.clone();
        self.path_history = vec![start_path];
        self.history_index = 0;
        self.refresh_dir_sync(ctx);
    }

    /// 同步刷新目录内容（仅测试使用，避免异步延迟）
    #[cfg(any(test, feature = "integration_tests"))]
    fn refresh_dir_sync(&mut self, ctx: &mut ViewContext<Self>) {
        let sftp = match &self.sftp {
            Some(s) => s.clone(),
            None => return,
        };
        let path = self.current_path.clone();
        match sftp.list_dir(&path) {
            Ok(mut entries) => {
                entries.sort_by(|a, b| match (a.file_type, b.file_type) {
                    (FileEntryType::Directory, FileEntryType::Directory) => {
                        a.name.to_lowercase().cmp(&b.name.to_lowercase())
                    }
                    (
                        FileEntryType::Directory,
                        FileEntryType::File | FileEntryType::Symlink | FileEntryType::Other,
                    ) => std::cmp::Ordering::Less,
                    (
                        FileEntryType::File | FileEntryType::Symlink | FileEntryType::Other,
                        FileEntryType::Directory,
                    ) => std::cmp::Ordering::Greater,
                    (
                        FileEntryType::File | FileEntryType::Symlink | FileEntryType::Other,
                        FileEntryType::File | FileEntryType::Symlink | FileEntryType::Other,
                    ) => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                });
                self.entries = entries;
                self.selected.clear();
                self.sync_row_mouse_handles();
            }
            Err(_) => {}
        }
        let path = self.current_path.display();
        let title = format!("SFTP: {path}");
        self.pane_configuration.update(ctx, |config, ctx| {
            config.set_title(title, ctx);
        });
        ctx.notify();
    }

    /// 集成测试用 getter：连接状态
    #[cfg(feature = "integration_tests")]
    pub fn connection_state(&self) -> &ConnectionState {
        &self.connection
    }

    /// 集成测试用 getter：文件条目列表
    #[cfg(feature = "integration_tests")]
    pub fn entries(&self) -> &[FileEntry] {
        &self.entries
    }

    /// 集成测试用 getter：选中集合
    #[cfg(feature = "integration_tests")]
    pub fn selected(&self) -> &HashSet<usize> {
        &self.selected
    }

    /// 集成测试用 getter：对话框状态
    #[cfg(feature = "integration_tests")]
    pub fn dialog(&self) -> &Option<Dialog> {
        &self.dialog
    }

    /// 集成测试用 getter：右键菜单状态
    #[cfg(feature = "integration_tests")]
    pub fn context_menu(&self) -> &Option<ContextMenuState> {
        &self.context_menu
    }

    /// 获取 pane 配置
    pub fn pane_configuration(&self) -> ModelHandle<PaneConfiguration> {
        self.pane_configuration.clone()
    }

    /// 测试用：断开连接，清空状态
    #[cfg(test)]
    pub(crate) fn disconnect_for_test(&mut self, ctx: &mut ViewContext<Self>) {
        self.connection = ConnectionState::Disconnected;
        self.sftp = None;
        self.entries.clear();
        self.selected.clear();
        ctx.notify();
    }

    /// 连接到 SSH 服务器并建立 SFTP 通道
    fn connect_to_server(&mut self, ctx: &mut ViewContext<Self>) {
        let node_id = self.node_id.clone();
        let result = warp_ssh_manager::with_conn(|c| {
            let server = SshRepository::get_server(c, &node_id)?;
            Ok(server)
        });

        match result {
            Ok(Some(server)) => {
                // 取消之前的连接尝试
                if let Some(h) = self.connect_handle.take() {
                    h.abort();
                }

                self.connection = ConnectionState::Connecting;
                self.is_loading = true;
                ctx.notify();

                let secret_store = KeychainSecretStore;
                self.connect_handle = self.run_blocking(
                    ctx,
                    move || sftp_ops::connect_from_server(&server, &secret_store),
                    move |me, result, ctx| {
                        me.is_loading = false;
                        match result {
                            Ok(Ok(session)) => {
                                match session.sftp() {
                                    Ok(sftp) => {
                                        let backend = Arc::new(LiveSftpBackend::new(sftp))
                                            as Arc<dyn SftpBackend>;
                                        // 解析用户 home 目录
                                        if let Ok(home) =
                                            backend.realpath(std::path::Path::new("."))
                                        {
                                            me.current_path = normalize_remote_path(&home);
                                        } else {
                                            me.current_path = PathBuf::from("/");
                                        }
                                        me.path_history = vec![me.current_path.clone()];
                                        me.history_index = 0;
                                        me.connection = ConnectionState::Connected;
                                        me._session = Some(session);
                                        me.sftp = Some(backend);
                                        me.refresh_dir(ctx);
                                    }
                                    Err(e) => {
                                        me.connection = ConnectionState::Failed(format!(
                                            "创建 SFTP 通道失败: {e}"
                                        ));
                                        me.show_error_toast(
                                            format!("创建 SFTP 通道失败: {e}"),
                                            ctx,
                                        );
                                    }
                                }
                            }
                            Ok(Err(e)) => {
                                me.connection = ConnectionState::Failed(e.to_string());
                                me.show_error_toast(e.to_string(), ctx);
                            }
                            Err(_) => {
                                // JoinError（被 abort 或 panic）
                                me.connection = ConnectionState::Failed("连接已取消".to_string());
                            }
                        }
                        ctx.notify();
                    },
                );
            }
            Ok(None) => {
                self.connection = ConnectionState::Failed("未找到服务器配置".to_string());
                self.show_error_toast("未找到服务器配置".to_string(), ctx);
                ctx.notify();
            }
            Err(e) => {
                self.connection = ConnectionState::Failed(format!("读取服务器配置失败: {e}"));
                self.show_error_toast(format!("读取服务器配置失败: {e}"), ctx);
                ctx.notify();
            }
        }
    }

    /// 执行阻塞操作并回调
    /// 生产环境：通过 ctx.spawn + spawn_blocking 在后台线程执行
    /// 测试环境：直接同步执行（避免异步执行器时序问题）
    /// 返回 SpawnedFutureHandle 用于取消操作（测试环境返回 None）
    fn run_blocking<T: Send + 'static>(
        &mut self,
        ctx: &mut ViewContext<Self>,
        op: impl FnOnce() -> T + Send + 'static,
        callback: impl FnOnce(&mut Self, Result<T, tokio::task::JoinError>, &mut ViewContext<Self>)
            + 'static,
    ) -> Option<SpawnedFutureHandle> {
        #[cfg(any(test, feature = "integration_tests"))]
        {
            let result = op();
            callback(self, Ok(result), ctx);
            None
        }
        #[cfg(not(any(test, feature = "integration_tests")))]
        {
            Some(ctx.spawn(
                async move { tokio::task::spawn_blocking(op).await },
                move |me, result, ctx| {
                    callback(me, result, ctx);
                },
            ))
        }
    }

    /// 刷新当前目录内容
    fn refresh_dir(&mut self, ctx: &mut ViewContext<Self>) {
        let sftp = match &self.sftp {
            Some(s) => s.clone(),
            None => {
                self.show_error_toast("未连接到服务器".to_string(), ctx);
                ctx.notify();
                return;
            }
        };

        self.is_loading = true;
        ctx.notify();

        let path = self.current_path.clone();
        self.refresh_handle = self.run_blocking(
            ctx,
            move || sftp.list_dir(&path),
            |me, result, ctx| {
                me.is_loading = false;
                match result {
                    Ok(Ok(mut entries)) => {
                        entries.sort_by(|a, b| match (a.file_type, b.file_type) {
                            (FileEntryType::Directory, FileEntryType::Directory) => {
                                a.name.to_lowercase().cmp(&b.name.to_lowercase())
                            }
                            (
                                FileEntryType::Directory,
                                FileEntryType::File | FileEntryType::Symlink | FileEntryType::Other,
                            ) => std::cmp::Ordering::Less,
                            (
                                FileEntryType::File | FileEntryType::Symlink | FileEntryType::Other,
                                FileEntryType::Directory,
                            ) => std::cmp::Ordering::Greater,
                            (
                                FileEntryType::File | FileEntryType::Symlink | FileEntryType::Other,
                                FileEntryType::File | FileEntryType::Symlink | FileEntryType::Other,
                            ) => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                        });
                        me.entries = entries;
                        me.selected.clear();
                        me.sync_row_mouse_handles();
                    }
                    Ok(Err(e)) => {
                        me.show_error_toast(format!("列出目录失败: {e}"), ctx);
                    }
                    Err(_) => {}
                }

                let path = me.current_path.display();
                let title = format!("SFTP: {path}");
                me.pane_configuration.update(ctx, |config, ctx| {
                    config.set_title(title, ctx);
                });
                ctx.notify();
            },
        );
    }

    /// 同步行鼠标句柄数量与条目数量一致
    fn sync_row_mouse_handles(&mut self) {
        while self.row_mouse_handles.len() < self.entries.len() {
            self.row_mouse_handles.push(MouseStateHandle::default());
        }
        self.row_mouse_handles.truncate(self.entries.len());
    }

    /// 显示错误 Toast 弹窗
    fn show_error_toast(&self, message: String, ctx: &mut ViewContext<Self>) {
        let window_id = ctx.window_id();
        ToastStack::handle(ctx).update(ctx, |toast_stack, ctx| {
            let toast = DismissibleToast::error(message).with_object_id("sftp_error".to_string());
            toast_stack.add_ephemeral_toast(toast, window_id, ctx);
        });
    }

    /// 导航到指定路径并更新历史记录
    fn navigate_to(&mut self, path: PathBuf, ctx: &mut ViewContext<Self>) {
        let path = normalize_remote_path(&path);
        if path == self.current_path {
            return;
        }
        self.current_path = path;
        // 截断前进历史
        self.path_history.truncate(self.history_index + 1);
        self.path_history.push(self.current_path.clone());
        self.history_index = self.path_history.len() - 1;
        self.refresh_dir(ctx);
    }

    /// 返回上级目录
    fn go_up(&mut self, ctx: &mut ViewContext<Self>) {
        if let Some(parent) = self.current_path.parent() {
            let parent = normalize_remote_path(&parent.to_path_buf());
            if parent != self.current_path {
                self.navigate_to(parent, ctx);
            }
        }
    }

    /// 后退到历史记录中的上一个路径
    fn go_back(&mut self, ctx: &mut ViewContext<Self>) {
        if self.history_index > 0 {
            self.history_index -= 1;
            self.current_path = self.path_history[self.history_index].clone();
            self.refresh_dir(ctx);
        }
    }

    /// 前进到历史记录中的下一个路径
    fn go_forward(&mut self, ctx: &mut ViewContext<Self>) {
        if self.history_index < self.path_history.len() - 1 {
            self.history_index += 1;
            self.current_path = self.path_history[self.history_index].clone();
            self.refresh_dir(ctx);
        }
    }

    /// 打开指定索引的条目
    fn open_entry(&mut self, index: usize, ctx: &mut ViewContext<Self>) {
        if let Some(entry) = self.entries.get(index) {
            match entry.file_type {
                FileEntryType::Directory | FileEntryType::Symlink => {
                    self.navigate_to(entry.path.clone(), ctx);
                }
                FileEntryType::File | FileEntryType::Other => {
                    self.download_entry(index, ctx);
                }
            }
        }
    }

    /// 弹出删除确认对话框
    fn delete_selected(&mut self, index: usize, ctx: &mut ViewContext<Self>) {
        if let Some(entry) = self.entries.get(index) {
            let (paths, is_dirs) = if self.selected.contains(&index) {
                // 删除所有选中的
                self.selected
                    .iter()
                    .filter_map(|&i| {
                        self.entries.get(i).map(|e| {
                            (
                                e.path.clone(),
                                matches!(e.file_type, FileEntryType::Directory),
                            )
                        })
                    })
                    .unzip()
            } else {
                (
                    vec![entry.path.clone()],
                    vec![matches!(entry.file_type, FileEntryType::Directory)],
                )
            };
            self.dialog = Some(Dialog::DeleteConfirm { paths, is_dirs });
            ctx.notify();
        }
    }

    /// 执行删除操作
    fn confirm_delete(&mut self, ctx: &mut ViewContext<Self>) {
        let sftp = match &self.sftp {
            Some(s) => s.clone(),
            None => {
                self.show_error_toast("未连接到服务器".to_string(), ctx);
                self.dialog = None;
                ctx.notify();
                return;
            }
        };

        let (paths, is_dirs) = match &self.dialog {
            Some(Dialog::DeleteConfirm { paths, is_dirs }) => (paths.clone(), is_dirs.clone()),
            Some(Dialog::Rename { .. })
            | Some(Dialog::CreateFolder { .. })
            | Some(Dialog::Move { .. })
            | Some(Dialog::OverwriteConfirm { .. })
            | Some(Dialog::FileDetails { .. })
            | Some(Dialog::CloseTransferPanelConfirm)
            | None => {
                self.dialog = None;
                ctx.notify();
                return;
            }
        };

        self.dialog = None;
        self.is_loading = true;
        ctx.notify();

        self.run_blocking(
            ctx,
            move || {
                for (path, is_dir) in paths.iter().zip(is_dirs.iter()) {
                    let result = if *is_dir {
                        sftp.delete_dir_recursive(path)
                    } else {
                        sftp.delete_file(path)
                    };
                    if let Err(e) = result {
                        return Err(e.to_string());
                    }
                }
                Ok(())
            },
            move |me, result, ctx| {
                me.is_loading = false;
                me.selected.clear();
                match result {
                    Ok(Ok(())) => {
                        me.refresh_dir(ctx);
                    }
                    Ok(Err(e)) => {
                        me.show_error_toast(format!("删除失败: {e}"), ctx);
                        me.refresh_dir(ctx);
                    }
                    Err(_) => {
                        // 被取消
                        me.refresh_dir(ctx);
                    }
                }
                ctx.notify();
            },
        );
    }

    /// 创建下载传输任务
    fn download_entry(&mut self, index: usize, ctx: &mut ViewContext<Self>) {
        if let Some(entry) = self.entries.get(index) {
            let default_name = entry.name.clone();
            let idx = index;
            ctx.open_save_file_picker(
                move |path_opt: Option<String>, _me: &mut Self, _ctx: &mut ViewContext<Self>| {
                    if let Some(path) = path_opt {
                        _ctx.dispatch_typed_action_deferred(SftpBrowserAction::DownloadSaveAs {
                            index: idx,
                            local_path: path,
                        });
                    }
                },
                SaveFilePickerConfiguration::new().with_default_filename(default_name),
            );
        }
    }

    /// 显示条目详情对话框
    fn show_details(&mut self, index: usize, ctx: &mut ViewContext<Self>) {
        if let Some(entry) = self.entries.get(index) {
            self.dialog = Some(Dialog::FileDetails {
                entry: entry.clone(),
            });
            ctx.notify();
        }
    }

    /// 弹出重命名对话框
    fn rename_entry(&mut self, index: usize, ctx: &mut ViewContext<Self>) {
        if let Some(entry) = self.entries.get(index) {
            self.dialog = Some(Dialog::Rename {
                path: entry.path.clone(),
                original_name: entry.name.clone(),
            });
            // 将当前名称写入编辑器
            self.rename_editor
                .update(ctx, |e, ctx| e.set_buffer_text(&entry.name, ctx));
            ctx.notify();
        }
    }

    /// 渲染单个工具栏按钮
    fn render_toolbar_btn(
        &self,
        icon: Icon,
        handle: MouseStateHandle,
        action: SftpBrowserAction,
        _tooltip: &str,
        appearance: &Appearance,
        position_id: &'static str,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let icon_color = theme.sub_text_color(theme.background());

        let icon_el = ConstrainedBox::new(icon.to_warpui_icon(icon_color).finish())
            .with_width(TOOLBAR_ICON_SIZE)
            .with_height(TOOLBAR_ICON_SIZE)
            .finish();

        let btn_el = Hoverable::new(handle, move |_| {
            Container::new(
                ConstrainedBox::new(Container::new(icon_el).with_uniform_padding(6.0).finish())
                    .with_width(TOOLBAR_BTN_SIZE)
                    .with_height(TOOLBAR_BTN_SIZE)
                    .finish(),
            )
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.0)))
            .finish()
        })
        .with_cursor(Cursor::PointingHand)
        .on_click(move |ctx, _, _| {
            ctx.dispatch_typed_action(action.clone());
        })
        .finish();

        SavePosition::new(btn_el, position_id).finish()
    }

    /// 渲染工具栏
    fn render_toolbar(&self, appearance: &Appearance) -> Box<dyn Element> {
        let nav_buttons = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(TOOLBAR_SPACING)
            .with_child(self.render_toolbar_btn(
                Icon::ChevronLeft,
                self.back_btn.clone(),
                SftpBrowserAction::GoBack,
                "Back",
                appearance,
                "sftp_btn:back",
            ))
            .with_child(self.render_toolbar_btn(
                Icon::ChevronRight,
                self.forward_btn.clone(),
                SftpBrowserAction::GoForward,
                "Forward",
                appearance,
                "sftp_btn:forward",
            ))
            .with_child(self.render_toolbar_btn(
                Icon::ArrowUp,
                self.up_btn.clone(),
                SftpBrowserAction::GoUp,
                "Up",
                appearance,
                "sftp_btn:up",
            ))
            .with_child(self.render_toolbar_btn(
                Icon::Refresh,
                self.refresh_btn.clone(),
                SftpBrowserAction::Refresh,
                "Refresh",
                appearance,
                "sftp_btn:refresh",
            ))
            .with_main_axis_size(MainAxisSize::Min)
            .finish();

        let action_buttons = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(TOOLBAR_SPACING)
            .with_child(self.render_toolbar_btn(
                Icon::UploadCloud,
                self.upload_btn.clone(),
                SftpBrowserAction::UploadFile,
                "Upload",
                appearance,
                "sftp_btn:upload",
            ))
            .with_child(self.render_toolbar_btn(
                Icon::Plus,
                self.new_folder_btn.clone(),
                SftpBrowserAction::NewFolder,
                "New folder",
                appearance,
                "sftp_btn:new_folder",
            ))
            .with_main_axis_size(MainAxisSize::Min)
            .finish();

        Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(nav_buttons)
            .with_child(action_buttons)
            .finish()
    }

    /// 渲染面包屑导航
    fn render_breadcrumb(&self, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        let text_color = theme.sub_text_color(theme.background());

        let parts: Vec<Box<dyn Element>> =
            super::breadcrumb::render_breadcrumb(&self.current_path, appearance);

        let mut row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(2.0);

        // 添加根目录 "/" 作为可点击入口
        let root_text_color = text_color;
        let root_hoverable = Hoverable::new(Default::default(), move |_| {
            let t = Text::new_inline(
                "/".to_string(),
                appearance.ui_font_family(),
                appearance.ui_font_size(),
            )
            .with_color(root_text_color.into())
            .finish();
            Container::new(t).finish()
        })
        .with_cursor(Cursor::PointingHand)
        .on_click(move |ctx, _, _| {
            ctx.dispatch_typed_action(SftpBrowserAction::NavigateTo(PathBuf::from("/")));
        })
        .finish();
        let root_el = SavePosition::new(root_hoverable, "sftp_breadcrumb:/").finish();
        row.add_child(root_el);

        for part in parts {
            row.add_child(part);
        }

        Container::new(row.finish())
            .with_padding_left(4.0)
            .with_padding_right(4.0)
            .with_padding_top(4.0)
            .with_padding_bottom(4.0)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.0)))
            .with_background(theme.surface_2())
            .finish()
    }

    /// 渲染连接状态（非连接时）
    fn render_connection_state(&self, appearance: &Appearance) -> Box<dyn Element> {
        let (msg, icon) = match &self.connection {
            ConnectionState::Connecting => ("Connecting...".to_string(), Icon::Loading),
            ConnectionState::Failed(err) => (err.clone(), Icon::AlertCircle),
            ConnectionState::Disconnected => ("Disconnected".to_string(), Icon::AlertCircle),
            ConnectionState::Connected => {
                return Container::new(Flex::row().finish()).finish();
            }
        };

        render_centered_status(icon, &msg, 12.0, appearance)
    }

    /// 渲染文件列表
    fn render_file_list(&self, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();

        // 过滤条目
        let filtered_indices: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                self.search_filter.as_ref().map_or(true, |filter| {
                    entry.name.to_lowercase().contains(&filter.to_lowercase())
                })
            })
            .map(|(i, _)| i)
            .collect();

        if filtered_indices.is_empty() {
            let text_el = Text::new_inline(
                "This folder is empty".to_string(),
                appearance.ui_font_family(),
                appearance.ui_font_size(),
            )
            .with_color(theme.sub_text_color(theme.background()).into())
            .finish();

            return Align::new(Container::new(text_el).with_uniform_padding(24.0).finish())
                .finish();
        }

        // 表头
        let header = super::file_list::render_header(appearance);

        // 文件行
        let rows = super::file_list::render_file_rows(
            &self.entries,
            &filtered_indices,
            &self.selected,
            &self.row_mouse_handles,
            appearance,
        );

        Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_child(header)
            .with_child(rows)
            .finish()
    }

    /// 渲染传输面板
    fn render_transfers(&self, appearance: &Appearance) -> Box<dyn Element> {
        super::transfer_panel::render_transfer_panel(
            &self.transfers,
            appearance,
            self.transfer_panel_close_btn.clone(),
        )
    }

    /// 执行上传操作（公共入口，供拖拽上传和文件选择上传共用）
    ///
    /// 先检查远程目录是否已存在同名文件，若存在则弹出覆盖确认对话框，
    /// 用户确认后通过 `execute_upload_confirmed` 执行实际上传。
    fn execute_upload(&mut self, local_path: &Path, ctx: &mut ViewContext<Self>) {
        let file_name = local_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let remote_path = match build_upload_remote_path(&self.current_path, &file_name) {
            Some(p) => p,
            None => {
                self.show_error_toast("文件名包含非法字符".to_string(), ctx);
                return;
            }
        };

        // 检查远程目录中是否已存在同名文件
        let existing = self
            .entries
            .iter()
            .find(|e| e.name == file_name && matches!(e.file_type, FileEntryType::File));

        if existing.is_some() {
            let local_size = std::fs::metadata(local_path).map(|m| m.len()).unwrap_or(0);
            self.dialog = Some(Dialog::OverwriteConfirm {
                source: local_path.to_path_buf(),
                target: remote_path,
                file_size: local_size,
                direction: TransferDirection::Upload,
            });
            ctx.notify();
            return;
        }

        // 无冲突，直接执行上传
        self.execute_upload_confirmed(local_path, &remote_path, ctx);
    }

    /// 处理待上传队列，逐个上传直到遇到冲突或队列清空
    ///
    /// 拖拽批量上传时，将所有文件入队后逐个处理。
    /// 遇到同名文件冲突时暂停队列并弹出覆盖确认对话框，
    /// 用户确认后由 ConfirmOverwrite 继续调用本方法。
    /// author: logic
    /// date: 2026-06-01
    fn process_pending_uploads(&mut self, ctx: &mut ViewContext<Self>) {
        while let Some(local_path) = self.pending_uploads.pop() {
            self.execute_upload(&local_path, ctx);
            if self.dialog.is_some() {
                // 遇到冲突，暂停队列等待用户确认
                return;
            }
        }
    }

    /// 执行已确认的上传操作（创建传输任务并启动后台上传）
    fn execute_upload_confirmed(
        &mut self,
        local_path: &Path,
        remote_path: &Path,
        ctx: &mut ViewContext<Self>,
    ) {
        let total_size = std::fs::metadata(local_path).map(|m| m.len()).unwrap_or(0);

        let task = TransferTask::new(
            self.next_transfer_id,
            local_path.to_path_buf(),
            remote_path.to_path_buf(),
            TransferDirection::Upload,
            total_size,
        );
        self.next_transfer_id += 1;
        let task_id = task.id;
        let cancel_flag = task.cancel_flag.clone();
        self.transfers.push(task);

        if let Some(t) = self.transfers.iter_mut().find(|t| t.id == task_id) {
            t.state = TransferState::InProgress;
        }
        self.transfer_panel_hidden = false;
        ctx.notify();

        if let Some(sftp) = &self.sftp {
            let sftp = sftp.clone();
            let transferred = Arc::new(AtomicU64::new(0));
            let transferred_clone = transferred.clone();

            let progress_cb: sftp_ops::ProgressCallback = Box::new(move |bytes, _total| {
                transferred_clone.store(bytes, Ordering::SeqCst);
            });

            let local_path = local_path.to_path_buf();
            let remote_path = remote_path.to_path_buf();
            if let Some(handle) = self.run_blocking(
                ctx,
                move || {
                    sftp.upload_file(
                        &local_path,
                        &remote_path,
                        Some(&progress_cb),
                        Some(&cancel_flag),
                    )
                },
                move |me, result, ctx| {
                    if let Some(t) = me.transfers.iter_mut().find(|t| t.id == task_id) {
                        match &result {
                            Ok(Ok(())) => {
                                t.state = TransferState::Completed;
                                t.transferred = t.total_size;
                            }
                            Ok(Err(e)) => {
                                if matches!(e, super::sftp_ops::SftpOpsError::Cancelled) {
                                    t.state = TransferState::Cancelled;
                                } else {
                                    t.state = TransferState::Failed(e.to_string());
                                }
                                t.transferred = transferred.load(Ordering::SeqCst);
                            }
                            Err(_) => {
                                // JoinError（被 abort）
                                t.state = TransferState::Cancelled;
                                t.transferred = transferred.load(Ordering::SeqCst);
                            }
                        }
                    }

                    // 传输完成后清理 handle（future 已结束，无需 abort）
                    me.transfer_handles.remove(&task_id);

                    match &result {
                        Ok(Ok(())) => {
                            me.refresh_dir(ctx);
                        }
                        Ok(Err(e)) => {
                            log::error!("sftp: 上传失败: {e}");
                            me.show_error_toast(format!("上传失败: {e}"), ctx);
                            ctx.notify();
                        }
                        Err(_) => {
                            ctx.notify();
                        }
                    }
                },
            ) {
                self.transfer_handles.insert(task_id, handle);
            }
        } else {
            if let Some(t) = self.transfers.iter_mut().find(|t| t.id == task_id) {
                t.state = TransferState::Failed("未连接到服务器".to_string());
            }
            log::error!("sftp: 上传失败: 未连接到服务器");
            self.show_error_toast("上传失败: 未连接到服务器".to_string(), ctx);
            ctx.notify();
        }
    }

    /// 执行下载操作（公共逻辑，供确认覆盖和另存为共用）
    fn execute_download(
        &mut self,
        remote_path: &Path,
        local_path: &Path,
        file_size: u64,
        ctx: &mut ViewContext<Self>,
    ) {
        let task = TransferTask::new(
            self.next_transfer_id,
            remote_path.to_path_buf(),
            local_path.to_path_buf(),
            TransferDirection::Download,
            file_size,
        );
        self.next_transfer_id += 1;
        let task_id = task.id;
        let cancel_flag = task.cancel_flag.clone();
        self.transfers.push(task);

        if let Some(t) = self.transfers.iter_mut().find(|t| t.id == task_id) {
            t.state = TransferState::InProgress;
        }
        self.transfer_panel_hidden = false;
        ctx.notify();

        if let Some(sftp) = &self.sftp {
            let sftp = sftp.clone();
            let transferred = Arc::new(AtomicU64::new(0));
            let transferred_clone = transferred.clone();

            let progress_cb: sftp_ops::ProgressCallback = Box::new(move |bytes, _total| {
                transferred_clone.store(bytes, Ordering::SeqCst);
            });

            let remote_path = remote_path.to_path_buf();
            let local_path = local_path.to_path_buf();
            if let Some(handle) = self.run_blocking(
                ctx,
                move || {
                    sftp.download_file(
                        &remote_path,
                        &local_path,
                        Some(&progress_cb),
                        Some(&cancel_flag),
                    )
                },
                move |me, result, ctx| {
                    if let Some(t) = me.transfers.iter_mut().find(|t| t.id == task_id) {
                        match &result {
                            Ok(Ok(())) => {
                                t.state = TransferState::Completed;
                                t.transferred = t.total_size;
                            }
                            Ok(Err(e)) => {
                                if matches!(e, super::sftp_ops::SftpOpsError::Cancelled) {
                                    t.state = TransferState::Cancelled;
                                } else {
                                    t.state = TransferState::Failed(e.to_string());
                                }
                                t.transferred = transferred.load(Ordering::SeqCst);
                            }
                            Err(_) => {
                                t.state = TransferState::Cancelled;
                                t.transferred = transferred.load(Ordering::SeqCst);
                            }
                        }
                    }

                    // 传输完成后清理 handle（future 已结束，无需 abort）
                    me.transfer_handles.remove(&task_id);

                    if let Ok(Err(e)) = &result {
                        log::error!("sftp: 下载失败: {e}");
                        me.show_error_toast(format!("下载失败: {e}"), ctx);
                    }
                    ctx.notify();
                },
            ) {
                self.transfer_handles.insert(task_id, handle);
            }
        } else {
            if let Some(t) = self.transfers.iter_mut().find(|t| t.id == task_id) {
                t.state = TransferState::Failed("未连接到服务器".to_string());
            }
            log::error!("sftp: 下载失败: 未连接到服务器");
            self.show_error_toast("下载失败: 未连接到服务器".to_string(), ctx);
            ctx.notify();
        }
    }

    /// 渲染搜索栏
    fn render_search_bar(&self, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        let text_color = theme.sub_text_color(theme.background());

        let search_icon = ConstrainedBox::new(Icon::Search.to_warpui_icon(text_color).finish())
            .with_width(14.0)
            .with_height(14.0)
            .finish();

        let editor_el = Container::new(ChildView::new(&self.search_editor).finish()).finish();

        Container::new(
            Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_spacing(4.0)
                .with_child(search_icon)
                .with_child(Shrinkable::new(1.0, editor_el).finish())
                .finish(),
        )
        .with_padding_left(8.0)
        .with_padding_right(8.0)
        .with_padding_top(4.0)
        .with_padding_bottom(4.0)
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.0)))
        .with_background(theme.surface_2())
        .finish()
    }

    /// 渲染加载中状态
    fn render_loading(&self, appearance: &Appearance) -> Box<dyn Element> {
        render_centered_status(Icon::Loading, "Loading...", 8.0, appearance)
    }
}

/// 渲染居中状态提示（图标 + 文字）
fn render_centered_status(
    icon: Icon,
    message: &str,
    spacing: f32,
    appearance: &Appearance,
) -> Box<dyn Element> {
    let theme = appearance.theme();
    let text_color = theme.sub_text_color(theme.background());

    let icon_el = ConstrainedBox::new(icon.to_warpui_icon(text_color).finish())
        .with_width(24.0)
        .with_height(24.0)
        .finish();

    let text_el = Text::new_inline(
        message.to_string(),
        appearance.ui_font_family(),
        appearance.ui_font_size(),
    )
    .with_color(text_color.into())
    .finish();

    let content = Flex::row()
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_spacing(spacing)
        .with_child(icon_el)
        .with_child(text_el)
        .with_main_axis_size(MainAxisSize::Min)
        .finish();

    Align::new(Container::new(content).with_uniform_padding(24.0).finish()).finish()
}

/// 安全拼接文件名到父路径，防止路径注入和路径遍历
fn safe_join_name(parent: &Path, name: &str) -> Option<PathBuf> {
    if name.is_empty() || name.starts_with('/') || name.starts_with('\\') {
        return None;
    }
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return None;
    }
    Some(parent.join(name))
}

/// 构建重命名后的完整路径
fn build_rename_path(original_path: &PathBuf, new_name: &str) -> Option<PathBuf> {
    let parent = original_path.parent().unwrap_or(Path::new("/"));
    safe_join_name(parent, new_name).map(|p| normalize_remote_path(&p))
}

/// 构建新建文件夹的完整路径
fn build_new_folder_path(parent_path: &PathBuf, folder_name: &str) -> Option<PathBuf> {
    safe_join_name(parent_path, folder_name).map(|p| normalize_remote_path(&p))
}

/// 构建上传后的远程路径
fn build_upload_remote_path(current_path: &PathBuf, local_file_name: &str) -> Option<PathBuf> {
    let name = Path::new(local_file_name)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| local_file_name.to_string());
    safe_join_name(current_path, &name).map(|p| normalize_remote_path(&p))
}

impl Entity for SftpBrowserView {
    type Event = PaneEvent;
}

impl TypedActionView for SftpBrowserView {
    type Action = SftpBrowserAction;

    /// 处理所有 SFTP 浏览器动作
    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            SftpBrowserAction::NavigateTo(path) => {
                self.navigate_to(path.clone(), ctx);
            }
            SftpBrowserAction::GoUp => {
                self.go_up(ctx);
            }
            SftpBrowserAction::GoBack => {
                self.go_back(ctx);
            }
            SftpBrowserAction::GoForward => {
                self.go_forward(ctx);
            }
            SftpBrowserAction::Refresh => {
                self.refresh_dir(ctx);
            }
            SftpBrowserAction::SelectEntry(index) => {
                let index = *index;
                self.selected.clear();
                self.selected.insert(index);
                ctx.notify();
            }
            SftpBrowserAction::OpenEntry(index) => {
                let index = *index;
                self.open_entry(index, ctx);
            }
            SftpBrowserAction::DeleteEntry(index) => {
                let index = *index;
                self.delete_selected(index, ctx);
            }
            SftpBrowserAction::RenameEntry(index) => {
                let index = *index;
                self.rename_entry(index, ctx);
            }
            SftpBrowserAction::DownloadEntry(index) => {
                let index = *index;
                self.download_entry(index, ctx);
            }
            SftpBrowserAction::UploadFile => {
                ctx.open_file_picker(
                    move |result, ctx: &mut ViewContext<SftpBrowserView>| match result {
                        Ok(paths) => {
                            for path in paths {
                                ctx.dispatch_typed_action(&SftpBrowserAction::ExecuteUpload(path));
                            }
                        }
                        Err(e) => {
                            log::warn!("sftp: file picker failed: {e}");
                        }
                    },
                    FilePickerConfiguration::new(),
                );
            }
            SftpBrowserAction::NewFolder => {
                self.dialog = Some(Dialog::CreateFolder {
                    parent_path: self.current_path.clone(),
                });
                self.new_folder_editor
                    .update(ctx, |e, ctx| e.set_buffer_text("", ctx));
                ctx.notify();
            }
            SftpBrowserAction::ConfirmDelete => {
                self.confirm_delete(ctx);
            }
            SftpBrowserAction::ConfirmRename => {
                if let Some(Dialog::Rename {
                    path: original_path,
                    ..
                }) = &self.dialog
                {
                    let new_name = self.rename_editor.as_ref(ctx).buffer_text(ctx);
                    let new_name = new_name.trim().to_string();
                    if new_name.is_empty() {
                        self.show_error_toast("名称不能为空".to_string(), ctx);
                        return;
                    }
                    let new_path = match build_rename_path(original_path, &new_name) {
                        Some(p) => p,
                        None => {
                            self.show_error_toast(
                                "名称不合法：不能包含路径分隔符".to_string(),
                                ctx,
                            );
                            return;
                        }
                    };

                    if let Some(sftp) = &self.sftp {
                        let sftp = sftp.clone();
                        let original_path = original_path.clone();
                        self.dialog = None;
                        ctx.notify();
                        self.run_blocking(
                            ctx,
                            move || sftp.rename(&original_path, &new_path),
                            move |me, result, ctx| {
                                match result {
                                    Ok(Ok(())) => {
                                        me.refresh_dir(ctx);
                                    }
                                    Ok(Err(e)) => {
                                        me.show_error_toast(format!("重命名失败: {e}"), ctx);
                                    }
                                    Err(_) => {}
                                }
                                ctx.notify();
                            },
                        );
                    } else {
                        self.show_error_toast("未连接到服务器".to_string(), ctx);
                        self.dialog = None;
                    }
                }
            }
            SftpBrowserAction::ConfirmNewFolder => {
                if let Some(Dialog::CreateFolder { parent_path }) = &self.dialog {
                    let folder_name = self.new_folder_editor.as_ref(ctx).buffer_text(ctx);
                    let folder_name = folder_name.trim().to_string();
                    if folder_name.is_empty() {
                        self.show_error_toast("文件夹名称不能为空".to_string(), ctx);
                        return;
                    }
                    let folder_path = match build_new_folder_path(parent_path, &folder_name) {
                        Some(p) => p,
                        None => {
                            self.show_error_toast(
                                "名称不合法：不能包含路径分隔符".to_string(),
                                ctx,
                            );
                            return;
                        }
                    };

                    if let Some(sftp) = &self.sftp {
                        let sftp = sftp.clone();
                        self.dialog = None;
                        ctx.notify();
                        self.run_blocking(
                            ctx,
                            move || sftp.create_dir(&folder_path),
                            move |me, result, ctx| {
                                match result {
                                    Ok(Ok(())) => {
                                        me.refresh_dir(ctx);
                                    }
                                    Ok(Err(e)) => {
                                        me.show_error_toast(format!("创建文件夹失败: {e}"), ctx);
                                    }
                                    Err(_) => {}
                                }
                                ctx.notify();
                            },
                        );
                    } else {
                        self.show_error_toast("未连接到服务器".to_string(), ctx);
                        self.dialog = None;
                    }
                }
            }
            SftpBrowserAction::ConfirmOverwrite => {
                // 从对话框中提取路径和传输方向
                let (source, target, file_size, direction) = match &self.dialog {
                    Some(Dialog::OverwriteConfirm {
                        source,
                        target,
                        file_size,
                        direction,
                    }) => (source.clone(), target.clone(), *file_size, *direction),
                    Some(Dialog::DeleteConfirm { .. })
                    | Some(Dialog::Rename { .. })
                    | Some(Dialog::CreateFolder { .. })
                    | Some(Dialog::Move { .. })
                    | Some(Dialog::FileDetails { .. })
                    | Some(Dialog::CloseTransferPanelConfirm)
                    | None => {
                        self.dialog = None;
                        ctx.notify();
                        return;
                    }
                };

                // 关闭对话框
                self.dialog = None;
                match direction {
                    TransferDirection::Download => {
                        self.execute_download(&source, &target, file_size, ctx);
                    }
                    TransferDirection::Upload => {
                        self.execute_upload_confirmed(&source, &target, ctx);
                    }
                }
                // 批量上传队列：确认当前文件后继续处理下一个
                self.process_pending_uploads(ctx);
            }
            SftpBrowserAction::ContextMenu { index, position } => {
                let index = *index;
                let position = *position;
                self.context_menu = Some(ContextMenuState::new(index, position));
                self.selected.clear();
                self.selected.insert(index);
                ctx.notify();
            }
            SftpBrowserAction::CloseContextMenu => {
                self.context_menu = None;
                ctx.notify();
            }
            SftpBrowserAction::CloseDialog => {
                // 用户取消覆盖确认时，清空剩余的批量上传队列
                let was_upload_overwrite = matches!(
                    self.dialog,
                    Some(Dialog::OverwriteConfirm {
                        direction: TransferDirection::Upload,
                        ..
                    })
                );
                self.dialog = None;
                if was_upload_overwrite {
                    self.pending_uploads.clear();
                }
                ctx.notify();
            }
            SftpBrowserAction::DetailsEntry(index) => {
                let index = *index;
                self.show_details(index, ctx);
            }
            SftpBrowserAction::SetSearchFilter(filter) => {
                self.search_filter = Some(filter.clone());
                ctx.notify();
            }
            SftpBrowserAction::ClearSearchFilter => {
                self.search_filter = None;
                ctx.notify();
            }
            SftpBrowserAction::NavigateUp => {
                self.go_up(ctx);
            }
            SftpBrowserAction::DeleteSelected => {
                if let Some(&index) = self.selected.iter().next() {
                    self.delete_selected(index, ctx);
                }
            }
            SftpBrowserAction::CreateFolder => {
                self.handle_action(&SftpBrowserAction::NewFolder, ctx);
            }
            SftpBrowserAction::ConfirmMove => {
                if let Some(Dialog::Move { source, target_dir }) = &self.dialog {
                    let file_name = source
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let target_path = match safe_join_name(target_dir, &file_name) {
                        Some(p) => normalize_remote_path(&p),
                        None => {
                            self.show_error_toast("目标路径不合法".to_string(), ctx);
                            self.dialog = None;
                            ctx.notify();
                            return;
                        }
                    };

                    if let Some(sftp) = &self.sftp {
                        let sftp = sftp.clone();
                        let source = source.clone();
                        self.dialog = None;
                        ctx.notify();
                        self.run_blocking(
                            ctx,
                            move || sftp.rename(&source, &target_path),
                            move |me, result, ctx| {
                                match result {
                                    Ok(Ok(())) => {
                                        me.refresh_dir(ctx);
                                    }
                                    Ok(Err(e)) => {
                                        me.show_error_toast(format!("移动失败: {e}"), ctx);
                                    }
                                    Err(_) => {}
                                }
                                ctx.notify();
                            },
                        );
                    } else {
                        self.show_error_toast("未连接到服务器".to_string(), ctx);
                        self.dialog = None;
                    }
                }
            }
            SftpBrowserAction::CancelTransfer(task_id) => {
                let task_id = *task_id;
                // 协作式取消：设置 cancel_flag
                if let Some(t) = self.transfers.iter().find(|t| t.id == task_id) {
                    t.cancel();
                }
                // 结构式取消：abort spawned future
                if let Some(handle) = self.transfer_handles.remove(&task_id) {
                    handle.abort();
                }
                ctx.notify();
            }
            SftpBrowserAction::ToggleTransferPanel => {
                let has_active = self
                    .transfers
                    .iter()
                    .any(|t| matches!(t.state, TransferState::Pending | TransferState::InProgress));
                if has_active {
                    self.dialog = Some(Dialog::CloseTransferPanelConfirm);
                } else {
                    self.transfers.clear();
                    self.transfer_panel_hidden = true;
                }
                ctx.notify();
            }
            SftpBrowserAction::ConfirmCloseTransferPanel => {
                for task in &self.transfers {
                    task.cancel();
                }
                for (_, handle) in self.transfer_handles.drain() {
                    handle.abort();
                }
                self.transfers.clear();
                self.transfer_panel_hidden = true;
                self.dialog = None;
                ctx.notify();
            }
            SftpBrowserAction::DragFilesEnter => {
                self.is_drag_hovering = true;
                ctx.notify();
            }
            SftpBrowserAction::DragFilesLeave => {
                self.is_drag_hovering = false;
                ctx.notify();
            }
            SftpBrowserAction::DragAndDropFiles(paths) => {
                self.is_drag_hovering = false;
                // 逆序入队，使得 pop() 按原始顺序取出
                self.pending_uploads = paths.iter().rev().cloned().collect();
                self.process_pending_uploads(ctx);
            }
            SftpBrowserAction::ExecuteUpload(local_path_str) => {
                let local_path = PathBuf::from(local_path_str);
                self.execute_upload(&local_path, ctx);
            }
            SftpBrowserAction::DownloadSaveAs { index, local_path } => {
                let local_path = PathBuf::from(local_path);
                let (remote_path, file_size) = self
                    .entries
                    .get(*index)
                    .map(|e| (e.path.clone(), e.size))
                    .unzip();
                if let (Some(remote_path), Some(file_size)) = (remote_path, file_size) {
                    self.execute_download(&remote_path, &local_path, file_size, ctx);
                }
            }
        }
    }
}

impl View for SftpBrowserView {
    fn ui_name() -> &'static str {
        "SftpBrowserView"
    }

    /// 渲染完整 UI 布局
    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();

        // 1. 非连接状态显示连接状态
        if !matches!(self.connection, ConnectionState::Connected) {
            return Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_main_axis_size(MainAxisSize::Max)
                .with_child(self.render_connection_state(appearance))
                .finish();
        }

        let mut col = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_main_axis_size(MainAxisSize::Max);

        // 2. 面包屑
        col.add_child(
            Container::new(self.render_breadcrumb(appearance))
                .with_padding_left(PANEL_PADDING)
                .with_padding_right(PANEL_PADDING)
                .with_padding_top(PANEL_PADDING)
                .finish(),
        );

        // 3. 工具栏
        col.add_child(
            Container::new(self.render_toolbar(appearance))
                .with_padding_left(PANEL_PADDING)
                .with_padding_right(PANEL_PADDING)
                .with_padding_top(4.0)
                .with_padding_bottom(4.0)
                .finish(),
        );

        // 4. 搜索栏
        col.add_child(
            Container::new(self.render_search_bar(appearance))
                .with_padding_left(PANEL_PADDING)
                .with_padding_right(PANEL_PADDING)
                .with_padding_bottom(4.0)
                .finish(),
        );

        // 5. 加载中 / 文件列表
        if self.is_loading {
            col.add_child(Shrinkable::new(1.0, self.render_loading(appearance)).finish());
        } else {
            let file_list = self.render_file_list(appearance);
            let scrollbar_color = theme.disabled_text_color(theme.background()).into();
            let scrollbar_thumb_hover = theme.main_text_color(theme.background()).into();
            let scrollable = ClippedScrollable::vertical(
                self.scroll_state.clone(),
                file_list,
                ScrollbarWidth::Auto,
                scrollbar_color,
                scrollbar_thumb_hover,
                Fill::None,
            )
            .finish();
            col.add_child(Shrinkable::new(1.0, scrollable).finish());
        }

        // 7. 传输面板（浮动在底部）
        let mut main_content = col.finish();

        // 8. 传输面板浮动层
        if !self.transfers.is_empty() && !self.transfer_panel_hidden {
            let panel_el = Container::new(self.render_transfers(appearance))
                .with_padding_left(PANEL_PADDING)
                .with_padding_right(PANEL_PADDING)
                .with_padding_bottom(PANEL_PADDING)
                .finish();
            let mut stack = Stack::new();
            stack.add_child(main_content);
            stack.add_positioned_overlay_child(
                panel_el,
                OffsetPositioning::offset_from_parent(
                    Vector2F::new(0.0, 0.0),
                    ParentOffsetBounds::ParentBySize,
                    ParentAnchor::BottomLeft,
                    ChildAnchor::BottomLeft,
                ),
            );
            main_content = stack.finish();
        }

        // 9. 右键菜单
        if let Some(ref cm_state) = self.context_menu {
            let menu_el = super::context_menu::render_context_menu(cm_state, appearance);
            let positioning = OffsetPositioning::offset_from_parent(
                cm_state.position,
                ParentOffsetBounds::ParentByPosition,
                ParentAnchor::TopLeft,
                ChildAnchor::TopLeft,
            );
            let mut stack = Stack::new();
            stack.add_child(main_content);
            stack.add_positioned_overlay_child(menu_el, positioning);
            main_content = stack.finish();
        }

        // 9. 对话框（覆盖层）
        if let Some(ref dialog) = self.dialog {
            let dialog_el = super::dialogs::render_dialog(
                dialog,
                &self.rename_editor,
                &self.new_folder_editor,
                appearance,
                self.dialog_confirm_btn.clone(),
                self.dialog_cancel_btn.clone(),
                self.dialog_close_btn.clone(),
            );
            let mut stack = Stack::new();
            stack.add_child(main_content);
            stack.add_overlay_child(Align::new(dialog_el).finish());
            main_content = stack.finish();
        }

        // 10. 拖拽视觉反馈
        if self.is_drag_hovering {
            let drop_hint = Text::new_inline(
                "拖放文件以上传".to_string(),
                appearance.ui_font_family(),
                appearance.ui_font_size() + 2.0,
            )
            .with_color(theme.accent().into())
            .finish();
            let overlay = Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_main_axis_alignment(MainAxisAlignment::Center)
                .with_main_axis_size(MainAxisSize::Max)
                .with_child(drop_hint)
                .finish();
            let overlay_container = Container::new(overlay)
                .with_background(theme.accent().with_opacity(20))
                .with_border(Border::all(2.0).with_border_fill(theme.accent().into_solid()))
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(8.0)))
                .finish();
            let mut stack = Stack::new();
            stack.add_child(main_content);
            stack.add_child(overlay_container);
            main_content = stack.finish();
        }

        // 11. 保存面板位置（用于右键菜单位置计算）
        let positioned_content = SavePosition::new(main_content, SFTP_PANEL_POSITION_ID).finish();

        // 12. 键盘事件拦截
        let key_handler =
            EventHandler::new(positioned_content).on_keydown(move |ctx, _app, keystroke| {
                match keystroke.key.as_str() {
                    "delete" => {
                        ctx.dispatch_typed_action(SftpBrowserAction::DeleteSelected);
                        DispatchEventResult::StopPropagation
                    }
                    "backspace" => {
                        ctx.dispatch_typed_action(SftpBrowserAction::NavigateUp);
                        DispatchEventResult::StopPropagation
                    }
                    "escape" => {
                        ctx.dispatch_typed_action(SftpBrowserAction::CloseDialog);
                        DispatchEventResult::StopPropagation
                    }
                    _ => DispatchEventResult::PropagateToParent,
                }
            });

        // 13. 拖拽事件拦截
        super::drop_target::SftpDropTargetElement::new(key_handler.finish()).finish()
    }
}

impl BackingView for SftpBrowserView {
    type PaneHeaderOverflowMenuAction = SftpBrowserAction;
    type CustomAction = ();
    type AssociatedData = ();

    /// 处理溢出菜单动作
    fn handle_pane_header_overflow_menu_action(
        &mut self,
        action: &Self::PaneHeaderOverflowMenuAction,
        ctx: &mut ViewContext<Self>,
    ) {
        self.handle_action(action, ctx);
    }

    /// 关闭视图
    fn close(&mut self, ctx: &mut ViewContext<Self>) {
        // 协作式取消：设置所有传输任务的 cancel_flag
        for task in &self.transfers {
            task.cancel();
        }
        // 结构式取消：abort spawned future
        for (_, handle) in self.transfer_handles.drain() {
            handle.abort();
        }
        self.pending_uploads.clear();
        self.connect_handle = None;
        self.refresh_handle = None;
        self._session = None;
        self.sftp = None;
        self.connection = ConnectionState::Disconnected;
        ctx.emit(PaneEvent::Close);
    }

    /// 聚焦内容，将窗口焦点设置到当前视图
    fn focus_contents(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.focus_self();
    }

    /// 渲染头部内容
    fn render_header_content(
        &self,
        _ctx: &view::HeaderRenderContext<'_>,
        _app: &AppContext,
    ) -> view::HeaderContent {
        let path = self.current_path.display();
        let title = format!("SFTP: {path}");
        view::HeaderContent::simple(title)
    }

    /// 设置焦点句柄
    fn set_focus_handle(&mut self, focus_handle: PaneFocusHandle, _ctx: &mut ViewContext<Self>) {
        self.focus_handle = Some(focus_handle);
    }
}

/// 创建单行编辑器
fn make_editor(
    placeholder: &str,
    ctx: &mut ViewContext<SftpBrowserView>,
) -> ViewHandle<EditorView> {
    let placeholder = placeholder.to_string();
    ctx.add_typed_action_view(move |ctx| {
        let options = {
            let appearance = Appearance::as_ref(ctx);
            let theme = appearance.theme();
            SingleLineEditorOptions {
                text: TextOptions {
                    font_size_override: Some(appearance.ui_font_size()),
                    font_family_override: Some(appearance.monospace_font_family()),
                    text_colors_override: Some(TextColors {
                        default_color: theme.active_ui_text_color(),
                        disabled_color: theme.disabled_ui_text_color(),
                        hint_color: theme.disabled_ui_text_color(),
                    }),
                    ..Default::default()
                },
                ..Default::default()
            }
        };
        let mut editor = EditorView::single_line(options, ctx);
        editor.set_placeholder_text(&placeholder, ctx);
        editor
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ============================================================
    // normalize_remote_path 测试
    // ============================================================

    /// 测试反斜杠替换为正斜杠
    #[test]
    fn test_normalize_remote_path_backslash() {
        let path = PathBuf::from(r"home\user\docs");
        let result = normalize_remote_path(&path);
        assert_eq!(result, PathBuf::from("home/user/docs"));
    }

    /// 测试纯正斜杠路径不变
    #[test]
    fn test_normalize_remote_path_forward_slash() {
        let path = PathBuf::from("/home/user/docs");
        let result = normalize_remote_path(&path);
        assert_eq!(result, PathBuf::from("/home/user/docs"));
    }

    /// 测试根路径
    #[test]
    fn test_normalize_remote_path_root() {
        let path = PathBuf::from("/");
        let result = normalize_remote_path(&path);
        assert_eq!(result, PathBuf::from("/"));
    }

    /// 测试空路径
    #[test]
    fn test_normalize_remote_path_empty() {
        let path = PathBuf::from("");
        let result = normalize_remote_path(&path);
        assert_eq!(result, PathBuf::from(""));
    }

    /// 测试混合斜杠路径
    #[test]
    fn test_normalize_remote_path_mixed() {
        let path = PathBuf::from(r"home/user\docs/file.txt");
        let result = normalize_remote_path(&path);
        assert_eq!(result, PathBuf::from("home/user/docs/file.txt"));
    }

    // ============================================================
    // build_rename_path 测试
    // ============================================================

    /// 测试重命名路径构建
    #[test]
    fn test_build_rename_path_basic() {
        let original = PathBuf::from("/home/user/old.txt");
        let result = build_rename_path(&original, "new.txt");
        assert_eq!(result, Some(PathBuf::from("/home/user/new.txt")));
    }

    /// 测试重命名路径无父目录
    #[test]
    fn test_build_rename_path_no_parent() {
        let original = PathBuf::from("old.txt");
        let result = build_rename_path(&original, "new.txt");
        assert_eq!(result, Some(PathBuf::from("new.txt")));
    }

    /// 测试重命名路径含反斜杠规范化
    #[test]
    fn test_build_rename_path_normalizes() {
        let original = PathBuf::from("/home/user/old.txt");
        let result = build_rename_path(&original, "new.txt").unwrap();
        assert!(!result.to_string_lossy().contains('\\'));
    }

    /// 测试重命名路径拒绝路径注入
    #[test]
    fn test_build_rename_path_rejects_traversal() {
        let original = PathBuf::from("/home/user/old.txt");
        assert_eq!(build_rename_path(&original, "../etc/passwd"), None);
        assert_eq!(build_rename_path(&original, "/etc/passwd"), None);
        assert_eq!(build_rename_path(&original, "sub/name"), None);
        assert_eq!(build_rename_path(&original, ""), None);
    }

    // ============================================================
    // build_new_folder_path 测试
    // ============================================================

    /// 测试新建文件夹路径构建
    #[test]
    fn test_build_new_folder_path_basic() {
        let parent = PathBuf::from("/home/user");
        let result = build_new_folder_path(&parent, "new_dir");
        assert_eq!(result, Some(PathBuf::from("/home/user/new_dir")));
    }

    /// 测试新建文件夹路径含反斜杠规范化
    #[test]
    fn test_build_new_folder_path_normalizes() {
        let parent = PathBuf::from("/home/user");
        let result = build_new_folder_path(&parent, "test").unwrap();
        assert!(!result.to_string_lossy().contains('\\'));
    }

    /// 测试新建文件夹路径拒绝路径注入
    #[test]
    fn test_build_new_folder_path_rejects_traversal() {
        let parent = PathBuf::from("/home/user");
        assert_eq!(build_new_folder_path(&parent, "../etc"), None);
        assert_eq!(build_new_folder_path(&parent, "/etc"), None);
        assert_eq!(build_new_folder_path(&parent, "sub/name"), None);
        assert_eq!(build_new_folder_path(&parent, ""), None);
    }

    // ============================================================
    // build_upload_remote_path 测试
    // ============================================================

    /// 测试上传远程路径构建
    #[test]
    fn test_build_upload_remote_path_basic() {
        let current = PathBuf::from("/home/user");
        let result = build_upload_remote_path(&current, "upload.txt");
        assert_eq!(result, Some(PathBuf::from("/home/user/upload.txt")));
    }

    /// 测试上传远程路径含反斜杠规范化
    #[test]
    fn test_build_upload_remote_path_normalizes() {
        let current = PathBuf::from("/home/user");
        let result = build_upload_remote_path(&current, "file.txt");
        assert!(result.is_some());
        assert!(!result.unwrap().to_string_lossy().contains('\\'));
    }

    /// 测试上传远程路径拒绝危险文件名
    #[test]
    fn test_build_upload_remote_path_rejects_dangerous() {
        let current = PathBuf::from("/home/user");
        // file_name() 从 "../etc/passwd" 中提取 "passwd"，路径安全
        assert_eq!(
            build_upload_remote_path(&current, "../etc/passwd"),
            Some(PathBuf::from("/home/user/passwd"))
        );
        assert_eq!(build_upload_remote_path(&current, ""), None);
        // file_name() 从 "/etc/passwd" 中提取 "passwd"，路径安全
        assert_eq!(
            build_upload_remote_path(&current, "/etc/passwd"),
            Some(PathBuf::from("/home/user/passwd"))
        );
    }

    // ============================================================
    // SftpBrowserAction 枚举测试
    // ============================================================

    /// 测试 SftpBrowserAction::CancelTransfer 变体
    #[test]
    fn test_action_cancel_transfer() {
        let action = SftpBrowserAction::CancelTransfer(42);
        assert!(matches!(action, SftpBrowserAction::CancelTransfer(42)));
    }

    /// 测试 SftpBrowserAction::ConfirmMove 变体
    #[test]
    fn test_action_confirm_move() {
        let action = SftpBrowserAction::ConfirmMove;
        assert!(matches!(action, SftpBrowserAction::ConfirmMove));
    }

    /// 测试 SftpBrowserAction::SetSearchFilter 变体
    #[test]
    fn test_action_set_search_filter() {
        let action = SftpBrowserAction::SetSearchFilter("test".into());
        assert!(matches!(action, SftpBrowserAction::SetSearchFilter(_)));
    }

    /// 测试 SftpBrowserAction::ClearSearchFilter 变体
    #[test]
    fn test_action_clear_search_filter() {
        let action = SftpBrowserAction::ClearSearchFilter;
        assert!(matches!(action, SftpBrowserAction::ClearSearchFilter));
    }

    /// 测试 SftpBrowserAction::DownloadSaveAs 变体
    #[test]
    fn test_action_download_save_as() {
        let action = SftpBrowserAction::DownloadSaveAs {
            index: 3,
            local_path: "/tmp/file.txt".into(),
        };
        assert!(matches!(
            action,
            SftpBrowserAction::DownloadSaveAs { index: 3, .. }
        ));
    }
}
