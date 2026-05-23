# Migrating settings to Zap

[简体中文](./migrate-from-warp.zh-CN.md) · [日本語](./migrate-from-warp.ja.md)

This guide is for people who want to bring **settings-style configuration**
(custom keybindings, themes, workflows, MCP config, etc.) into Zap from a
previous install.

There are two source installs this might apply to:

1. **OpenWarp** — Zap's own previous name. If you used this project before
   it was renamed to Zap, your old config likely still lives in `~/.openwarp`.
2. **Upstream [Warp](https://github.com/warpdotdev/warp)** — the project Zap
   is forked from. Config lives in `~/.warp`.

The two cases have **different safety profiles** and are covered separately
below. Always migrate from OpenWarp first if both apply.

This guide deliberately does **not** cover command history, the SQLite
database, Drive objects, or any credentials. Those live in stores that are
either machine-bound (Keychain / DPAPI / libsecret) or schema-coupled and are
not safe to copy across forks.

---

## 1. From OpenWarp (recommended path for existing users)

### Why this is the safe case

OpenWarp **was** Zap. The rename commit (`feat: rename project Warp/OpenWarp →
Zap`) only renamed identifiers and on-disk paths — the **configuration file
formats and schemas did not change**. On-disk locations changed like this:

| Platform | OpenWarp config dir | Zap config dir |
|---|---|---|
| macOS | `~/.openwarp/` | `~/.zap/` |
| Linux | `~/.openwarp/` | `~/.zap/` |
| Windows | `%USERPROFILE%\.openwarp\` | `%USERPROFILE%\.zap\` |

Because the schemas match, you can copy the **same seven items** listed below
across, and Zap will read them as-is.

| File or folder | What it controls |
|---|---|
| `settings.toml` | Public settings (the TOML-backed settings file). |
| `keybindings.yaml` | Custom keybindings. |
| `themes/` | Custom themes. |
| `workflows/` | Custom workflows. |
| `launch_configs/` | Launch configurations. |
| `.mcp.json` | MCP server configuration. |
| `skills/` | Agent skills. |

### Steps

> Quit Zap before copying, so no process is holding the files open.

**macOS / Linux**

```sh
mkdir -p "$HOME/.zap"
for f in settings.toml keybindings.yaml themes workflows launch_configs skills .mcp.json; do
  if [ -e "$HOME/.openwarp/$f" ] && [ ! -e "$HOME/.zap/$f" ]; then
    cp -R "$HOME/.openwarp/$f" "$HOME/.zap/$f"
  fi
done
```

**Windows (PowerShell)**

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

The `[ ! -e ... ]` / `-not (Test-Path $to)` guard avoids overwriting anything
you might have already set in Zap. Drop it if you'd rather have OpenWarp's
values win.

After verifying Zap looks right, you can delete `~/.openwarp` (or its Windows
equivalent) to reclaim disk space. It's no longer used by anything.

---

## 2. From upstream Warp

### Why this case is different

Upstream Warp is a separate product with its own on-disk identity:

| Platform | Warp config dir | Zap config dir |
|---|---|---|
| macOS | `~/.warp/` | `~/.zap/` |
| Linux | `~/.warp/` | `~/.zap/` |
| Windows | `%USERPROFILE%\.warp\` | `%USERPROFILE%\.zap\` |

Zap is built with channel `Oss`, which gives it its own app ID (`dev.zap.Zap`)
and its own config directory. The two installations cannot see each other's
files, which is also what keeps your Warp account / cloud state out of Zap.

The text-format files listed below have stable, compatible schemas, so copying
them across is safe. **Other state is not** — Warp evolves independently of
Zap, and binary / private stores can be tied to Warp's auth and bundle
identity.

### What to copy

Same seven items as above:

| File or folder | What it controls |
|---|---|
| `settings.toml` | Public settings (the TOML-backed settings file). |
| `keybindings.yaml` | Custom keybindings. |
| `themes/` | Custom themes. |
| `workflows/` | Custom workflows. |
| `launch_configs/` | Launch configurations. |
| `.mcp.json` | MCP server configuration. |
| `skills/` | Agent skills. |

### What **not** to copy

- **`user_preferences.json`** — a private store under
  `~/Library/Application Support/dev.warp.Warp/` (macOS) or the equivalent
  state directory on Linux/Windows. Mixes user preferences with auth tokens,
  machine-bound IDs and cached cloud state. Copying it can leak identity and
  confuse Zap's auth state. Zap defaults are already privacy-friendly.
- **`warp.sqlite`** (and its `-wal` / `-shm` sidecars) — schema is coupled
  to upstream Warp and not guaranteed to be compatible with Zap's migrations.
- **Keychain / DPAPI / libsecret entries** — bound to the Warp bundle /
  service name, useless to Zap.

### Steps

> Quit both Warp and Zap before copying.

**macOS / Linux**

```sh
mkdir -p "$HOME/.zap"
for f in settings.toml keybindings.yaml themes workflows launch_configs skills .mcp.json; do
  if [ -e "$HOME/.warp/$f" ] && [ ! -e "$HOME/.zap/$f" ]; then
    cp -R "$HOME/.warp/$f" "$HOME/.zap/$f"
  fi
done
```

**Windows (PowerShell)**

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

Your original `~/.warp` is never touched — Warp itself keeps working.

---

## Verifying

Start Zap. You should see your custom themes in the theme picker, your
keybindings in the keybinding editor, and your workflows in the workflow
launcher. Settings UI values should reflect what was in `settings.toml`.

If something looks off, the offending file is one of the seven above — open
it in a text editor, or just delete it and let Zap fall back to defaults.

## Rolling back

Nothing in this guide is destructive: every file copied is something Zap will
recreate from defaults on next launch. To undo everything:

```sh
# macOS / Linux
rm -rf ~/.zap
```

or on Windows:

```powershell
Remove-Item -Recurse -Force "$env:USERPROFILE\.zap"
```

`~/.openwarp` and `~/.warp` are never touched by this guide.
