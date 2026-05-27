# gh-110: SSH config import — Tech Spec

Functional spec: `PRODUCT.md`. Tracks GitHub issue #110.

## 1. Module placement

All new code lives in `crates/warp_ssh_manager` (data layer) and
`app/src/ssh_manager` (UI layer). No new crate is introduced. The
existing layering rule from `AGENTS.md` is preserved:
`warp_ssh_manager` stays pure-Rust, no `warpui` dependency.

```
crates/warp_ssh_manager/
├── src/
│   ├── lib.rs                       (export new module)
│   ├── ssh_config_parser.rs         (NEW — parser + IO)
│   ├── repository.rs                (existing — used unchanged)
│   ├── ssh_command.rs               (existing — unchanged)
│   └── types.rs                     (existing — unchanged)

app/src/ssh_manager/
├── candidates.rs                    (NEW — candidate-list view model)
├── candidates_tests.rs              (NEW — pure `rows()` unit tests)
├── mod.rs                           (modified — `pub mod candidates;`)
├── panel.rs                         (modified — render Candidates section)
└── server_view.rs                   (unchanged)

app/i18n/{en,ja,zh-CN}/warp.ftl      (modified — new strings)
```

No database migration. The `ssh_servers` SQLite schema is untouched.

## 2. Parser

### 2.1 Public API

In `crates/warp_ssh_manager/src/ssh_config_parser.rs`:

```rust
use std::path::PathBuf;

/// A single importable host parsed from `~/.ssh/config`.
///
/// Holds only the five fields the importer cares about (see PRODUCT.md
/// decision I/J/K). The original `Host` alias is preserved in `alias`
/// and used as `SshServerInfo::host` on import so that OpenSSH alias
/// semantics keep working when the saved entry is later launched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SshConfigCandidate {
    pub alias: String,
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<PathBuf>,
}

/// Resolve the default `~/.ssh/config` path for the current user.
/// Returns `None` if the home directory cannot be determined.
pub fn default_ssh_config_path() -> Option<PathBuf>;

/// Parse the body of an ssh_config file into candidate hosts.
///
/// Pure function over the file contents — no IO. Tests should call
/// this directly with literal strings.
///
/// Behavior is specified by PRODUCT.md decisions F/G/H/I/J/K/L.
/// In particular:
/// - `Host *`, `Host pattern-*`, `Host !foo` blocks are skipped.
/// - `Match` blocks are dropped until the next `Host` or EOF.
/// - `Include` directives produce a `log::warn!` and are otherwise
///   ignored.
/// - `Port` values that do not parse as `u16` produce `None`, not 22.
/// - A `Host a b c` block produces three candidates with shared body.
pub fn parse_ssh_config(content: &str) -> Vec<SshConfigCandidate>;

/// Top-level loader for the SSH Manager UI.
///
/// Returns the resolved path it attempted to read (for UI display) and
/// the parse result. The path is returned even on error so the UI can
/// show it in the "not found" / "parse error" message.
pub fn load_candidates() -> LoadResult;

pub struct LoadResult {
    pub path: Option<PathBuf>,
    pub outcome: LoadOutcome,
}

pub enum LoadOutcome {
    /// File parsed successfully.
    Loaded(Vec<SshConfigCandidate>),
    /// `path` did not exist. UI shows "No SSH config found at ...".
    NotFound,
    /// IO or parse error. UI shows the message verbatim in a red row.
    Error(String),
}
```

### 2.2 Parser implementation

Hand-written line-based scanner — no new crate dependency. The
`crates/warp_ssh_manager/Cargo.toml` already pulls in `regex`,
`anyhow`, `log`, `thiserror`, which are sufficient.

State machine:

