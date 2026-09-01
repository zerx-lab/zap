# Repository Workspaces 产品规格

## Summary

为 Zap 增加本机项目组织能力，以 repository 作为一级组织单元，以独立 Git worktree workspace 作为二级组织单元，以页签作为 workspace 的子节点。每个 workspace 管理自己完整的页签集合，并允许在同一 workspace 内运行多个独立终端及其他 PaneGroup 内容。侧栏打开时用树切换页签，顶部改为当前 workspace 的 Git 信息栏。

## Problem

当前产品只通过 TabConfig 创建一次性 worktree 页签，缺少 repository、workspace 生命周期、稳定归属关系和多页签恢复能力。用户无法按仓库和工作空间管理并行开发任务，也无法安全地创建、切换和删除这些工作单元。

## Goals

- 提供清晰的 repository → workspace → 页签三层组织结构。
- 保持每个 workspace 的 Git 目录和分支隔离。
- 页签状态（含 agent 身份）展示在对应树节点上，避免 workspace 行聚合头像造成误读。
- 侧栏打开时用顶部信息栏展示当前分支、upstream 和未提交行数；侧栏收起时恢复 TabBar。
- 复用现有 PaneGroup、分屏和会话恢复体验。
- 为创建、迁移和删除操作提供明确且保守的安全语义。
- 首期仅保存本机状态，并通过 Dogfood Feature Flag 灰度。

## Non-goals

- 跨设备同步 repository、workspace、worktree 或终端状态。
- Pull Request workspace、远程主机 workspace、端口管理或 setup/teardown scripts。
- 替换现有终端、PaneGroup 或分屏模型。
- 在项目组织模式下同时显示 Vertical Tabs。
- 第一期不把未归类页签展开为树子节点。
- 第一期不在树上跨 workspace 拖拽页签，也不从树节点拖出拆窗。
- 第一期信息不栏不展示 worktree 路径或内存 / CPU。
- 不在折叠的 workspace 行上叠多个 agent 头像或显示 +N。

## Figma

Figma: none provided. 交互和布局以本规格确认过程中批准的视觉方案为准。页签进树与信息栏的已确认决策见 `docs/plans/2026-08-31-workspace-tabs-in-tree-design.md`。

## References

- Superset Your First Workspace: https://docs.superset.sh/first-workspace
- Superset Workspaces: https://docs.superset.sh/workspaces
- Superset Terminal: https://docs.superset.sh/terminal-integration

## Behavior

1. 当 `RepositoryWorkspaces` Feature Flag 启用时，主窗口左侧显示 repository → workspace → 页签三层树。左侧 ToolsPanel 打开且当前是真正的 repository workspace 时，侧栏通顶，右侧内容列顶部是 workspace 信息栏而不是页签列表；侧栏收起，或当前是未归类页签时，顶部恢复 TabBar。项目组织模式不显示 Vertical Tabs，也不修改用户原有的 Vertical Tabs 设置值。

2. 当 Feature Flag 关闭时，用户继续看到原有项目和页签体验。关闭 Flag 不删除已经保存的 repository、workspace 或页签归属数据。

3. repository 树支持展开、折叠、选择、重命名、刷新、在文件管理器中打开和移除。repository 默认名称取目录名，重命名只改变显示名称，不重命名磁盘目录或 Git remote。

4. 用户可以通过选择本地目录添加 repository。所选目录必须是 Git 主工作目录；linked worktree、普通目录、缺失目录和不可访问目录均被拒绝，并显示具体原因。

5. 用户可以通过 Git URL 添加 repository。界面提供 Git URL 和 clone 目标路径，目标路径默认位于 `~/.warp/repositories/` 下，且允许在开始 clone 前修改。

6. clone 期间界面显示明确进度并阻止重复提交。clone 失败时保留用户输入并展示 Git 错误；产品只清理本次操作新建的、不含用户既有内容的目标目录。

7. 同一规范化本地路径只能添加一次。重复添加时选中已有 repository，而不是创建重复记录。

8. repository 下存在 workspace 时不能移除 repository，界面要求用户先处理这些 workspace。

9. 移除 repository 默认只删除 Zap 中的组织记录，不删除本地仓库。仅当 repository 由 Zap clone 时，界面额外提供“同时删除本地仓库目录”复选框，默认不选中。

10. 每个 repository 行提供创建 workspace 的明确入口。workspace 创建界面使用“从远端分支新建”和“关联本地分支”两个互斥模式。

11. “从远端分支新建”模式在打开时刷新远端引用，允许搜索和选择远端分支，并基于该远端分支创建新的本地分支和独立 worktree。

