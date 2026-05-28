# 迁移设置到 Zap

[English](./migrate-from-warp.md) · [日本語](./migrate-from-warp.ja.md)

本文给希望把**设置类配置**(自定义快捷键、主题、工作流、MCP 配置等)从历史安装
带到 Zap 的用户。

可能的"源端"有两种,**两者的安全等级不同**,本文分两节说明。如果两边都有,
**请先迁 OpenWarp,再考虑迁 Warp**。

1. **OpenWarp** —— Zap 自己之前的名字。
2. **上游 [Warp](https://github.com/warpdotdev/warp)** —— Zap 所 fork 的项目。

本文**有意不覆盖**命令历史、SQLite 数据库、Drive 对象,以及任何凭证。这些要么
绑定到本机(Keychain / DPAPI / libsecret),要么 schema 与对方强耦合,跨过来
并不安全。

---

## 磁盘布局总览

Zap(以及 OpenWarp / 上游 Warp)把磁盘状态分成**三类目录**:

- **config** —— `settings.toml`、`keybindings.yaml`
- **data** —— `themes/`、`workflows/`、`launch_configurations/`、`tab_configs/`
- **home dotfile** —— `.mcp.json`、`skills/`

macOS 上三类目录都收敛到同一个 home dotfile 目录(`~/.warp/`、`~/.openwarp/`
或 `~/.zap/`);Linux 上按 XDG 规范分到**三个不同的位置**,Windows 上按
`directories` crate 的等价布局分。下面的迁移脚本会按平台把每个文件放到
正确的目标。

### Zap 目标路径

| 类别 | macOS | Linux | Windows |
|---|---|---|---|
| config | `~/.zap/` | `${XDG_CONFIG_HOME:-~/.config}/zap/` | `%LOCALAPPDATA%\zap\Zap\config\` |
| data | `~/.zap/` | `${XDG_DATA_HOME:-~/.local/share}/zap/` | `%APPDATA%\zap\Zap\data\` |
| home dotfile | `~/.zap/` | `~/.zap/` | `%USERPROFILE%\.zap\` |

### OpenWarp 源路径

| 类别 | macOS | Linux | Windows |
|---|---|---|---|
| config | `~/.openwarp/` | `${XDG_CONFIG_HOME:-~/.config}/openwarp/` | `%LOCALAPPDATA%\openwarp\OpenWarp\config\` |
| data | `~/.openwarp/` | `${XDG_DATA_HOME:-~/.local/share}/openwarp/` | `%APPDATA%\openwarp\OpenWarp\data\` |
| home dotfile | `~/.openwarp/` | `~/.openwarp/` | `%USERPROFILE%\.openwarp\` |

### 上游 Warp 源路径

| 类别 | macOS | Linux | Windows |
|---|---|---|---|
| config | `~/.warp/` | `${XDG_CONFIG_HOME:-~/.config}/warp-terminal/` | `%LOCALAPPDATA%\warp\Warp-Terminal\config\` |
| data | `~/.warp/` | `${XDG_DATA_HOME:-~/.local/share}/warp-terminal/` | `%APPDATA%\warp\Warp-Terminal\data\` |
| home dotfile | `~/.warp/` | `~/.warp/` | `%USERPROFILE%\.warp\` |

> Linux 下目录名 `warp-terminal` 与 Linux 软件包名一致(例如 Debian/Ubuntu 下
> `/opt/warpdotdev/warp-terminal/`)。Windows 上的 organization 文件夹名可能因
> 打包方式而异;如果你在 `%APPDATA%\warp\Warp-Terminal`(或 `%LOCALAPPDATA%\warp\Warp-Terminal`)
> 找不到,请检查 Warp 实际使用的 `%APPDATA%` / `%LOCALAPPDATA%` 路径。

---

## 1. 从 OpenWarp 迁过来(老用户推荐路径)

OpenWarp 就是改名前的 Zap。改名提交(`feat: rename project Warp/OpenWarp → Zap`)
只改了标识符与磁盘路径名,**配置文件的格式和 schema 完全没变**,下面这些文件
可以直接拷过来。

### 可以拷的内容

| 文件 / 目录 | 类别 | 控制什么 |
|---|---|---|
| `settings.toml` | config | 公开设置(TOML 设置文件)。 |
| `keybindings.yaml` | config | 自定义快捷键。 |
| `themes/` | data | 自定义主题。 |
| `workflows/` | data | 自定义 workflow。 |
| `launch_configurations/` | data | Launch 配置。 |
| `tab_configs/` | data | Tab 配置。 |
| `.mcp.json` | home dotfile | MCP server 配置。 |
| `skills/` | home dotfile | Agent skills。 |

### 操作步骤

> 拷贝前**关掉 Zap**,以免有进程持有这些文件。

**macOS**

```sh
mkdir -p "$HOME/.zap"
for f in settings.toml keybindings.yaml themes workflows launch_configurations tab_configs skills .mcp.json; do
  if [ -e "$HOME/.openwarp/$f" ] && [ ! -e "$HOME/.zap/$f" ]; then
    cp -R "$HOME/.openwarp/$f" "$HOME/.zap/$f"
  fi
done
```

**Linux**

```sh
src_config="${XDG_CONFIG_HOME:-$HOME/.config}/openwarp"
src_data="${XDG_DATA_HOME:-$HOME/.local/share}/openwarp"
src_home="$HOME/.openwarp"

dst_config="${XDG_CONFIG_HOME:-$HOME/.config}/zap"
dst_data="${XDG_DATA_HOME:-$HOME/.local/share}/zap"
dst_home="$HOME/.zap"
mkdir -p "$dst_config" "$dst_data" "$dst_home"

copy() {
  if [ -e "$1/$3" ] && [ ! -e "$2/$3" ]; then
    cp -R "$1/$3" "$2/$3"
  fi
}

copy "$src_config" "$dst_config" settings.toml
copy "$src_config" "$dst_config" keybindings.yaml
copy "$src_data"   "$dst_data"   themes
copy "$src_data"   "$dst_data"   workflows
copy "$src_data"   "$dst_data"   launch_configurations
copy "$src_data"   "$dst_data"   tab_configs
copy "$src_home"   "$dst_home"   .mcp.json
copy "$src_home"   "$dst_home"   skills
```

**Windows(PowerShell)**

```powershell
$src_config = "$env:LOCALAPPDATA\openwarp\OpenWarp\config"
$src_data   = "$env:APPDATA\openwarp\OpenWarp\data"
$src_home   = "$env:USERPROFILE\.openwarp"

$dst_config = "$env:LOCALAPPDATA\zap\Zap\config"
$dst_data   = "$env:APPDATA\zap\Zap\data"
$dst_home   = "$env:USERPROFILE\.zap"
New-Item -ItemType Directory -Force -Path $dst_config, $dst_data, $dst_home | Out-Null

function Copy-IfMissing($srcDir, $dstDir, $name) {
  $from = Join-Path $srcDir $name
  $to   = Join-Path $dstDir $name
  if ((Test-Path $from) -and -not (Test-Path $to)) {
    Copy-Item -Path $from -Destination $to -Recurse
  }
}

Copy-IfMissing $src_config $dst_config settings.toml
Copy-IfMissing $src_config $dst_config keybindings.yaml
Copy-IfMissing $src_data   $dst_data   themes
Copy-IfMissing $src_data   $dst_data   workflows
Copy-IfMissing $src_data   $dst_data   launch_configurations
Copy-IfMissing $src_data   $dst_data   tab_configs
Copy-IfMissing $src_home   $dst_home   .mcp.json
Copy-IfMissing $src_home   $dst_home   skills
```

`[ ! -e ... ]` / `-not (Test-Path $to)` 这层守卫是为了避免覆盖你已经在 Zap
里改过的内容。如果你就是想让 OpenWarp 的值覆盖掉 Zap,去掉它即可。

确认 Zap 一切正常之后,可以删掉上面那些 OpenWarp 目录来回收空间。它们已经不会
被任何程序使用了。

---

## 2. 从上游 Warp 迁过来

上游 Warp 是另一个独立产品,有自己的磁盘身份(见上面"上游 Warp 源路径"表)。
Zap 编译时 channel = `Oss`,对应独立的 app id(`dev.zap.Zap`)和按平台分开的
目录布局。两边互相看不到对方的文件 —— 这也正是 Zap 能让你的 Warp 账号 / 云端
状态留在 Warp 那边的原因。

下表里的文本格式文件 schema 稳定、跨过来安全;**其它东西就不一定了** —— Warp
独立演进,二进制 / 私有存储可能绑定到 Warp 的认证和 bundle 身份。

### 可以拷的内容

和第 1 节相同的 8 项:

| 文件 / 目录 | 类别 | 控制什么 |
|---|---|---|
| `settings.toml` | config | 公开设置(TOML 设置文件)。 |
| `keybindings.yaml` | config | 自定义快捷键。 |
| `themes/` | data | 自定义主题。 |
| `workflows/` | data | 自定义 workflow。 |
| `launch_configurations/` | data | Launch 配置。 |
| `tab_configs/` | data | Tab 配置。 |
| `.mcp.json` | home dotfile | MCP server 配置。 |
| `skills/` | home dotfile | Agent skills。 |

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

**macOS**

```sh
mkdir -p "$HOME/.zap"
for f in settings.toml keybindings.yaml themes workflows launch_configurations tab_configs skills .mcp.json; do
  if [ -e "$HOME/.warp/$f" ] && [ ! -e "$HOME/.zap/$f" ]; then
    cp -R "$HOME/.warp/$f" "$HOME/.zap/$f"
  fi
done
```

**Linux**

```sh
src_config="${XDG_CONFIG_HOME:-$HOME/.config}/warp-terminal"
src_data="${XDG_DATA_HOME:-$HOME/.local/share}/warp-terminal"
src_home="$HOME/.warp"

dst_config="${XDG_CONFIG_HOME:-$HOME/.config}/zap"
dst_data="${XDG_DATA_HOME:-$HOME/.local/share}/zap"
dst_home="$HOME/.zap"
mkdir -p "$dst_config" "$dst_data" "$dst_home"

copy() {
  if [ -e "$1/$3" ] && [ ! -e "$2/$3" ]; then
    cp -R "$1/$3" "$2/$3"
  fi
}

copy "$src_config" "$dst_config" settings.toml
copy "$src_config" "$dst_config" keybindings.yaml
copy "$src_data"   "$dst_data"   themes
copy "$src_data"   "$dst_data"   workflows
copy "$src_data"   "$dst_data"   launch_configurations
copy "$src_data"   "$dst_data"   tab_configs
copy "$src_home"   "$dst_home"   .mcp.json
copy "$src_home"   "$dst_home"   skills
```

**Windows(PowerShell)**

```powershell
$src_config = "$env:LOCALAPPDATA\warp\Warp-Terminal\config"
$src_data   = "$env:APPDATA\warp\Warp-Terminal\data"
$src_home   = "$env:USERPROFILE\.warp"

$dst_config = "$env:LOCALAPPDATA\zap\Zap\config"
$dst_data   = "$env:APPDATA\zap\Zap\data"
$dst_home   = "$env:USERPROFILE\.zap"
New-Item -ItemType Directory -Force -Path $dst_config, $dst_data, $dst_home | Out-Null

function Copy-IfMissing($srcDir, $dstDir, $name) {
  $from = Join-Path $srcDir $name
  $to   = Join-Path $dstDir $name
  if ((Test-Path $from) -and -not (Test-Path $to)) {
    Copy-Item -Path $from -Destination $to -Recurse
  }
}

Copy-IfMissing $src_config $dst_config settings.toml
Copy-IfMissing $src_config $dst_config keybindings.yaml
Copy-IfMissing $src_data   $dst_data   themes
Copy-IfMissing $src_data   $dst_data   workflows
Copy-IfMissing $src_data   $dst_data   launch_configurations
Copy-IfMissing $src_data   $dst_data   tab_configs
Copy-IfMissing $src_home   $dst_home   .mcp.json
Copy-IfMissing $src_home   $dst_home   skills
```

Warp 自己的数据从始至终不会被改动,Warp 本体继续可用。

---

## 验证

启动 Zap,你应该能在主题选择器里看到自定义主题,在快捷键编辑器里看到自定义
键位,在 workflow 启动器里看到自定义 workflow。设置界面里所有出现在
`settings.toml` 中的项,值应该和源端一致。

如果哪一项不对,问题一定在上面 8 个文件中的某一个 —— 用文本编辑器打开看看,或者
直接删掉让 Zap 用默认值。

## 回滚

本文里的操作**都不是破坏性的**:拷过去的每个文件都是 Zap 启动时能自动用默认
值重建的。整体回滚:

```sh
# macOS
rm -rf ~/.zap
```

```sh
# Linux
rm -rf "${XDG_CONFIG_HOME:-$HOME/.config}/zap"
rm -rf "${XDG_DATA_HOME:-$HOME/.local/share}/zap"
rm -rf "$HOME/.zap"
```

```powershell
# Windows
Remove-Item -Recurse -Force "$env:APPDATA\zap"
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\zap"
Remove-Item -Recurse -Force "$env:USERPROFILE\.zap"
```

OpenWarp 与 Warp 的源端目录都不会被本指南改动。
