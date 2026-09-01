# Workspace Agent Activity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. REQUIRED SKILLS: @rust-unit-tests @warp-ui-guidelines

**Goal:** 当 repository workspace 的页签里有 agent 处于 InProgress 或 Blocked 时，在对应 workspace 行左侧用品牌圆头像替换绿点；InProgress 呼吸环，Blocked 静态黄环。

**Architecture:** 聚合逻辑留在 `Workspace`，纯类型和决胜规则抽到 `project_organization/workspace_agent_activity.rs`，展示留在 `ProjectTreePanel`。树只消费 `HashMap<RepositoryWorkspaceId, WorkspaceAgentActivity>`，不遍历 terminal / conversation。绿点继续用现有 `running_workspace_ids`，渲染时与头像互斥。

**Tech Stack:** Rust, WarpUI `Container`/`Border`/`repaint_after`, 现有 `IconWithStatus` / `render_cli_agent_logo`, `CLIAgentSessionsModel`, `BlocklistAIHistoryModel`, `RepositoryWorkspaceTabSets`。

**Spec:** `docs/superpowers/specs/2026-08-26-workspace-agent-activity-design.md`

## Global Constraints

- 注释用简体中文；日志、断言消息、commit message 用英文。
- `format!` 使用内联参数 `"{x}"`。
- `match` 禁止无必要的 `_` 通配。
- 未使用参数直接删除，不加 `_` 前缀。
- 不改 tab indicator、vertical tabs、pane header。
- 不做叠头像、+N、按底层模型显示品牌、Error 残留头像、点击跳页签。
- 多 agent 决胜：同一 workspace 内按页签从左到右扫描，后出现的 InProgress/Blocked 覆盖先前的。同一 terminal 上若 CLI 与 Oz 同时命中，先收 Oz 再收 CLI，因此 CLI 胜出。
- `panes_of` 遍历 HashMap，分屏多 agent 的次序不稳定；首期接受，不为此引入时间戳字段。
- 验证以相关单测和 `cargo check` 为准。
- 每完成一个 Task 提交一次，只暂存该任务文件。

## File Structure

- Create: `app/src/project_organization/workspace_agent_activity.rs`
- Create: `app/src/project_organization/workspace_agent_activity_tests.rs`
- Create: `app/src/ui_components/breathing_ring.rs`
- Create: `app/src/ui_components/breathing_ring_tests.rs`
- Create: `app/assets/bundled/svg/grok.svg`
- Modify: `app/src/project_organization/mod.rs`
- Modify: `app/src/workspace/repository_workspace_tabs.rs`
- Modify: `app/src/workspace/repository_workspace_tabs_tests.rs`
- Modify: `crates/warp_core/src/ui/icons.rs`
- Modify: `app/src/terminal/cli_agent.rs`
- Modify: `app/src/terminal/cli_agent_tests.rs`
- Modify: `app/src/ui_components/mod.rs`
- Modify: `app/src/project_organization/view/project_tree.rs`
- Modify: `app/src/project_organization/view/project_tree_tests.rs`
- Modify: `app/src/workspace/view/left_panel.rs`
- Modify: `app/src/workspace/view.rs`
- Modify: `specs/repository-workspaces/TECH.md`

---

### Task 1: Agent 活动类型与槽位互斥

**Files:**
- Create: `app/src/project_organization/workspace_agent_activity.rs`
- Create: `app/src/project_organization/workspace_agent_activity_tests.rs`
- Modify: `app/src/project_organization/mod.rs`

- [ ] **Step 1: Write the failing tests**

Create `app/src/project_organization/workspace_agent_activity_tests.rs`:

```rust
use crate::terminal::CLIAgent;

use super::workspace_agent_activity::{
    last_agent_activity, workspace_activity_slot, WorkspaceActivitySlot, WorkspaceAgentActivity,
    WorkspaceAgentIdentity, WorkspaceAgentPhase,
};

fn grok_running() -> WorkspaceAgentActivity {
    WorkspaceAgentActivity {
        identity: WorkspaceAgentIdentity::Cli(CLIAgent::Grok),
        phase: WorkspaceAgentPhase::InProgress,
    }
}

fn claude_blocked() -> WorkspaceAgentActivity {
    WorkspaceAgentActivity {
        identity: WorkspaceAgentIdentity::Cli(CLIAgent::Claude),
        phase: WorkspaceAgentPhase::Blocked,
    }
}

#[test]
fn last_agent_activity_returns_later_candidate() {
    assert_eq!(
        last_agent_activity([grok_running(), claude_blocked()]),
        Some(claude_blocked())
    );
}

#[test]
fn last_agent_activity_returns_none_when_empty() {
    assert_eq!(last_agent_activity([]), None);
}

#[test]
fn activity_slot_prefers_agent_over_running_dot() {
    assert_eq!(
        workspace_activity_slot(Some(grok_running()), true),
        WorkspaceActivitySlot::Agent(grok_running())
    );
}

#[test]
fn activity_slot_falls_back_to_running_dot() {
    assert_eq!(
        workspace_activity_slot(None, true),
        WorkspaceActivitySlot::RunningDot
    );
}

#[test]
fn activity_slot_is_empty_when_idle() {
    assert_eq!(workspace_activity_slot(None, false), WorkspaceActivitySlot::Empty);
}
```

- [ ] **Step 2: Run tests and verify they fail**

Run:

```bash
cargo test -p warp --lib project_organization::workspace_agent_activity_tests
```