```
state = OutsideHost
for each line in input:
  trimmed = strip leading whitespace, drop everything from first '#'
  if trimmed is empty: continue
  (keyword, value) = split on first whitespace (case-insensitive keyword)
  match keyword:
    "Host":
      flush current candidate(s) if any
      tokenize value as space-separated aliases
      filter aliases: drop any containing '*', '?', '!'
      if no aliases left: state = OutsideHost
      else: state = InsideHost(aliases, empty body)
    "Match":
      flush current candidate(s) if any
      state = InsideMatch  (consumes lines until next Host)
    "Include":
      log::warn!("Include not supported in MVP: {value}");
      continue (do not change state)
    other when state == InsideHost:
      apply field to current body (HostName / User / Port / IdentityFile)
    _:
      ignore
flush at EOF
```

Value parsing rules:

- **Quoted values** (`IdentityFile "C:\Users\Jiaqi Jiang\.ssh\id"`): if
  the trimmed value starts and ends with `"`, strip them. Otherwise pass
  through. (OpenSSH supports both.)
- **Tilde expansion in `IdentityFile`**: leading `~/` is replaced with
  the resolved home directory. Other `~user` forms are left as-is.
- **Port**: `u16::from_str(value).ok()` — `None` for any failure.
- **Repeated keys inside one Host block**: OpenSSH uses first-wins; we
  match that (ignore subsequent values for the same key in the same
  block).
- **Unknown keywords**: silently ignored.

### 2.3 Test cases

In `crates/warp_ssh_manager/src/ssh_config_parser.rs` under `#[cfg(test)]`:

| # | Test | Input shape | Asserts |
|---|---|---|---|
| 1 | `empty_file_produces_no_candidates` | `""` | `vec![]` |
| 2 | `comments_only_produces_no_candidates` | `"# comment\n# another"` | `vec![]` |
| 3 | `single_host_with_all_fields` | full block | exact `SshConfigCandidate` |
| 4 | `host_with_only_alias_uses_alias_as_implicit_hostname` | `Host foo\n` | `hostname == None`, importer maps to `server.host = "foo"` |
| 5 | `multiple_hosts` | 3 blocks | 3 candidates in order |
| 6 | `wildcard_host_skipped` | `Host *.prod\n  User x\n` | `vec![]` |
| 7 | `negation_host_skipped` | `Host !bad\n` | `vec![]` |
| 8 | `host_with_question_mark_skipped` | `Host a?\n` | `vec![]` |
| 9 | `match_block_ignored_until_next_host` | `Host a\n...\nMatch user x\n  User y\nHost b\n  User z` | only `a` and `b`, `b.user == Some("z")` |
| 10 | `match_block_at_eof_does_not_panic` | ends mid-Match | parses without panic |
| 11 | `include_directive_warns_and_continues` | `Include other.conf\nHost a\n` | candidate for `a` exists, log captured |
| 12 | `port_invalid_yields_none` | `Host a\n  Port not-a-number\n` | `port == None` |
| 13 | `port_out_of_range_yields_none` | `Host a\n  Port 70000\n` | `port == None` |
| 14 | `port_valid_yields_some` | `Host a\n  Port 2222\n` | `port == Some(2222)` |
| 15 | `quoted_identity_file_unquoted` | `Host a\n  IdentityFile "C:\Users\foo\\bar"` | path has no surrounding quotes |
| 16 | `tilde_in_identity_file_expanded` | `Host a\n  IdentityFile ~/x` | path starts with home dir |
| 17 | `case_insensitive_keywords` | `host a\n  hostname x\n  port 22` | parses identically |
| 18 | `host_with_multiple_aliases` | `Host a b c\n  Port 22` | 3 candidates `a`, `b`, `c`, all `port==Some(22)` |
| 19 | `repeated_field_first_wins` | `Host a\n  Port 1\n  Port 2` | `port == Some(1)` |
| 20 | `inline_trailing_comment_dropped` | `Host a # alias` | alias is `a`, not `a # alias` |
| 21 | `leading_indent_tolerated` | `  Host a\n  Port 22` | parses |

Tests 1–21 run as `cargo test -p warp_ssh_manager`. No fs, no env, no
`tokio`. All inputs are literal `&str`.