12. 远端模式允许用户输入新本地分支名，也允许启用自动生成。自动生成默认开启；生成名称必须在当前 repository 的本地分支和 worktree 中唯一。

13. “关联本地分支”模式只列出已有本地分支。若分支已在主仓库或其他 worktree 中检出，该分支不可创建 workspace，界面显示占用它的路径。

14. 每个 workspace 始终对应一个 repository、一个本地分支和一个独立 worktree 路径。同一 repository 下不能有两个 workspace 使用同一本地分支。

15. worktree 默认位于 `~/.warp/worktrees/<repository>/<safe-branch>-<short-id>/`。默认目录名把分支中的路径分隔符和不适合文件名的字符转换为 `-`，并添加短 ID 保证唯一；用户可在创建前修改。目标路径已存在、不可写或位于冲突位置时阻止创建。

16. workspace 默认显示名称等于最终本地分支名。用户可以独立重命名 workspace；重命名不改变 Git 分支名。树行在辅助信息中始终显示真实分支名。

17. 用户在开始 Git 操作前可以取消创建。创建开始后界面进入不可重复提交的进度状态；完成或失败前不接受第二个创建请求。

18. workspace 只有在 worktree、数据库记录和首个终端页签均创建成功后才显示为可用。任一步骤失败时，界面显示原始可操作错误，并撤销本次操作创建的资源。

19. 新 workspace 创建成功后自动成为当前 workspace，并打开一个以其 worktree 为启动目录的终端页签。

20. 在 workspace 中创建的新页签自动归属当前 workspace。终端默认从 workspace 的 worktree 路径启动；页签仍可使用现有 PaneGroup、分屏、重命名、颜色能力。侧栏打开时，新建页签的可见入口是对应 workspace 行上的 `+`：若该 workspace 已是当前 workspace，则直接新建并激活；否则先切换到该 workspace 再新建并激活。快捷键新建页签的语义不变，仍归属当时的当前 workspace。

21. workspace 管理完整页签集合，而不只管理终端页签。AI、Notebook、代码或混合 PaneGroup 页签与终端页签使用相同归属和切换规则。

22. 点击 workspace 父节点时，切换到该 workspace 并恢复该窗口上次活动的页签；若该节点未展开则展开，并高亮对应页签子节点。其他 workspace 的终端和后台进程继续运行，不因切换而关闭或重启。侧栏打开时，该 workspace 的页签作为子节点出现在树里，不出现在顶部栏；侧栏收起后顶部 TabBar 只显示当前 workspace 的页签。

23. workspace 没有页签时显示空状态，并提供“新建终端”操作。空 workspace 仍然有效，不会被自动删除。

24. 每个窗口独立记住当前 workspace，以及每个 workspace 的活动页签。应用重启后恢复相同的窗口、workspace、页签顺序和活动位置。

25. 跨窗口拖动页签时保留其 workspace 归属；目标窗口切换到该 workspace 并激活被拖入的页签。第一期树节点只支持同一 workspace 内排序；跨 workspace 拖拽和拖出拆窗只在侧栏收起后的 TabBar 上保留。

26. 左侧树底部提供“未归类页签”入口。非 Git 页签、位于主仓库工作目录的旧页签，以及无法可靠映射到 linked worktree 的页签进入该集合。第一期未归类不展开为页签子节点；选中未归类后顶部仍使用 TabBar 展示这些页签。

27. 首次启用功能时，Zap 自动把现有项目路径迁移为 repository，并根据 Git common directory 识别 linked worktree。位于同一 linked worktree 的现有页签归入同一个 workspace；迁移不得移动目录、切换分支或重启终端。

28. 自动迁移无法确定唯一归属时，页签保留在“未归类页签”，不得通过猜测建立错误关联。

29. 删除 workspace 前，Zap 检查 worktree 是否存在、是否有未提交或未跟踪改动，以及分支是否已经合并。任何安全检查失败时，不关闭页签、不终止终端、不修改 Git，也不删除记录。

30. workspace 有未提交或未跟踪改动时，删除操作被阻止。用户必须先自行处理改动；首期不提供丢弃改动的快捷方式。

31. 删除对话框默认选中“同时删除本地分支”。用户取消勾选时只移除 worktree 和 workspace 记录，保留本地分支。

32. 当“同时删除本地分支”已选中且分支未合并到其 upstream 或 repository 默认分支时，Zap 在任何破坏性操作发生前显示第二次强制删除确认。用户取消确认后不改变任何状态。已配置但 remote-tracking ref 已消失的 upstream 视为没有可用 upstream，按 repository 默认分支判断合并状态，不得因此阻断删除。

