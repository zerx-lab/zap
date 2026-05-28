# gh-110: Import hosts from `~/.ssh/config` into SSH Manager

Tracks: [GitHub issue #110](https://github.com/zerx-lab/zap/issues/110).
Technical implementation details are in `TECH.md`.

## 1. Problem

Zap's SSH Manager (left panel, `Alt+5` / `Ctrl+5`) stores every server in its
own SQLite store (`crates/warp_ssh_manager`). It does not read, parse, or
import `~/.ssh/config` — verified by grep across `crates/warp_ssh_manager/`,
`app/src/ssh_manager/`, and `specs/`: zero references to ssh_config parsing.

Two consequences:

1. **Setup friction.** A developer with N hosts in `~/.ssh/config` has to
   re-enter every host/user/port/key by hand before the SSH Manager UI is
   useful to them.
2. **Inconsistency across SSH entry paths.** Zap has two SSH paths:
   - Path ① — typing `ssh prodbox` in any terminal tab triggers Warpification
     and `exec`s the system `ssh`, which transparently reads `~/.ssh/config`
     and resolves the `prodbox` alias.
   - Path ② — the SSH Manager panel completely ignores the same file.
   The same `~/.ssh/config` is the source of truth for path ① and invisible to
   path ②.

This spec adds a one-way **import** (not sync) from `~/.ssh/config` into the
SSH Manager.

## 2. Solution

In the SSH Manager panel, render a "Candidates" section above the user's
saved tree. It lazy-reads `~/.ssh/config` when the panel first opens (and on
manual refresh), parses `Host` blocks, and lists each parsed host as a
candidate row. Clicking the candidate's `+` button copies that host into the
saved tree as a normal `SshServerInfo` — from that moment the entry is
owned by Zap and is decoupled from the source file.

Behavior is **read-only against `~/.ssh/config`**. The user's hand-written
config file is never written to, watched, or moved.

## 3. User stories

1. As a developer who already maintains `~/.ssh/config`, I want Zap to show
   my hosts in the SSH Manager without re-entering them, so I can start using
   the panel immediately.
2. As a user who imported a host, I want to edit it in Zap freely without
   affecting `~/.ssh/config`, so my hand-written file stays exactly as I
   wrote it.
3. As a user who deletes a host from `~/.ssh/config`, I want my Zap-side
   imported copy to remain, so I don't lose work I've already done in Zap.
4. As a user who already imported a host, I want the candidate row to show
   "Added" so I don't import it twice by accident.
5. As a user whose `~/.ssh/config` does not exist, I want the SSH Manager to
   open normally (no error popup, no panic), so a fresh install is not
   broken.
6. As a user whose `~/.ssh/config` is malformed, I want a visible error in
   the Candidates header (not a panic, not a silent empty list), so I can
   fix my file.
7. As a user who adds a new host to `~/.ssh/config` in `vim`, I want to come
   back to Zap, click Refresh, and see the new host immediately.
8. As a user with `Host *` / `Host pattern-*` blocks, I want those wildcard
   templates to be skipped (not shown as importable hosts), because they're
   configuration templates, not machines.
9. As a user with a `Match` block in my config, I want it ignored cleanly
   (not crash the parser), so the rest of my hosts still show up.
10. As a user, I want the resolved path to my config file shown in the UI,
    so I'm not guessing which `.ssh/config` Zap is reading.

## 4. Functional decisions

| # | Decision | Choice | Rationale |
|---|---|---|---|
| A | When is the config read? | On first open of the SSH Manager panel; manual Refresh button to re-read. No fs watcher. | Avoids cold-start cost and cross-platform watcher complexity. |
| B | What is the relationship between an imported entry and its source line? | Fully decoupled copy. After import, the saved entry has no link back to the config file. | Eliminates an entire class of "which side wins" sync bugs. |
| C | What happens if the source host changes after import? | Nothing on the Zap side. Candidate row stays "Added"; the candidate is not re-evaluated against the saved entry. | Same reason as B — no implicit sync. |
| D | What happens if the source host is removed from the config? | Candidate row disappears. Saved entry is untouched. | Same reason as B. |
| E | What happens if the user clicks Add on an already-imported host? | Button is disabled with tooltip "Already added". | Prevents duplicate entries; no modal. |
| F | `Include` directive | MVP: not supported. Log a warning, do not recurse. | Recursive include + cycle detection is significant scope; revisit after MVP feedback. |
| G | `Host *`, `Host pattern-*`, `Host !foo` | Skipped (not shown as candidates). | Wildcard / negation patterns are templates, not machines. Importing them as servers is meaningless. |
| H | `Match` block | Entire block ignored until the next `Host` or EOF. Does not count as a parse error. | `Match` semantics depend on runtime env; out of scope for MVP. |
| I | Imported `server.host` value | Set to the `Host` alias (e.g. `prodbox`), not the resolved `HostName`. | Preserves OpenSSH alias semantics — when the saved entry is launched via `ssh`, the system `ssh` still applies `~/.ssh/config` directives like `ProxyJump` for that alias. Matches issue #110's design. |
| J | Auth type inference | `IdentityFile` present → `Key`. Otherwise → `Password`. | Best-effort default; the user can change it in the saved entry. |
| K | `Port` parsing | Valid u16 → set. Invalid / out-of-range → unset (UI shows blank). | Do not silently substitute `22`; the user knows their port is wrong and should see it. |
| L | Same Host block lists multiple aliases (`Host a b c`) | Produce 3 candidate rows sharing the same body. | Matches OpenSSH semantics — each alias is independently usable. |

## 5. UX details

| Scenario | Behavior |
|---|---|
| `~/.ssh/config` does not exist | Candidates section shows one line: "No SSH config found at: `<resolved-path>`". |
| Config exists, 0 importable hosts | "No importable hosts found in `<resolved-path>`". |
| Parse / IO error | One red error row at the top of the Candidates section, showing the error string and the resolved path. SSH Manager panel still opens normally. |
| Many candidates (50+) | The Candidates section is collapsible (default expanded) with an internal scroll area; it never pushes the saved tree below the fold. |
| Resolved path visibility | Section header reads "From `~/.ssh/config`" with the absolute path as a tooltip / second line on hover. |
| `Match` block | Lines between `Match` and the next `Host` (or EOF) are dropped silently. |
| `Host alias` with no `HostName` | Still shown; on import, `server.host = alias`. Consistent with OpenSSH falling back to alias-as-hostname. |
| Already-imported candidate | Row is dimmed, button replaced with an "Added" badge, click is a no-op. |

## 6. Non-goals (explicit)

The first version does **not** implement:

- Writing back to `~/.ssh/config` (one-way only).
- Filesystem watcher / auto-refresh on file change.
- `Include` directive (recursive parsing of nested config files).
- `Match` block evaluation (matched against runtime env).
- `%h`, `%p`, `%r`, `%u`, `~` percent / tilde expansion in `IdentityFile`
  beyond resolving `~` to home dir.
- `ProxyJump`, `ProxyCommand`, `ForwardAgent`, etc. — only the 5 fields in
  decision (I/J/K) are imported.
- `known_hosts` display.
- "Test connection" button (the SSH Manager's main Connect button is itself
  still WIP — see `app/i18n/en/warp.ftl:2689,2703`).
- Custom config path setting / multiple config paths.

Each of these is a defensible feature request, but adding any of them to the
MVP roughly doubles parser + UI complexity. They are deferred until the MVP
ships and we have real usage feedback.

## 7. Acceptance criteria

1. With a non-empty `~/.ssh/config` containing at least one plain `Host`
   block, opening the SSH Manager panel shows a Candidates section listing
   each importable host. The resolved config path is visible in the UI.
2. Clicking `+` on a candidate adds it to the saved tree as a normal
   `SshServerInfo` with `host = alias`, port / user / identity_file
   populated where present, auth type inferred per decision J.
3. After step 2, the same candidate row shows "Added" with the button
   disabled. Closing and re-opening the panel preserves this state.
4. Editing the saved entry afterward does not affect `~/.ssh/config`. A
   diff of the file before and after import + edit is empty.
5. Deleting the same host from `~/.ssh/config` and clicking Refresh removes
   it from the Candidates section. The saved entry remains untouched in
   the tree.
6. `Host *`, `Host pattern-*`, `Host !bad`, and `Match` blocks are all
   absent from the Candidates section. The parser does not panic.
7. `Port abc` produces a candidate with no port (not port 22, not a panic).
8. With no `~/.ssh/config` file, the SSH Manager panel opens, Candidates
   section shows the "not found" message, saved tree behaves normally.
9. With a `~/.ssh/config` containing an `Include other.conf` line, the
   parser does not crash. The Include is logged and the rest of the file
   continues to parse.
10. With a malformed file (e.g. binary garbage), the SSH Manager panel
    opens, Candidates section shows an error row with the parse error and
    the path. No panic, no toast spam.

## 8. Diff from issue #110

Issue #110 is the original feature request. This spec aligns with it on the
main shape (candidate area + per-host Add button + read-only) and deviates
on three small points, justified above:

- **Decision K** (invalid `Port`): #110 says "fallback to 22"; this spec
  says "leave unset". Reason: do not silently substitute a wrong value.
- **Decisions B/C/D** (post-import relationship): #110 does not address
  what happens after import when the source file changes. This spec
  explicitly states "fully decoupled copy" to nail down behavior.
- **Decision E** (re-add): #110 does not address it. This spec specifies
  "disabled button + Added badge".

These deltas will be summarized in the PR description and posted as a
comment on #110 when PR 1 opens.