Expected: fail because the module does not exist.

- [ ] **Step 3: Implement the types**

Add to `app/src/project_organization/mod.rs`:

```rust
pub(crate) mod workspace_agent_activity;
```

Create `app/src/project_organization/workspace_agent_activity.rs`:

```rust
use crate::terminal::CLIAgent;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorkspaceAgentIdentity {
    Cli(CLIAgent),
    Oz { ambient: bool },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorkspaceAgentPhase {
    InProgress,
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct WorkspaceAgentActivity {
    pub identity: WorkspaceAgentIdentity,
    pub phase: WorkspaceAgentPhase,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorkspaceActivitySlot {
    Empty,
    RunningDot,
    Agent(WorkspaceAgentActivity),
}

pub(crate) fn last_agent_activity(
    activities: impl IntoIterator<Item = WorkspaceAgentActivity>,
) -> Option<WorkspaceAgentActivity> {
    activities.into_iter().last()
}

pub(crate) fn workspace_activity_slot(
    agent: Option<WorkspaceAgentActivity>,
    has_running_terminal: bool,
) -> WorkspaceActivitySlot {
    match agent {
        Some(activity) => WorkspaceActivitySlot::Agent(activity),
        None if has_running_terminal => WorkspaceActivitySlot::RunningDot,
        None => WorkspaceActivitySlot::Empty,
    }
}

impl WorkspaceAgentActivity {
    pub(crate) fn should_breathe(self) -> bool {
        matches!(self.phase, WorkspaceAgentPhase::InProgress)
    }
}
```

At the end of `workspace_agent_activity.rs`:

```rust
#[cfg(test)]
#[path = "workspace_agent_activity_tests.rs"]
mod tests;
```

- [ ] **Step 4: Run tests and verify they pass**

```bash
cargo test -p warp --lib project_organization::workspace_agent_activity_tests
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add app/src/project_organization/mod.rs \
  app/src/project_organization/workspace_agent_activity.rs \
  app/src/project_organization/workspace_agent_activity_tests.rs
git commit -m "feat: add workspace agent activity types"
```

---

### Task 2: 按页签顺序取每个 workspace 的最后一个命中

**Files:**
- Modify: `app/src/workspace/repository_workspace_tabs.rs`
- Modify: `app/src/workspace/repository_workspace_tabs_tests.rs`

- [ ] **Step 1: Write the failing tests**

Append to `app/src/workspace/repository_workspace_tabs_tests.rs`:

```rust
#[test]
fn map_last_matching_keeps_later_tab_in_the_same_workspace() {
    let workspace_a = RepositoryWorkspaceId(uuid::Uuid::from_u128(1));
    let workspace_b = RepositoryWorkspaceId(uuid::Uuid::from_u128(2));
    let mut sets = RepositoryWorkspaceTabSets::new(Some(workspace_a));
    sets.insert_inactive(
        Some(workspace_b),
        RepositoryWorkspaceTabState::new(vec![20_u64, 21, 22], 0),
    );

    let active_tabs = vec![10_u64, 11, 12];
    let matches = sets.map_last_matching(&active_tabs, |tab| match tab {
        11 | 12 => Some(*tab),
        20 | 22 => Some(*tab),
        _ => None,
    });

    assert_eq!(matches.get(&workspace_a), Some(&12));
    assert_eq!(matches.get(&workspace_b), Some(&22));
}

#[test]
fn map_last_matching_ignores_unclassified_tabs() {
    let workspace_a = RepositoryWorkspaceId(uuid::Uuid::from_u128(1));
    let mut sets = RepositoryWorkspaceTabSets::new(Some(workspace_a));
    sets.insert_inactive(None, RepositoryWorkspaceTabState::new(vec![20_u64], 0));

    let matches = sets.map_last_matching(&[10_u64], |tab| (*tab == 20).then_some(*tab));
    assert!(matches.is_empty());
}
```

- [ ] **Step 2: Run tests and verify they fail**

```bash
cargo test -p warp --lib workspace::repository_workspace_tabs_tests::map_last_matching
```

Expected: fail because `map_last_matching` does not exist.

- [ ] **Step 3: Implement `map_last_matching`**

Add inside `impl<T> RepositoryWorkspaceTabSets<T>` after `workspace_ids_matching`:

```rust
    pub(crate) fn map_last_matching<U>(
        &self,
        active_tabs: &[T],
        mut map_tab: impl FnMut(&T) -> Option<U>,
    ) -> HashMap<RepositoryWorkspaceId, U> {
        let mut matches = HashMap::new();

        if let Some(workspace_id) = self.active_workspace_id {
            let mut last = None;
            for tab in active_tabs {
                if let Some(value) = map_tab(tab) {
                    last = Some(value);
                }
            }
            if let Some(value) = last {
                matches.insert(workspace_id, value);
            }
        }

        for (workspace_id, state) in &self.inactive {
            let Some(workspace_id) = workspace_id else {
                continue;
            };
            let mut last = None;
            for tab in &state.tabs {
                if let Some(value) = map_tab(tab) {
                    last = Some(value);
                }
            }
            if let Some(value) = last {
                matches.insert(*workspace_id, value);
            }
        }

        matches
    }
```

- [ ] **Step 4: Run tests and verify they pass**

```bash
cargo test -p warp --lib workspace::repository_workspace_tabs_tests::map_last_matching
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add app/src/workspace/repository_workspace_tabs.rs \
  app/src/workspace/repository_workspace_tabs_tests.rs
git commit -m "feat: keep last matching tab per repository workspace"
```

