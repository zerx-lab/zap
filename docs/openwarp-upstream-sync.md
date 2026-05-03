# openWarp 上游同步指南

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