### 2.4 `load_candidates` IO

```rust
pub fn load_candidates() -> LoadResult {
    let path = match default_ssh_config_path() {
        Some(p) => p,
        None => {
            return LoadResult {
                path: None,
                outcome: LoadOutcome::Error("Could not determine home directory".into()),
            };
        }
    };

    let outcome = match std::fs::read_to_string(&path) {
        Ok(s) => LoadOutcome::Loaded(parse_ssh_config(&s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => LoadOutcome::NotFound,
        Err(e) => LoadOutcome::Error(format!("{e}")),
    };

    LoadResult { path: Some(path), outcome }
}
```

`default_ssh_config_path` uses `dirs::home_dir()` — already a workspace
member, no Cargo.toml change needed. (Verify with `grep -rn "^dirs"
crates/warp_ssh_manager/Cargo.toml` before assuming.)

## 3. UI integration (`app/src/ssh_manager/`)

### 3.1 View model

New file `app/src/ssh_manager/candidates.rs`. The view model has no UI
of its own and is held as a `ModelHandle<CandidatesViewModel>` (not
`ViewHandle`) — same pattern as `SshTreeChangedNotifier`.

```rust
pub struct CandidatesViewModel {
    /// Result of the most recent load attempt. None = not yet loaded.
    state: Option<LoadResult>,
    /// Cached set of `host` strings already present in the saved tree,
    /// used to mark candidates as "Added". Rebuilt on tree change.
    added_aliases: HashSet<String>,
    expanded: bool,
}

impl CandidatesViewModel {
    pub fn refresh(&mut self, ctx: &mut ModelContext<Self>);
    /// Takes an iterator of saved-tree `host` strings — easier to mock
    /// in tests than passing a `&SshRepository` (no SQLite needed).
    pub fn on_tree_changed(&mut self, hosts: impl IntoIterator<Item = String>);
    pub fn rows(&self) -> Vec<CandidateRow>;
}

pub enum CandidateRow {
    Header { path_display: String, count: usize, can_refresh: bool },
    NotFound { path_display: String },
    Empty { path_display: String },
    Error { path_display: String, message: String },
    Candidate { alias: String, hostname: Option<String>, user: Option<String>, port: Option<u16>, identity_file: Option<String>, added: bool },
}
```

The view model owns no IO context — `refresh` calls
`warp_ssh_manager::ssh_config_parser::load_candidates()` synchronously
on the calling thread. Config files are typically <10 KB; sync IO is
acceptable and matches the existing `SshRepository` pattern (which also
does sync SQLite calls from the UI thread).

### 3.2 Panel changes

In `app/src/ssh_manager/panel.rs` (currently ~1000 lines):

1. Add `candidates: ModelHandle<CandidatesViewModel>` field on
   `SshManagerPanel`.
2. In `SshManagerPanel::new`, create the view model and trigger an
   initial `refresh`.
3. In `view()`, render the Candidates section above the existing tree.
   Use the same row-rendering primitives as the current tree so visuals
   stay consistent.
4. Subscribe to the existing `SshTreeChangedNotifier` so
   `on_tree_changed` reruns when the saved tree changes — this is what
   flips a row from `+` to "Added" after the user clicks Add.
5. New actions:
   - `SshManagerPanelAction::ImportCandidate(alias)` — looks the
     candidate up by alias, calls `SshRepository::add_server` with
     fields mapped per PRODUCT.md decisions I/J/K, emits the existing
     `SshManagerPanelEvent::OpenServerEditor` for consistency with
     manual "Add server".
   - `SshManagerPanelAction::RefreshCandidates` — calls
     `candidates.refresh()`.
   - `SshManagerPanelAction::ToggleCandidatesSection` — flips
     `expanded`.

No new context-menu items in this MVP — the section header has explicit
Refresh + collapse affordances, which is enough.

### 3.3 Importer mapping