---

### Task 3: 从 CLI / Oz 会话源收集候选活动

**Files:**
- Modify: `app/src/project_organization/workspace_agent_activity.rs`
- Modify: `app/src/project_organization/workspace_agent_activity_tests.rs`

把 AppContext 之外能测的映射做成纯函数，Workspace 接线只负责读 model。

- [ ] **Step 1: Write the failing tests**

Append to `workspace_agent_activity_tests.rs`:

```rust
use crate::ai::agent::conversation::ConversationStatus;
use crate::terminal::cli_agent_sessions::CLIAgentSessionStatus;

use super::workspace_agent_activity::{
    activities_from_terminal_sources, OzConversationSource,
};

#[test]
fn cli_in_progress_is_collected() {
    let activities = activities_from_terminal_sources(
        Some((CLIAgent::Grok, CLIAgentSessionStatus::InProgress)),
        None,
        false,
    );
    assert_eq!(activities, vec![grok_running()]);
}

#[test]
fn cli_success_is_ignored() {
    let activities = activities_from_terminal_sources(
        Some((CLIAgent::Grok, CLIAgentSessionStatus::Success)),
        None,
        false,
    );
    assert!(activities.is_empty());
}

#[test]
fn oz_blocked_is_collected() {
    let activities = activities_from_terminal_sources(
        None,
        Some(OzConversationSource {
            status: ConversationStatus::Blocked {
                blocked_action: "ask".to_string(),
            },
            is_empty: false,
            is_entirely_passive: false,
        }),
        false,
    );
    assert_eq!(
        activities,
        vec![WorkspaceAgentActivity {
            identity: WorkspaceAgentIdentity::Oz { ambient: false },
            phase: WorkspaceAgentPhase::Blocked,
        }]
    );
}

#[test]
fn empty_or_passive_oz_is_ignored() {
    let activities = activities_from_terminal_sources(
        None,
        Some(OzConversationSource {
            status: ConversationStatus::InProgress,
            is_empty: true,
            is_entirely_passive: false,
        }),
        false,
    );
    assert!(activities.is_empty());
}

#[test]
fn cli_wins_over_oz_on_the_same_terminal() {
    let activities = activities_from_terminal_sources(
        Some((CLIAgent::Grok, CLIAgentSessionStatus::InProgress)),
        Some(OzConversationSource {
            status: ConversationStatus::Blocked {
                blocked_action: "ask".to_string(),
            },
            is_empty: false,
            is_entirely_passive: false,
        }),
        false,
    );
    assert_eq!(last_agent_activity(activities), Some(grok_running()));
}

#[test]
fn ambient_in_progress_uses_oz_cloud_identity() {
    let activities = activities_from_terminal_sources(None, None, true);
    assert_eq!(
        activities,
        vec![WorkspaceAgentActivity {
            identity: WorkspaceAgentIdentity::Oz { ambient: true },
            phase: WorkspaceAgentPhase::InProgress,
        }]
    );
}
```

- [ ] **Step 2: Run tests and verify they fail**

```bash
cargo test -p warp --lib project_organization::workspace_agent_activity_tests
```

Expected: fail because `activities_from_terminal_sources` does not exist.

- [ ] **Step 3: Implement the mapping**

Add to `workspace_agent_activity.rs`:

```rust
use crate::ai::agent::conversation::ConversationStatus;
use crate::terminal::cli_agent_sessions::CLIAgentSessionStatus;

pub(crate) struct OzConversationSource {
    pub status: ConversationStatus,
    pub is_empty: bool,
    pub is_entirely_passive: bool,
}

pub(crate) fn activities_from_terminal_sources(
    cli: Option<(CLIAgent, CLIAgentSessionStatus)>,
    oz: Option<OzConversationSource>,
    ambient_in_progress: bool,
) -> Vec<WorkspaceAgentActivity> {
    let mut activities = Vec::new();

    if ambient_in_progress {
        activities.push(WorkspaceAgentActivity {
            identity: WorkspaceAgentIdentity::Oz { ambient: true },
            phase: WorkspaceAgentPhase::InProgress,
        });
    }

    if let Some(oz) = oz {
        if !oz.is_empty && !oz.is_entirely_passive {
            if let Some(phase) = phase_from_conversation_status(&oz.status) {
                activities.push(WorkspaceAgentActivity {
                    identity: WorkspaceAgentIdentity::Oz { ambient: false },
                    phase,
                });
            }
        }
    }

    if let Some((agent, status)) = cli {
        if let Some(phase) = phase_from_cli_status(&status) {
            activities.push(WorkspaceAgentActivity {
                identity: WorkspaceAgentIdentity::Cli(agent),
                phase,
            });
        }
    }

    activities
}

fn phase_from_cli_status(status: &CLIAgentSessionStatus) -> Option<WorkspaceAgentPhase> {
    match status {
        CLIAgentSessionStatus::InProgress => Some(WorkspaceAgentPhase::InProgress),
        CLIAgentSessionStatus::Blocked { .. } => Some(WorkspaceAgentPhase::Blocked),
        CLIAgentSessionStatus::Success => None,
    }
}

fn phase_from_conversation_status(status: &ConversationStatus) -> Option<WorkspaceAgentPhase> {
    match status {
        ConversationStatus::InProgress => Some(WorkspaceAgentPhase::InProgress),
        ConversationStatus::Blocked { .. } => Some(WorkspaceAgentPhase::Blocked),
        ConversationStatus::Success
        | ConversationStatus::Error
        | ConversationStatus::Cancelled => None,
    }
}
```

