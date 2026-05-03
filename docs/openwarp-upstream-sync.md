# openWarp 上游同步指南

<<<<<<< HEAD
=======
## 当前同步状态(基线 master = a5fde8f,本地 openWarp tip = e3e6eaf)

上游 0443f3f..origin/master 共 **133 条 commit**,处理结果:

| 状态 | 数量 | 说明 |
|---|---|---|
| **已合入** | **79** | A 类 31 fix + SSH 对齐 4 + B 类 2 + C1 入 5 + C2 入 8 + D 桶 4 通用 fix 28 + 1 个上游 99f80df 通过 SSH 整族对齐间接合入 |
| **永久黑名单** | **54** | cloud / codex / OzHandoff / orchestration / cloud_mode 链路 + workflow + STAKEHOLDERS 治理 + 上游内部 docs |

**所有 133 条 commit 已 100% 明确归属**,黑名单写在下方表格,后续 sync 用同一份判断标准。

>>>>>>> origin/main
## 一次性配置(每个 clone)

```bash
bash script/setup-merge-drivers.sh
```

这会启用 `rerere`(记忆冲突解析)+ 注册 `openwarp-ours` 合并驱动(`.gitattributes` 中标记的路径永远保留 openWarp 版本)。

## 已知"上游不再合并"的 commit 黑名单

下列 commit 已评估,在 openWarp 中**永久跳过**,后续 sync 时不需要再评估:

> **为什么不能用 `merge=openwarp-ours` 路径排除来代替?** 已实验验证:把 ai/agent_sdk/、blocklist/、ambient_agent/、slash_commands/ 等路径加进自治区后,这 10 个 commit 物理上能 cherry-pick(冲突自动消化),**但新增文件**(codex.rs / wake_driver.rs / orchestration_event_streamer 等)会引用自治区中已不存在的字段、enum 变体、trait 方法,导致 **85+ 编译错误**。修这些错误需要逐个补回 openWarp 已删的 cloud/orchestration API,得不偿失。所以保持 commit 级黑名单。