The actual `SshRepository` API is `create_server(conn, parent, name, info)`
(not `add_server` as an earlier draft of this spec said). Use that —
it's the same code path the manual "New server" button uses, and it
handles duplicate-name dedup via `unique_name(conn, parent, base)`.
First import is `alias`, second is `alias 2`, etc.

```rust
fn candidate_to_server(c: &SshConfigCandidate, path_display: &str) -> SshServerInfo {
    let auth_type = if c.identity_file.is_some() {
        AuthType::Key
    } else {
        AuthType::Password
    };
    // PRODUCT.md decision I: store the ALIAS as `host`, so OpenSSH
    // `~/.ssh/config` directives still apply when the saved entry is
    // launched (not the resolved `HostName`).
    let host = c.alias.clone();
    SshServerInfo {
        node_id: uuid::Uuid::new_v4().to_string(),
        host,
        port: c.port.unwrap_or(22),
        username: c.user.clone().unwrap_or_default(),
        auth_type,
        key_path: c.identity_file.as_ref().map(|p| p.to_string_lossy().into_owned()),
        startup_command: None,
        notes: Some(format!("Imported from {path_display}")),
        last_connected_at: None,
    }
}
```

The `notes` line is informational only — it lets the user see in the
server detail view where the entry came from, without introducing a
formal "source" field in the schema.

## 4. i18n strings

Add to `app/i18n/en/warp.ftl` (and mirror in `app/i18n/ja/warp.ftl` and
`app/i18n/zh-CN/warp.ftl`; Chinese is direct, Japanese is
machine-translation-quality with `# TODO: review` markers above each
key):

```
workspace-left-panel-ssh-manager-candidates-header = From { $path }
workspace-left-panel-ssh-manager-candidates-empty = No importable hosts in { $path }
workspace-left-panel-ssh-manager-candidates-not-found = No SSH config found at { $path }
workspace-left-panel-ssh-manager-candidates-error = Could not read SSH config at { $path }: { $error }
workspace-left-panel-ssh-manager-candidates-add = Add to SSH Manager
workspace-left-panel-ssh-manager-candidates-added = Added
workspace-left-panel-ssh-manager-candidates-refresh = Refresh from ~/.ssh/config
```

## 5. PR split

### PR 1 — Parser + IO (`warp_ssh_manager` only)

Scope: `crates/warp_ssh_manager/src/ssh_config_parser.rs` + 21 unit
tests + `lib.rs` re-exports. No `app/` changes.

This PR is independently mergeable and reviewable. It adds a pure
function + a tiny IO wrapper with full test coverage. Approx 400 lines
including tests.

### PR 2 — UI integration (`app/src/ssh_manager/` + i18n)

Scope: `candidates.rs`, `panel.rs` modifications, i18n keys.
Depends on PR 1. Approx 500 lines.

This PR includes the screenshot from issue #110 in the description and
sets `Closes #110`.

## 6. Out of scope (per PRODUCT.md non-goals, explicit reminder)

- No fs watcher.
- No write-back to `~/.ssh/config`.
- No `Include` recursion.
- No `Match` evaluation.
- No `ProxyJump`, `ProxyCommand`, `ForwardAgent`, etc.
- No multi-config-path setting.

## 7. Risks

| Risk | Mitigation |
|---|---|
| Parser silently misinterprets an edge-case config and creates a wrong candidate | 21 unit tests covering each PRODUCT.md decision; user can always delete a wrongly-imported entry. |
| Sync IO blocks the UI thread on a slow disk | Config files are <10 KB and read once on panel open + manual refresh. Matches existing `SshRepository` pattern. If observed as a problem, easy follow-up to move to `async-fs`. |
| Future change to the SQLite schema for `SshServerInfo` breaks the importer | Importer goes through `SshRepository::add_server`, the same code path manual creation uses. Schema changes affect both equally. |
| Conflict with another upstream "import" PR | Verified at spec time: no open PR touches `crates/warp_ssh_manager`. Issue #110 has no PR linked. |