候选顺序必须是 ambient → Oz → CLI，这样 `last_agent_activity` 在同一 terminal 上让 CLI 覆盖 Oz。

- [ ] **Step 4: Run tests and verify they pass**

```bash
cargo test -p warp --lib project_organization::workspace_agent_activity_tests
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add app/src/project_organization/workspace_agent_activity.rs \
  app/src/project_organization/workspace_agent_activity_tests.rs
git commit -m "feat: map CLI and Oz session sources to workspace agent activity"
```

---

### Task 4: 补 Grok logo

**Files:**
- Create: `app/assets/bundled/svg/grok.svg`
- Modify: `crates/warp_core/src/ui/icons.rs`
- Modify: `app/src/terminal/cli_agent.rs`
- Modify: `app/src/terminal/cli_agent_tests.rs`

- [ ] **Step 1: Write the failing test**

Append to `app/src/terminal/cli_agent_tests.rs`（已有 `use super::{..., CLIAgent}`）：

```rust
#[test]
fn grok_has_brand_icon() {
    assert_eq!(
        CLIAgent::Grok.icon(),
        Some(crate::ui_components::icons::Icon::GrokLogo)
    );
}
```

- [ ] **Step 2: Run the test and verify it fails**

```bash
cargo test -p warp --lib terminal::cli_agent_tests::grok_has_brand_icon
```

Expected: fail because `Icon::GrokLogo` does not exist, and `CLIAgent::Grok.icon()` is `None`.

- [ ] **Step 3: Add SVG, enum variant, and icon mapping**

Create `app/assets/bundled/svg/grok.svg`，遵循现有 logo 约定：24×24、单 path、`fill="#FF0000"` 以便 WarpUI tint。使用简洁四角星（Grok / xAI sparkle）：

```svg
<svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<path d="M12 1.5L13.7 9.3L21.5 11L13.7 12.7L12 20.5L10.3 12.7L2.5 11L10.3 9.3L12 1.5Z" fill="#FF0000"/>
</svg>
```

In `crates/warp_core/src/ui/icons.rs`:

1. 在 `OmpLogo` 后增加 `GrokLogo,`
2. 在 `as_str`/`path` match 中 `Icon::OmpLogo` 旁增加：

```rust
Icon::GrokLogo => "bundled/svg/grok.svg",
```

该 match 必须穷尽，漏加会编译失败。

In `app/src/terminal/cli_agent.rs` `icon()`:

```rust
CLIAgent::Grok => Some(Icon::GrokLogo),
```

`brand_icon_color` 对 Grok 保持默认白色（品牌底是近黑 `GROK_COLOR`）。

- [ ] **Step 4: Run the test and verify it passes**

```bash
cargo test -p warp --lib terminal::cli_agent_tests::grok_has_brand_icon
cargo check -p warp
```

Expected: PASS / check 绿。

- [ ] **Step 5: Commit**

```bash
git add app/assets/bundled/svg/grok.svg \
  crates/warp_core/src/ui/icons.rs \
  app/src/terminal/cli_agent.rs \
  app/src/terminal/cli_agent_tests.rs
git commit -m "feat: add Grok CLI agent brand icon"
```

---

### Task 5: 呼吸环透明度和 Element

**Files:**
- Create: `app/src/ui_components/breathing_ring.rs`
- Create: `app/src/ui_components/breathing_ring_tests.rs`
- Modify: `app/src/ui_components/mod.rs`

透明度计算必须可单测。动画 handle 与 `SpinnerStateHandle` 同模式：跨 render 持久化 `Instant`。

- [ ] **Step 1: Write the failing tests**

Create `app/src/ui_components/breathing_ring_tests.rs`:

```rust
use std::time::Duration;

use super::breathing_ring::{breathing_opacity, BREATHING_PERIOD};

#[test]
fn breathing_opacity_starts_near_low_end() {
    assert_eq!(breathing_opacity(Duration::ZERO, BREATHING_PERIOD), 102);
}

#[test]
fn breathing_opacity_peaks_at_half_period() {
    assert_eq!(
        breathing_opacity(BREATHING_PERIOD / 2, BREATHING_PERIOD),
        255
    );
}

#[test]
fn breathing_opacity_is_periodic() {
    assert_eq!(
        breathing_opacity(Duration::ZERO, BREATHING_PERIOD),
        breathing_opacity(BREATHING_PERIOD, BREATHING_PERIOD)
    );
}
```

`102` = `round(0.4 * 255)`。若实现用 `u8` 截断而不是 round，把断言改成与实现一致的精确值，但公式必须是 `0.4 + 0.6 * (sin 映射到 0..1)`。

- [ ] **Step 2: Run tests and verify they fail**

```bash
cargo test -p warp --lib ui_components::breathing_ring_tests
```

Expected: fail because the module does not exist.

- [ ] **Step 3: Implement opacity helper and wrapping element**

`app/src/ui_components/mod.rs` 增加 `pub(crate) mod breathing_ring;`。

Create `app/src/ui_components/breathing_ring.rs`：

