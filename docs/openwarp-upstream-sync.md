# openWarp 上游同步指南

## 一次性配置(每个 clone)

```bash
bash script/setup-merge-drivers.sh
```

这会启用 `rerere`(记忆冲突解析)+ 注册 `openwarp-ours` 合并驱动(`.gitattributes` 中标记的路径永远保留 openWarp 版本)。

## 已知"上游不再合并"的 commit 黑名单

下列 commit 已评估,在 openWarp 中**永久跳过**,后续 sync 时不需要再评估:

| Commit | 标题 | 跳过原因 |
|---|---|---|
| _(目前 0 条)_ | | |

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
