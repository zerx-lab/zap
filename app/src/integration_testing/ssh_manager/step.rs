//! SSH 管理器集成测试步骤辅助函数。
//!
//! 所有步骤使用 `TestStep::new` 创建，不附加终端相关的默认断言，
//! 因为 SSH 管理器测试不依赖终端视图。

use std::sync::{Arc, Mutex};

use warp_ssh_manager::{SshRepository, SshServerInfo};
use warpui::integration::TestStep;
use warpui::windowing::WindowManager;
use warpui::SingletonEntity;
use warpui::TypedActionView;

use crate::ssh_manager::server_view::SshServerAction;
use crate::workspace::{Workspace, WorkspaceAction};

use super::assertions::ssh_server_view;

/// 打开 SSH 管理器左侧面板。
pub fn open_ssh_manager_panel() -> TestStep {
    TestStep::new("Open SSH manager panel").with_action(move |app, _, _| {
        let window_id = app.read(|ctx| {
            WindowManager::as_ref(ctx)
                .active_window()
                .expect("no active window")
        });
        let workspace_view_id = app
            .views_of_type::<Workspace>(window_id)
            .and_then(|views| views.first().map(|view| view.id()))
            .expect("no workspace view");
        log::info!("dispatching ToggleSshManager to workspace view {}", workspace_view_id);
        app.dispatch_typed_action(
            window_id,
            &[workspace_view_id],
            &WorkspaceAction::ToggleSshManager,
        );
    })
}

/// 通过 DB 创建测试文件夹，返回文件夹节点 ID。
pub fn create_folder_via_db(name: &str) -> String {
    let name = name.to_string();
    warp_ssh_manager::with_conn(move |c| {
        let node = SshRepository::create_folder(c, None, &name)
            .unwrap_or_else(|e| panic!("create folder failed: {e:?}"));
        Ok(node.id)
    })
    .expect("create folder via db")
}

/// 通过 DB 在指定文件夹下创建测试服务器，返回节点 ID。
pub fn create_server_via_db(name: &str, parent_id: Option<&str>) -> String {
    let name = name.to_string();
    let parent = parent_id.map(String::from);
    warp_ssh_manager::with_conn(move |c| {
        let info = SshServerInfo {
            node_id: String::new(),
            host: format!("{name}.example.com"),
            port: 22,
            username: "root".into(),
            auth_type: warp_ssh_manager::AuthType::Password,
            key_path: None,
            startup_command: None,
            notes: None,
            last_connected_at: None,
        };
        let node = SshRepository::create_server(c, parent.as_deref(), &name, &info)
            .unwrap_or_else(|e| panic!("create server failed: {e:?}"));
        Ok(node.id)
    })
    .expect("create server via db")
}

/// 通过 workspace 打开指定服务器的编辑器视图。
pub fn open_server_editor(node_id: String) -> TestStep {
    TestStep::new("Open SSH server editor").with_action(move |app, _, _| {
        let window_id = app.read(|ctx| {
            WindowManager::as_ref(ctx)
                .active_window()
                .expect("no active window")
        });
        let workspace = app
            .views_of_type::<Workspace>(window_id)
            .and_then(|views| views.first().cloned())
            .expect("no workspace view");
        workspace.update(app, |ws, ctx| {
            ws.open_ssh_server(node_id.clone(), ctx);
        });
    })
}

/// 在分组下拉选择器中选择指定分组。
/// 接收 `Arc<Mutex<Option<String>>>` 以便运行时读取文件夹 ID，
/// 通过 ID 查找对应的 index，然后 dispatch SelectGroup。
pub fn select_group_by_id(folder_id: Arc<Mutex<Option<String>>>) -> TestStep {
    TestStep::new("Select group by folder id").with_action(move |app, _, _| {
        let window_id = app.read(|ctx| {
            WindowManager::as_ref(ctx)
                .active_window()
                .expect("no active window")
        });
        let view = ssh_server_view(app, window_id);
        let gid = folder_id.lock().unwrap().clone();
        view.update(app, |v, ctx| {
            let index = gid.as_ref().and_then(|gid| {
                v.folders().iter().position(|(id, _)| id == gid)
            });
            v.handle_action(&SshServerAction::SelectGroup(index), ctx);
        });
    })
}

/// 保存服务器编辑器内容。
pub fn save_server() -> TestStep {
    TestStep::new("Save server").with_action(move |app, _, _| {
        let window_id = app.read(|ctx| {
            WindowManager::as_ref(ctx)
                .active_window()
                .expect("no active window")
        });
        let view = ssh_server_view(app, window_id);
        view.update(app, |v, ctx| {
            v.handle_action(&SshServerAction::Save, ctx);
        });
    })
}