```rust
use std::sync::{Arc, Mutex};
use std::time::Duration;

use instant::Instant;
use pathfinder_color::ColorU;
use pathfinder_geometry::vector::Vector2F;
use warp_core::ui::color::coloru_with_opacity;
use warpui::elements::{
    Border, ConstrainedBox, Container, CornerRadius, Element, Point, Radius,
};
use warpui::{
    AfterLayoutContext, AppContext, EventContext, LayoutContext, PaintContext, SizeConstraint,
};

pub(crate) const BREATHING_PERIOD: Duration = Duration::from_millis(1600);
const REPAINT_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Clone)]
pub(crate) struct BreathingStateHandle(Arc<Mutex<Instant>>);

impl Default for BreathingStateHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl BreathingStateHandle {
    pub(crate) fn new() -> Self {
        Self(Arc::new(Mutex::new(Instant::now())))
    }

    fn elapsed(&self) -> Duration {
        self.0
            .lock()
            .expect("breathing state poisoned")
            .elapsed()
    }
}

pub(crate) fn breathing_opacity(elapsed: Duration, period: Duration) -> u8 {
    let period_secs = period.as_secs_f32().max(f32::EPSILON);
    let turns = elapsed.as_secs_f32() / period_secs;
    let wave = (turns * std::f32::consts::TAU).sin().mul_add(0.5, 0.5);
    ((0.4 + 0.6 * wave) * 255.0).round() as u8
}

pub(crate) struct BreathingRing {
    child: Box<dyn Element>,
    color: ColorU,
    animate: bool,
    state: BreathingStateHandle,
    border_width: f32,
    inner: Option<Container>,
    size: Option<Vector2F>,
    origin: Option<Point>,
}

impl BreathingRing {
    pub(crate) fn new(
        child: Box<dyn Element>,
        color: ColorU,
        animate: bool,
        state: BreathingStateHandle,
    ) -> Self {
        Self {
            child,
            color,
            animate,
            state,
            border_width: 1.5,
            inner: None,
            size: None,
            origin: None,
        }
    }
}

impl Element for BreathingRing {
    fn layout(
        &mut self,
        constraint: SizeConstraint,
        ctx: &mut LayoutContext,
        app: &AppContext,
    ) -> Vector2F {
        let opacity = if self.animate {
            breathing_opacity(self.state.elapsed(), BREATHING_PERIOD)
        } else {
            255
        };
        let mut inner = Container::new(
            ConstrainedBox::new(std::mem::replace(
                &mut self.child,
                Box::new(warpui::elements::Empty::new().finish()),
            ))
            .finish(),
        )
        .with_border(
            Border::all(self.border_width)
                .with_border_fill(coloru_with_opacity(self.color, opacity)),
        )
        .with_corner_radius(CornerRadius::with_all(Radius::Percentage(50.)));
        let size = inner.layout(constraint, ctx, app);
        // 把 child 从 inner 拿不回来时，改为 BreathingRing 持有 child，layout 里 clone 不了。
        // 正确写法：child 只移入 inner 一次。见下面修正。
        self.size = Some(size);
        size
    }
    // ...
}
```

**不要**用上面那段会把 child 换成 Empty 的错误 layout。正确结构：

`BreathingRing` 只包一层已经 layout 好的 `Box<dyn Element>`，在 `layout` 里对 **已有 child** 做 `Container::new(child 的占位)` 不行，因为 `Box<dyn Element>` 不能每帧重建。

采用与 `BrailleSpinner` 相同的「每帧重建 inner」策略，但 child 是调用方已经 build 好的 element，只能 layout 一次。因此更简单：

**推荐实现：** `BreathingRing` 不托管 child。调用方：

```rust
Container::new(avatar)
    .with_border(Border::all(1.5).with_border_fill(coloru_with_opacity(color, opacity)))
    .with_corner_radius(CornerRadius::with_all(Radius::Percentage(50.)))
```

另做一个只负责 `repaint_after` 的 ticker element，叠在 Stack 里：

```rust
pub(crate) struct BreathingTicker {
    state: BreathingStateHandle,
    origin: Option<Point>,
    size: Option<Vector2F>,
}

impl Element for BreathingTicker {
    fn layout(
        &mut self,
        _constraint: SizeConstraint,
        _ctx: &mut LayoutContext,
        _app: &AppContext,
    ) -> Vector2F {
        let size = Vector2F::new(0., 0.);
        self.size = Some(size);
        size
    }

    fn after_layout(&mut self, _ctx: &mut AfterLayoutContext, _app: &AppContext) {}

    fn paint(&mut self, origin: Vector2F, ctx: &mut PaintContext, _app: &AppContext) {
        self.origin = Some(Point::from_vec2f(origin, ctx.scene.z_index()));
        ctx.repaint_after(REPAINT_INTERVAL);
    }

    fn size(&self) -> Option<Vector2F> {
        self.size
    }

    fn request_capture(&self, _pos: Vector2F) -> Option<warpui::elements::CaptureReason> {
        None
    }

    fn handle_event(&mut self, _event: &mut warpui::event::Event, _ctx: &mut EventContext) {}
}
```

对照 `BrailleSpinner` 把 `Element` 必实现方法补全（该 trait 近期若有默认实现则不要多余 override）。

`render` 路径：

```rust
let opacity = if activity.should_breathe() {
    breathing_opacity(handle.elapsed(), BREATHING_PERIOD)
} else {
    255
};
```

需要把 `BreathingStateHandle::elapsed` 做成 `pub(crate)`。

看 `BrailleSpinner` 的 `impl Element`，把 BreathingTicker 写成同样签名，避免漏方法。

`mod.rs` 末尾：