| Commit | 标题 | 跳过原因 |
|---|---|---|
| `b59e351` | add /continue-locally slash command | 依赖 cloud Oz handoff(`conversation_is_cloud_oz_for_slash_command` 已删) |
| `9551831` | Initial codex CLI harness setup | `load_conversation_from_server` 在 openWarp 已 stub 为 None |
| `70c725f` | Conversation resuming for codex | 依赖 9551831 的 cloud 加载链路 |
| `2bdbb61` | Save and upload codex conversation transcript | cloud 链路 |
| `5c89948` | add hook for file editing | snapshot DeclarationsWriter 走 OzHandoff cloud 链路 |
| `1148ae3` | Wake up remote Claude Code agents on new events | cloud agent orchestration,5 处冲突且本质 cloud-tied |
| `6995005` | Scope orchestration SSE subscriptions | cloud orchestration SSE 流,openWarp 无 |
| `1314819` | Merge org and user command denylists | UI 重写,与 openWarp i18n + render_list_section 路径完全不同 |
| `fd8e0fb` | preserve user query modes in CloudMode | CloudMode UI,openWarp 已删 cloud 路径 |
| `71054d6` | Remove `NotAmbientAgent` state | 大型 ambient_agent 重构,32 处冲突,openWarp 已分叉 |
| `99f80df` | Fix bad merge for remote server(替代 SSH 对齐分支) | 已通过整族对齐(f0c8b7f→b19866a→99f80df→e75b315)合入,单独路径过时 |
| `6eefa4b` | OSS .desktop align Exec | openWarp 用 `warp-oss` + `OpenWarp`,与上游 `warp-terminal-oss` 命名分叉 |
<<<<<<< HEAD
=======
| `4dddda6` | Preseed auth and trust settings for codex CLI | codex CLI harness 是 cloud-tied,openWarp BYOP 不需要 |
| `5762baa` | feature flag + API binding scaffolding for cloud→cloud handoff | cloud→cloud 编排,openWarp 已删 cloud_conversations |
| `0ab9e71` | Orchestration pills bar in Agent View (1/N) | orchestration UI,依赖已删 orchestration_event_streamer |
| `88930cf` | Cache settings schema between Linux builds | openWarp 用自己的 openwarp_release.yml |
| `99b287f` | ci: simplify external contributor check | openWarp 自有 workflow |
| `0fca61d` | ci: label external-contributor PRs | openWarp 自有 workflow |
| `805b3e2` | Increase timeout for linux builds | openWarp 自有 workflow |
| `404bfbe` | ci: remove workflows now served by Vercel webhook | openWarp 不接 Vercel webhook |
| `874a257` | Add stakeholders for `lsp` and `languages` crates | Warp 内部 code owners 治理,与 fork 无关 |
| `d1601f5` | add stakeholders(vertical tabs / tab configs / worktree / notifications / rich input) | 同上 |
| `67b929c` | Add @harryalbert as CLI agent stakeholder | 同上 |
| `33c4885` | Add vkodithala as co-owner of skills/MCP/long-running commands | 同上 |
| `a12d9e4` | Add more UI framework stakeholders | 同上 |
| `182c1ac` | chore: assign / route to @warpdotdev/oss-maintainers in STAKEHOLDERS | 同上 |
| `73074ba` | remove @moirahuang from context chips stakeholders | 同上 |
| `1849795` | Point stable-skill instructions at resources/bundled/skills/ | Warp 内部 stable channel 用,openWarp 走 oss channel 无关 |
| `bb5edc0` | Drop warp-internal references from docker/linux-dev README | 内部 dogfood docker 文档,openWarp 不用 |
| `33c4860` | Update env_vars README to match current file layout | 上游 README 内部路径调整 |
| `b740b82` | Update persistence README paths to crates/persistence | 同上 |
| `799e13f` | docs: simplify PR template for public contributors | Warp 自己的 PR 模板 |
| `6898ac2` | docs: surface #oss-contributors Slack channel | Warp 自己的 Slack |
| `ed0cdae` | docs: attribute Alacritty/vte derivative code(2 more files) | 上游 license 归属 |
| `a8f57a8` | Clarify `alacritty_terminal` origins for terminal model code | 同上注释类 |
| `7784428` | Remove stray backticks from Windows installer README | Warp Windows installer README,openWarp 用 openwarp_release.yml 自己生成 |
| `b7c64bc` | Add Build Status section linking to build.warp.dev | build.warp.dev 是 Warp 内部 dashboard |
| `acb2fc6` | Add telemetry events for git button clicks | telemetry 事件,openWarp 不上报到 Warp 后端 |
| `d0f045c` | Auto oss vs cost efficient 50/50 A/B test | Warp 实验框架 + 计费路径 |
| `79df582` | Initialize privacy settings from `WarpDrivePrivacySettings` | WarpDrive 是 cloud,openWarp 不接 |
| `899d966` | Show all personal runs in the conversation list | cloud personal runs,需要 server 支持 |
| `9eaee8f` | Add experiment setup for SSH | 实验框架 |
| `4ac7378` | Rename Warp Agent to Warp | cloud "Warp Agent" 品牌,openWarp 用本地名 |
| `e058136` | Slash command menu working(cloud mode input v2) | cloud_mode_v2_view 已删 |
| `199cd94` | Slash command menu sidecar(cloud mode input v2) | 同上 |
| `9b3a990` | Enabled cloud mode input v2 on dogfood | 同上 |
| `157f358` | Introduce `/harness` `/host` `/environment` slash commands | cloud mode 新命令,openWarp 删 cloud_mode_v2 |
| `aa2ac33` | Skip onboarding UIs in SDK/headless mode | SDK / headless 是 cloud-tied 模式 |
| `0ac090c` | [REMOTE-1326] Link shared sessions to local interactive Oz runs | Oz orchestration |
| `10ec3d1` | Hide host selector menu if no default host present | cloud host selector,openWarp 无 |
| `ac493e6` | Auto-open rich input for non-Oz harness cloud agent sessions | cloud agent |
| `6184f4e` | Refactor AmbientAgentViewModel to handle follow-up run executions | 自治区核心,与 71054d6 同代次重构 |
| `f696f5b` | Revert "Fix schema generator binary recompilation" | 上游回滚一个 commit,openWarp 没合那个原 commit |
| `159a0bf` | ci: remove broken oz-for-oss adapter workflows | Warp 内部 workflow |
| `59fc1a9` | use multi-harness cloud agent icons + status | cloud agent UI |
>>>>>>> origin/main

## openWarp 已删除/特化的模块(合并时若被恢复,需手工删除)

| 模块 / 路径 | 删除原因 | 处理方式 |
|---|---|---|
| `cloud_conversations` 全家桶 | openWarp BYOP 不接 Warp 云 | 上游若新增此目录文件,直接 `git rm` |
| AI 回复 footer 点赞/点踩(`render_response_footer` 中的 thumbs up/down) | 移除 telemetry 反馈链路 | 上游若改 output.rs 这段,保留 openWarp 版 |
| 智能体署名 `AgentAttributionWidget` + `AISettings.agent_attribution_enabled` | 不需要 | 上游若修改,丢弃 |
| Oz 更新日志 toggle UI | 仅删 UI/action/keybinding,字段保留 | 同上 |
| `app/src/pane_group/mod_tests.rs` 等 9 个 _tests.rs(b120bbe 配套删除) | 类型已删 | 上游 typo fix 触及时 `git rm` |
| `conversation_is_cloud_oz_for_slash_command` 函数 | cloud_oz 路径已删 | 上游引入时丢弃 |

## 合并流程

1. `git fetch origin master`
2. 创建 worktree:`git worktree add ../warp-merge -b merge-upstream-<date> openWarp`
3. 在 worktree 内:
   - `git log --reverse --oneline openWarp..origin/master` 列出待评估 commit
   - 跳过黑名单中的(若有)
   - 按拓扑顺序 cherry-pick
4. `merge=openwarp-ours` 路径自动保留本地版本,无需手工解决
5. modify/delete 类冲突直接 `git rm`(参考上表)
6. 其它冲突手工解决;rerere 会记下来
7. `cargo check -p warp` 验证后合回 openWarp
