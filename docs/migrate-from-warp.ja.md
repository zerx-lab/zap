# Zap への設定の移行

[English](./migrate-from-warp.md) · [简体中文](./migrate-from-warp.zh-CN.md)

このガイドは、**設定系の構成**(カスタムキーバインド・テーマ・ワークフロー・
MCP 設定など)を以前のインストールから Zap へ引き継ぎたい方向けです。

移行元として想定されるのは 2 つあり、**両者では安全性プロファイルが異なる**ため、
本書では別々のセクションで扱います。両方該当する場合は、**まず OpenWarp から
移行してから** Warp の移行を検討してください。

1. **OpenWarp** —— Zap の以前の名称。プロジェクトが Zap に改名される前から
   使っていた場合、旧設定は `~/.openwarp/` に残っている可能性が高いです。
2. **上流の [Warp](https://github.com/warpdotdev/warp)** —— Zap が fork して
   いるプロジェクト。設定は `~/.warp/` 配下にあります。

本書は コマンド履歴・SQLite データベース・Drive オブジェクト・認証情報の
いずれも **意図的に対象外** としています。これらはマシン依存のストア
(Keychain / DPAPI / libsecret)、または schema が結合したストアに格納されて
いるため、コピーは安全ではありません。

---

## 1. OpenWarp からの移行(既存ユーザーの推奨経路)

### このケースが安全な理由

OpenWarp は改名前の Zap そのものです。改名コミット
(`feat: rename project Warp/OpenWarp → Zap`)は識別子とディスク上のパス名を
変更しただけで、**設定ファイルのフォーマットと schema は一切変わっていません**。
ディスク上の位置の対応は以下の通りです:

| プラットフォーム | OpenWarp 設定ディレクトリ | Zap 設定ディレクトリ |
|---|---|---|
| macOS | `~/.openwarp/` | `~/.zap/` |
| Linux | `~/.openwarp/` | `~/.zap/` |
| Windows | `%USERPROFILE%\.openwarp\` | `%USERPROFILE%\.zap\` |

schema が一致しているため、下記 7 項目をそのままコピーすれば Zap はそのまま
読み込みます。

| ファイル / ディレクトリ | 役割 |
|---|---|
| `settings.toml` | 公開設定(TOML ベースの設定ファイル)。 |
| `keybindings.yaml` | カスタムキーバインド。 |
| `themes/` | カスタムテーマ。 |
| `workflows/` | カスタムワークフロー。 |
| `launch_configs/` | Launch 設定。 |
| `.mcp.json` | MCP サーバー設定。 |
| `skills/` | Agent skills。 |

### 手順

> コピー前に **Zap を終了**してください。プロセスがファイルを掴んでいると
> 失敗します。

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

`[ ! -e ... ]` / `-not (Test-Path $to)` のガードは、Zap 側で既にカスタマイズ
した内容を上書きしないためのものです。OpenWarp の値で上書きしたい場合は外して
ください。

Zap が問題なく動くことを確認したら、`~/.openwarp`(または Windows 上の同等
ディレクトリ)を削除してディスク領域を回収できます。もう誰も使っていません。

---

## 2. 上流 Warp からの移行

### このケースが別扱いになる理由

上流 Warp は独立した別プロダクトで、独自のディスク上アイデンティティを持ち
ます:

| プラットフォーム | Warp 設定ディレクトリ | Zap 設定ディレクトリ |
|---|---|---|
| macOS | `~/.warp/` | `~/.zap/` |
| Linux | `~/.warp/` | `~/.zap/` |
| Windows | `%USERPROFILE%\.warp\` | `%USERPROFILE%\.zap\` |

Zap はビルド時の channel が `Oss` で、独自の app id(`dev.zap.Zap`)と独自の
設定ディレクトリを持ちます。両者のインストールは互いに相手のファイルを見る
ことができません —— これは Warp のアカウントやクラウド状態を Zap に持ち込ま
ないための仕様でもあります。

下記のテキスト形式ファイルは schema が安定しており、コピーしても安全です。
**それ以外はそうではありません** —— Warp は Zap とは独立に進化しており、
バイナリ / プライベートストアは Warp の認証や bundle 識別子に紐付いている
場合があります。

### コピー対象

セクション 1 と同じ 7 項目です:

| ファイル / ディレクトリ | 役割 |
|---|---|
| `settings.toml` | 公開設定(TOML ベースの設定ファイル)。 |
| `keybindings.yaml` | カスタムキーバインド。 |
| `themes/` | カスタムテーマ。 |
| `workflows/` | カスタムワークフロー。 |
| `launch_configs/` | Launch 設定。 |
| `.mcp.json` | MCP サーバー設定。 |
| `skills/` | Agent skills。 |

### コピー**してはいけない**もの

- **`user_preferences.json`** —— macOS の
  `~/Library/Application Support/dev.warp.Warp/`(Linux / Windows ではそれに
  相当する state ディレクトリ)にあるプライベートストアで、ユーザー設定・
  認証トークン・マシン依存 ID・クラウドキャッシュが混在しています。ファイル
  ごとコピーすると ID 情報が漏れたり、Zap の認証状態が壊れたりします。
  Zap のデフォルトはもともとプライバシー優先なので、**触らないでください**。
- **`warp.sqlite`**(および `-wal` / `-shm` のサイドカー)—— schema が上流
  Warp に結合しており、Zap の migration が通る保証はありません。
- **Keychain / DPAPI / libsecret のエントリ** —— Warp の bundle / service 名に
  紐付いており、Zap からは利用できません。

### 手順

> コピー前に **Warp と Zap を終了**してください。

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

元の `~/.warp` には一切手を加えません —— Warp 本体はそのまま使い続けられます。

---

## 確認方法

Zap を起動すると、テーマピッカーにカスタムテーマ、キーバインドエディタに
カスタムキーバインド、ワークフローランチャーにカスタムワークフローが見える
はずです。`settings.toml` に存在するキーは、設定 UI 上でも移行元の値が反映
されます。

うまく反映されない場合、原因は上記 7 ファイルのいずれかに含まれています ——
テキストエディタで開いて確認するか、削除して Zap のデフォルトに戻して
ください。

## ロールバック

本書の操作はいずれも破壊的ではありません。コピーしたファイルはどれも Zap が
起動時にデフォルトから再生成できるものです。すべてを元に戻すには:

```sh
# macOS / Linux
rm -rf ~/.zap
```

Windows:

```powershell
Remove-Item -Recurse -Force "$env:USERPROFILE\.zap"
```

`~/.openwarp` と `~/.warp` はいずれも本ガイドからは変更されません。