```rust
#[cfg(test)]
#[path = "breathing_ring_tests.rs"]
mod breathing_ring_tests;
```

若 `ui_components/mod.rs` 已有 `#[cfg(test)]` 模块，把 tests path 放到 `breathing_ring.rs` 底部，与 spinner 一致。

- [ ] **Step 4: Run tests and verify they pass**

```bash
cargo test -p warp --lib ui_components::breathing_ring_tests
```

若 Step 1 的 102/255 与 round 结果差 1，按实际公式修正断言，不要改成硬编码魔法值而不写公式。

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add app/src/ui_components/mod.rs \
  app/src/ui_components/breathing_ring.rs \
  app/src/ui_components/breathing_ring_tests.rs
git commit -m "feat: add breathing ring opacity animation"
```

---

### Task 6: Project tree 消费活动并渲染头像槽

**Files:**
- Modify: `app/src/project_organization/view/project_tree.rs`
- Modify: `app/src/project_organization/view/project_tree_tests.rs`

槽位宽度固定为 `WORKSPACE_ACTIVITY_SLOT_SIZE`（建议 `16.`），绿点在槽内居中，头像填满该槽。有 agent 时不画绿点。

- [ ] **Step 1: Write failing tests**

Update `workspace_visual_state_keeps_selection_and_running_separate` 仍覆盖「无 agent + running → 绿点」。新增：

```rust
use crate::project_organization::workspace_agent_activity::{
    workspace_activity_slot, WorkspaceActivitySlot, WorkspaceAgentActivity, WorkspaceAgentIdentity,
    WorkspaceAgentPhase,
};
use crate::terminal::CLIAgent;

#[test]
fn workspace_visual_state_hides_running_dot_when_agent_is_present() {
    let activity = WorkspaceAgentActivity {
        identity: WorkspaceAgentIdentity::Cli(CLIAgent::Grok),
        phase: WorkspaceAgentPhase::InProgress,
    };
    let visual_state = WorkspaceVisualState::new(true, true, Some(activity));
    assert!(visual_state.should_render_selection_accent());
    assert!(!visual_state.should_render_running_indicator());
    assert_eq!(
        visual_state.activity_slot(),
        WorkspaceActivitySlot::Agent(activity)
    );
    assert!(visual_state.should_breathe_agent_ring());
}

#[test]
fn workspace_visual_state_blocked_agent_does_not_breathe() {
    let activity = WorkspaceAgentActivity {
        identity: WorkspaceAgentIdentity::Oz { ambient: false },
        phase: WorkspaceAgentPhase::Blocked,
    };
    let visual_state = WorkspaceVisualState::new(false, false, Some(activity));
    assert!(!visual_state.should_breathe_agent_ring());
    assert_eq!(
        visual_state.activity_slot(),
        WorkspaceActivitySlot::Agent(activity)
    );
}
```

现有 `WorkspaceVisualState::new(selected, running)` 调用点必须全部改为三参数。先改测试里四处 `new`：idle / selected / running 传 `None`。

再加一个 render smoke，复制 `project_tree_renders_running_selected_workspace_activity_badge`，在 `set_running_workspaces` 后调用 `set_agent_activities`：

```rust
project_tree.set_agent_activities(
    HashMap::from([(
        workspace_id,
        WorkspaceAgentActivity {
            identity: WorkspaceAgentIdentity::Cli(CLIAgent::Grok),
            phase: WorkspaceAgentPhase::InProgress,
        },
    )]),
    ctx,
);
```

名字：`project_tree_renders_running_grok_agent_avatar`。断言至少 `build_scene` 不 panic。不要对像素做截图断言。

- [ ] **Step 2: Run tests and verify they fail**

```bash
cargo test -p warp --lib project_organization::view::project_tree_tests::workspace_visual_state
```

Expected: fail because `WorkspaceVisualState::new` 还是两参数。

- [ ] **Step 3: Implement state and rendering**

`ProjectTreePanel` 增加：

```rust
agent_activities: HashMap<RepositoryWorkspaceId, WorkspaceAgentActivity>,
workspace_breathing_states: HashMap<RepositoryWorkspaceId, BreathingStateHandle>,
```

`new` / `set_running_workspaces` 旁增加：

```rust
pub fn set_agent_activities(
    &mut self,
    agent_activities: HashMap<RepositoryWorkspaceId, WorkspaceAgentActivity>,
    ctx: &mut ViewContext<Self>,
) {
    if self.agent_activities == agent_activities {
        return;
    }
    self.agent_activities = agent_activities;
    self.sync_breathing_states();
    ctx.notify();
}

fn sync_breathing_states(&mut self) {
    self.workspace_breathing_states
        .retain(|workspace_id, _| {
            self.agent_activities
                .get(workspace_id)
                .is_some_and(|activity| activity.should_breathe())
        });
    for (workspace_id, activity) in &self.agent_activities {
        if activity.should_breathe() {
            self.workspace_breathing_states
                .entry(*workspace_id)
                .or_default();
        }
    }
}
```

`refresh_tree` 在 retain running ids 之后：

```rust
self.agent_activities
    .retain(|workspace_id, _| workspace_ids.contains(workspace_id));
self.sync_breathing_states();
```

扩展 `WorkspaceVisualState`：

```rust
pub(crate) struct WorkspaceVisualState {
    is_selected: bool,
    has_running_terminal: bool,
    agent_activity: Option<WorkspaceAgentActivity>,
}

