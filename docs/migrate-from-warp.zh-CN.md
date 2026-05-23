# 迁移设置到 Zap

[English](./migrate-from-warp.md) · [日本語](./migrate-from-warp.ja.md)

本文给希望把**设置类配置**(自定义快捷键、主题、工作流、MCP 配置等)从历史安装
带到 Zap 的用户。

可能的"源端"有两种,**两者的安全等级不同**,本文分两节说明。如果两边都有,
**请先迁 OpenWarp,再考虑迁 Warp**。

1. **OpenWarp** —— Zap 自己之前的名字。如果你在改名 Zap 之前就一直在用这个项目,
   旧配置很可能还在 `~/.openwarp/` 下。
2. **上游 [Warp](https://github.com/warpdotdev/warp)** —— Zap 所 fork 的项目,
   配置在 `~/.warp/` 下。

本文**有意不覆盖**命令历史、SQLite 数据库、Drive 对象,以及任何凭证。这些要么
绑定到本机(Keychain / DPAPI / libsecret),要么 schema 与对方强耦合,跨过来
并不安全。

---

## 1. 从 OpenWarp 迁过来(老用户推荐路径)

### 为什么这条路是安全的

OpenWarp 就是改名前的 Zap。改名提交(`feat: rename project Warp/OpenWarp → Zap`)
只改了标识符与磁盘路径名,**配置文件的格式和 schema 完全没变**。磁盘位置变化
如下:

| 平台 | OpenWarp 配置目录 | Zap 配置目录 |
|---|---|---|
| macOS | `~/.openwarp/` | `~/.zap/` |
| Linux | `~/.openwarp/` | `~/.zap/` |
| Windows | `%USERPROFILE%\.openwarp\` | `%USERPROFILE%\.zap\` |

因为 schema 一致,下面这 7 项直接拷过来 Zap 就能原样读取。

| 文件 / 目录 | 控制什么 |
|---|---|
| `settings.toml` | 公开设置(TOML 设置文件)。 |
| `keybindings.yaml` | 自定义快捷键。 |
| `themes/` | 自定义主题。 |
| `workflows/` | 自定义 workflow。 |
| `launch_configs/` | Launch 配置。 |
| `.mcp.json` | MCP server 配置。 |
| `skills/` | Agent skills。 |

### 操作步骤

> 拷贝前**关掉 Zap**,以免有进程持有这些文件。

**macOS / Linux**

```sh
mkdir -p "$HOME/.zap"
for f in settings.toml keybindings.yaml themes workflows launch_configs skills .mcp.json; do
  if [ -e "$HOME/.openwarp/$f" ] && [ ! -e "$HOME/.zap/$f" ]; then
    cp -R "$HOME/.openwarp/$f" "$HOME/.zap/$f"
  fi
done
```

**Windows(PowerShell)**

```powershell
$src = "$env:USERPROFILE\.openwarp"
$dst = "$env:USERPROFILE\.zap"
New-Item -ItemType Directory -Force -Path $dst | Out-Null

$items = @(
  'settings.toml',
  'keybindings.yaml',
  'themes',
  'workflows',
  'launch_configs',
  'skills',
  '.mcp.json'
)

foreach ($name in $items) {
  $from = Join-Path $src $name
  $to   = Join-Path $dst $name
  if ((Test-Path $from) -and -not (Test-Path $to)) {
    Copy-Item -Path $from -Destination $to -Recurse
  }
}
```

`[ ! -e ... ]` / `-not (Test-Path $to)` 这层守卫是为了避免覆盖你已经在 Zap
里改过的内容。如果你就是想让 OpenWarp 的值覆盖掉 Zap,去掉它即可。

确认 Zap 一切正常之后,可以删掉 `~/.openwarp`(或 Windows 对应目录)来回收空间。
它已经不会被任何程序使用了。

---

## 2. 从上游 Warp 迁过来

### 为什么这一节要单独说

上游 Warp 是另一个独立产品,有自己的磁盘身份:

| 平台 | Warp 配置目录 | Zap 配置目录 |
|---|---|---|
| macOS | `~/.warp/` | `~/.zap/` |
| Linux | `~/.warp/` | `~/.zap/` |
| Windows | `%USERPROFILE%\.warp\` | `%USERPROFILE%\.zap\` |

Zap 编译时 channel = `Oss`,对应独立的 app id(`dev.zap.Zap`)与独立的配置目录,
两边互相看不到对方的文件 —— 这也正是 Zap 能让你的 Warp 账号 / 云端状态留在 Warp
那边的原因。

下表里的文本格式文件 schema 稳定、跨过来安全;**其它东西就不一定了** —— Warp
独立演进,二进制 / 私有存储可能绑定到 Warp 的认证和 bundle 身份。

### 可以拷的内容

和第 1 节相同的 7 项:

| 文件 / 目录 | 控制什么 |
|---|---|
| `settings.toml` | 公开设置(TOML 设置文件)。 |
| `keybindings.yaml` | 自定义快捷键。 |
| `themes/` | 自定义主题。 |
| `workflows/` | 自定义 workflow。 |
| `launch_configs/` | Launch 配置。 |
| `.mcp.json` | MCP server 配置。 |
| `skills/` | Agent skills。 |

### **不要**拷的内容

- **`user_preferences.json`** —— 这是私有存储,位于 macOS 上的
  `~/Library/Application Support/dev.warp.Warp/`(Linux / Windows 对应的 state
  目录),里面混杂了用户偏好、登录 token、机器绑定 ID 和云端缓存状态。整文件
  拷过去会泄漏身份信息,也会让 Zap 误判登录状态。Zap 默认值本身已经是隐私
  优先的,**不要碰它**。
- **`warp.sqlite`**(以及 `-wal` / `-shm` 伴生文件)—— schema 与上游 Warp 耦合,
  不保证能跑 Zap 的 migrations。
- **Keychain / DPAPI / libsecret 中的条目** —— 绑定到 Warp 的 bundle / service
  名,对 Zap 没有意义。

### 操作步骤

> 拷贝前**关掉 Warp 与 Zap**。

**macOS / Linux**

```sh
mkdir -p "$HOME/.zap"
for f in settings.toml keybindings.yaml themes workflows launch_configs skills .mcp.json; do
  if [ -e "$HOME/.warp/$f" ] && [ ! -e "$HOME/.zap/$f" ]; then
    cp -R "$HOME/.warp/$f" "$HOME/.zap/$f"
  fi
done
```

**Windows(PowerShell)**

```powershell
$src = "$env:USERPROFILE\.warp"
$dst = "$env:USERPROFILE\.zap"
New-Item -ItemType Directory -Force -Path $dst | Out-Null

$items = @(
  'settings.toml',
  'keybindings.yaml',
  'themes',
  'workflows',
  'launch_configs',
  'skills',
  '.mcp.json'
)

foreach ($name in $items) {
  $from = Join-Path $src $name
  $to   = Join-Path $dst $name
  if ((Test-Path $from) -and -not (Test-Path $to)) {
    Copy-Item -Path $from -Destination $to -Recurse
  }
}
```

`~/.warp` 始终不会被改动,Warp 自己继续可用。

---

## 验证

启动 Zap,你应该能在主题选择器里看到自定义主题,在快捷键编辑器里看到自定义
键位,在 workflow 启动器里看到自定义 workflow。设置界面里所有出现在
`settings.toml` 中的项,值应该和源端一致。

如果哪一项不对,问题一定在上面 7 个文件中的某一个 —— 用文本编辑器打开看看,或者
直接删掉让 Zap 用默认值。

## 回滚

本文里的操作**都不是破坏性的**:拷过去的每个文件都是 Zap 启动时能自动用默认
值重建的。整体回滚:

```sh
# macOS / Linux
rm -rf ~/.zap
```

Windows:

```powershell
Remove-Item -Recurse -Force "$env:USERPROFILE\.zap"
```

`~/.openwarp` 与 `~/.warp` 都不会被本指南改动。