33. 删除确认通过后，Zap 关闭该 workspace 的全部页签和终端、移除 worktree、按选择安全或强制删除本地分支，最后移除 workspace 记录。

34. repository 主目录、workspace worktree 或 Git 分支在 Zap 外部被移动、删除或修改时，树行显示明确错误状态。用户可以重新定位 repository/worktree 或移除失效记录；Zap 不静默改用其他目录或分支。

35. clone、fetch、分支创建、worktree 创建和删除错误必须保留 Git stderr 中的关键原因，同时提供面向用户的操作上下文。错误不得被空列表、自动选择或无提示关闭所掩盖。

36. 长时间 Git 操作不阻塞窗口渲染。对应 repository/workspace 行显示进行中状态，并禁止会与当前操作冲突的重命名、删除或重复创建操作。

37. 三层树、创建弹窗和删除确认支持键盘焦点、方向键导航、Enter 确认和 Escape 取消。在树上，方向键在 repository / workspace / 页签节点间移动；Enter 激活聚焦的 workspace 或页签。图标按钮提供可访问名称或 Tooltip，文本和状态在浅色、深色主题下均使用现有主题色。新建、关闭、Ctrl+Tab、Cmd/Ctrl+1..9 等现有页签快捷键语义不变。

38. repository 和 workspace 的路径、分支和页签归属仅在本机持久化，不上传到云端，也不在其他设备自动创建或恢复。

39. 每个页签子节点左侧活动槽展示该页签自己的状态，而不是把多个页签聚合到 workspace 行上。agent 处于 InProgress 或 Blocked 时显示对应品牌圆形图标：CLI 会话用该 CLI 的品牌图标，原生 Oz / Warp Agent 用 Oz 图标。InProgress 时图标外圈以品牌色做缓慢呼吸；Blocked 时外圈改为静态警告黄。会话结束、取消或出错后图标立即消失。没有 agent 活动但存在 shell 长任务时，活动槽为 6px 绿点；都没有时显示与当前 TabBar 一致的页签类型图标。切到其他 workspace 后，后台仍在跑的页签子节点继续显示自己的状态。详情见 `docs/plans/2026-08-31-workspace-tabs-in-tree-design.md`。

40. 页签子节点的标题与侧栏收起后的 TabBar 使用同一套规则。自定义名永远优先。聚焦 pane 是 terminal 时，标题依次为：CLI agent / 对话标题、与 cwd 不同的长任务 OSC 标题、最近一条有信息量的命令（跳过 `cd` / `ls` / `pwd` / `clear` / `exit` 等）、否则按当前 workspace 从左到右编号为 `Terminal 1`、`Terminal 2`。非 terminal 页签仍用原 display_title。路径留在 hover tooltip，不进标题。**选中高亮在当前活动页签子节点上**，不在其 workspace 父节点上。空 workspace 没有子节点时，父节点自己处于选中，右侧显示现有空状态，并提供“新建终端”。

41. 点击页签子节点激活该页签；若它属于后台 workspace，先切换到该 workspace 再激活。hover 子节点右侧显示关闭按钮，关闭规则与现有页签相同（含未保存确认）。关掉最后一个页签后 workspace 变为空，不删除 workspace、不终止未关联的 Git 状态。右键打开现有页签上下文菜单。第一期不支持树上双击重命名。

42. 同一 workspace 内可通过拖拽页签子节点排序，顺序与侧栏收起后 TabBar 中的顺序一致，并随窗口快照持久化。

43. 折叠的 workspace 不显示具体 agent 头像。若任一子页签（含后台页签）有 agent 处于 InProgress / Blocked 或存在 shell 长任务，父节点活动槽显示通用绿点；页签数仍显示在父节点右侧。展开后绿点/头像落在各子节点上。

44. 侧栏打开且当前是真正的 repository workspace 时，内容列顶部信息栏显示：真实 Git 分支名；若存在仍可用的 upstream，则显示 `from <upstream 短名>`，否则省略该段；相对 HEAD 的未提交改动行数 `+n −n`（含暂存与 untracked 文本行）。`+n` 与 `−n` 都为 0 时不显示数字。Git 失败或 worktree 丢失时不显示假数字，workspace 树行走现有错误态。信息栏高度与原 TabBar 相同，窗控、红绿灯和拖动窗口行为不变。

45. 信息不栏第一期不展示 worktree 路径、内存或 CPU。信息栏也没有新建页签按钮。