impl WorkspaceVisualState {
    pub(crate) fn new(
        is_selected: bool,
        has_running_terminal: bool,
        agent_activity: Option<WorkspaceAgentActivity>,
    ) -> Self {
        Self {
            is_selected,
            has_running_terminal,
            agent_activity,
        }
    }

    pub(crate) fn activity_slot(self) -> WorkspaceActivitySlot {
        workspace_activity_slot(self.agent_activity, self.has_running_terminal)
    }

    pub(crate) fn should_render_running_indicator(&self) -> bool {
        matches!(self.activity_slot(), WorkspaceActivitySlot::RunningDot)
    }

    pub(crate) fn should_breathe_agent_ring(&self) -> bool {
        self.agent_activity
            .is_some_and(WorkspaceAgentActivity::should_breathe)
    }
}
```

`should_render_running_indicator` 现在必须走 slot 互斥，不能只看 `has_running_terminal`。

`render_workspace_row`：

```rust
let visual_state = WorkspaceVisualState::new(
    selected,
    self.running_workspace_ids.contains(&workspace.workspace_id),
    self.agent_activities.get(&workspace.workspace_id).copied(),
);
let activity_slot = self.render_workspace_activity_slot(
    visual_state,
    workspace.workspace_id,
    appearance,
);
```

把原来的 `running_dot` child 换成 `activity_slot`。

`render_workspace_activity_slot`：

- `Empty`：`ConstrainedBox` 宽高 `WORKSPACE_ACTIVITY_SLOT_SIZE`，内部 `Empty`。
- `RunningDot`：现有绿点，外层同样约束到 `WORKSPACE_ACTIVITY_SLOT_SIZE`。
- `Agent`：用 `IconWithStatus` / `render_cli_agent_logo` 画 16px 圆头像，`status: None`（角标不是本设计）。外圈 `Border::all(1.5)`：
  - InProgress：品牌色 + `breathing_opacity`，并在 Stack 里放 `BreathingTicker`。
  - Blocked：`theme.ansi_fg_yellow()`，opacity 255，不放 ticker。
- CLI 品牌色：`agent.brand_color().unwrap_or(...)`；Unknown 用 `IconWithStatusVariant::Neutral { icon: WarpIcon::Terminal, ... }`。
- Oz：`IconWithStatusVariant::OzAgent { status: None, is_ambient }`，环颜色用 `theme.accent().into_solid_bias_right_color()`。

Sizing 建议：

```rust
const WORKSPACE_AGENT_ICON_SIZING: IconWithStatusSizing = IconWithStatusSizing {
    icon_size: 10.,
    padding: 3.,
    badge_icon_size: 8.,
    badge_padding: 1.,
    overall_size_override: Some(16.),
    badge_offset: (0., 0.),
};
```

`badge_*` 在 `status: None` 时不会画出来。

环颜色：

```rust
fn agent_ring_color(activity: WorkspaceAgentActivity, theme: &WarpTheme) -> ColorU {
    match activity.phase {
        WorkspaceAgentPhase::Blocked => theme.ansi_fg_yellow(),
        WorkspaceAgentPhase::InProgress => match activity.identity {
            WorkspaceAgentIdentity::Cli(agent) => agent
                .brand_color()
                .unwrap_or_else(|| theme.accent().into_solid_bias_right_color()),
            WorkspaceAgentIdentity::Oz { .. } => theme.accent().into_solid_bias_right_color(),
        },
    }
}
```

`unwrap_or_else` 闭包不要捕获不需要的东西。`accent()` 返回类型按现有 `into_solid_bias_right_color` 用法对齐 `render_workspace_tab_count`。

- [ ] **Step 4: Run tests and verify they pass**

```bash
cargo test -p warp --lib project_organization::view::project_tree_tests
```

Expected: 旧 visual-state 测试与新 avatar smoke 均 PASS。

- [ ] **Step 5: Commit**

```bash
git add app/src/project_organization/view/project_tree.rs \
  app/src/project_organization/view/project_tree_tests.rs
git commit -m "feat: render workspace agent avatar in activity slot"
```

---

### Task 7: Workspace / LeftPanel 接线

**Files:**
- Modify: `app/src/workspace/view/left_panel.rs`
- Modify: `app/src/workspace/view.rs`

- [ ] **Step 1: Write a focused extraction test if a tab-level helper stays pure**

`tab_agent_activity` 必须碰 `AppContext`，不要为它搭完整 TerminalView。Task 3 的纯函数已经覆盖映射。本任务只接线。

若 `Workspace` 里还能抽出：

```rust
fn agent_activity_for_terminal_view(
    terminal_view: &TerminalView,
    ctx: &AppContext,
) -> Option<WorkspaceAgentActivity>
```

它只是读 model 再调 `activities_from_terminal_sources` + `last_agent_activity`。可以不单测这一层，靠 Task 3 + 编译。

- [ ] **Step 2: Implement LeftPanel setter**

In `left_panel.rs`，`set_project_tree_running_workspaces` 旁：

```rust
pub fn set_project_tree_agent_activities(
    &mut self,
    agent_activities: HashMap<RepositoryWorkspaceId, crate::project_organization::workspace_agent_activity::WorkspaceAgentActivity>,
    ctx: &mut ViewContext<Self>,
) {
    self.project_tree_view.update(ctx, |tree, ctx| {
        tree.set_agent_activities(agent_activities, ctx);
    });
}
```

import 用 `WorkspaceAgentActivity` 短名。

- [ ] **Step 3: Implement Workspace aggregation**

In `workspace/view.rs` `sync_project_tree`：

```rust
let agent_activities = self.repository_workspace_agent_activities(ctx);
self.left_panel_view.update(ctx, |left_panel, ctx| {
    left_panel.set_project_tree_tab_counts(tab_counts, ctx);
    left_panel.set_project_tree_active_workspace(active_workspace_id, ctx);
    left_panel.set_project_tree_running_workspaces(running_workspace_ids, ctx);
    left_panel.set_project_tree_agent_activities(agent_activities, ctx);
});
```

新增：

```rust
fn repository_workspace_agent_activities(
    &self,
    ctx: &AppContext,
) -> HashMap<RepositoryWorkspaceId, WorkspaceAgentActivity> {
    self.repository_workspace_tabs
        .map_last_matching(&self.tabs, |tab| self.tab_agent_activity(tab, ctx))
}

fn tab_agent_activity(
    &self,
    tab: &TabData,
    ctx: &AppContext,
) -> Option<WorkspaceAgentActivity> {
    let pane_group = tab.pane_group.as_ref(ctx);
    let mut activities = Vec::new();
    for pane in pane_group.panes_of::<TerminalPane>() {
        let terminal_view = pane.terminal_view(ctx);
        let terminal_view_id = terminal_view.id();
        let terminal_view = terminal_view.as_ref(ctx);
        if terminal_view.is_read_only() {
            continue;
        }

        let cli = CLIAgentSessionsModel::as_ref(ctx)
            .session(terminal_view_id)
            .map(|session| (session.agent, session.status.clone()));
        let oz = BlocklistAIHistoryModel::as_ref(ctx)
            .active_conversation(terminal_view_id)
            .map(|conversation| OzConversationSource {
                status: conversation.status().clone(),
                is_empty: conversation.is_empty(),
                is_entirely_passive: conversation.is_entirely_passive(),
            });
        let ambient_in_progress = terminal_view
            .model
            .lock()
            .is_shared_ambient_agent_session();

        activities.extend(activities_from_terminal_sources(
            cli,
            oz,
            ambient_in_progress,
        ));
    }
    last_agent_activity(activities)
}
```

**TerminalModel 锁：** `tab_agent_activity` 里只锁一次、立刻读完 `is_shared_ambient_agent_session`，不要在持锁时再调可能加锁的函数。若 `is_read_only()` 内部也锁 model，先读 `is_read_only` 再锁 ambient，不要嵌套。

只读 / shared viewer：`is_read_only()` 为 true 则 skip，与现有 long-running 过滤对齐。

确认 `CLIAgentSessionsModel`、`BlocklistAIHistoryModel`、`TerminalPane`、`OzConversationSource` 已在 `view.rs` import。`status.clone()`：`CLIAgentSessionStatus` 已是 `Clone`。

`sync_project_tree` 已在 `TerminalViewStateChanged` 等路径调用；CLI status 变化若已触发同一 notify，不必新订阅。若接线后手动跑 grok 头像不更新，再在处理 `CLIAgentSessionsModelEvent::StatusChanged` 的现有路径确认会 `sync_project_tree`。没有的话，在 Workspace 对 `CLIAgentSessionsModel` 的现有 subscribe 里补一次 `sync_project_tree`，不要新开全局 cache。

- [ ] **Step 4: cargo check**

```bash
cargo check -p warp
```

Expected: 绿。修任何未使用 import / 穷尽 match。

- [ ] **Step 5: Commit**

```bash
git add app/src/workspace/view.rs app/src/workspace/view/left_panel.rs
git commit -m "feat: sync running workspace agent activity into the project tree"
```

---

### Task 8: 规格对齐与回归

**Files:**
- Modify: `specs/repository-workspaces/TECH.md`（在 project tree / tab 同步段落补一句：workspace 行活动槽消费 `WorkspaceAgentActivity`，与 long-running id 集合互斥渲染）
- 不改 PRODUCT.md 行为 39，除非实现偏离设计。

- [ ] **Step 1: 跑本功能相关单测**

```bash
cargo test -p warp --lib project_organization::workspace_agent_activity_tests
cargo test -p warp --lib workspace::repository_workspace_tabs_tests::map_last_matching
cargo test -p warp --lib terminal::cli_agent_tests::grok_has_brand_icon
cargo test -p warp --lib ui_components::breathing_ring_tests
cargo test -p warp --lib project_organization::view::project_tree_tests
```

Expected: 全部 PASS。

- [ ] **Step 2: cargo check**

```bash
cargo check -p warp
```

Expected: 绿。

- [ ] **Step 3: 手工验收清单（实现者在本地 Flag 打开后点一次）**

1. workspace A 开 grok CLI 并让它 InProgress → A 行出现 Grok 头像和呼吸环，绿点消失。
2. 切到 workspace B → A 行头像仍在。
3. grok 进入 Blocked → 环变静态黄，头像留下。
4. grok 结束且没有 shell 长任务 → 头像消失。
5. 仅 `sleep 30` 长任务 → 绿点，无头像。
6. 原生 Oz InProgress → Oz 头像，不是当前模型品牌。
7. 点击行仍只切换 workspace。
8. 右侧 tab count 数字不变。

- [ ] **Step 4: Commit TECH.md if changed**

```bash
git add specs/repository-workspaces/TECH.md
git commit -m "docs: describe workspace agent activity slot"
```

若 TECH.md 无需改动则跳过本 commit。
