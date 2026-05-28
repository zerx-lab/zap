//! `~/.ssh/config` → `SshConfigCandidate` 解析器与一次性加载器。
//!
//! 设计与边界见 `specs/gh-110-ssh-config-import/{PRODUCT,TECH}.md`(对应 GitHub
//! issue #110):只支持 5 个字段(`Host` / `HostName` / `User` / `Port` /
//! `IdentityFile`),跳过通配符 / 否定 `Host`、忽略 `Match` 块、`Include` 仅
//! warn 不递归、`Port` 非法返 `None` 而不是静默填 22。
//!
//! 解析器是纯函数(`parse_ssh_config(&str) -> Vec<_>`),不碰 IO、env、tokio,
//! 单元测试用字面量驱动。`load_candidates()` 是顶层 IO 包装,返回的
//! `LoadResult` 把"路径"和"结果"分开,让 UI 在 NotFound / Error 情况也能告诉
//! 用户实际尝试读的是哪个路径。

use std::path::PathBuf;

/// 一条可导入候选,来自 `~/.ssh/config` 中一个有效的 `Host` 块。
///
/// 字段是 OpenSSH `ssh_config` 的子集 —— PRODUCT.md decision I/J/K 选定
/// 的最小集。`alias` 是 `Host` 行上的字面别名,导入到 `SshServerInfo`
/// 时作为 `host` 字段使用,这样后续从 Zap 启动 `ssh` 时 OpenSSH 仍能
/// 应用 `~/.ssh/config` 里这个别名对应的高级指令(`ProxyJump` 等)。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SshConfigCandidate {
    pub alias: String,
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<PathBuf>,
}

/// 解析 `ssh_config` 文件正文,返回有序的候选列表。
///
/// 顺序按文件中 `Host` 块出现的顺序;`Host a b c` 一行展开成 3 条
/// 共享 body 的候选。具体边界规则见 `PRODUCT.md` 第 4 节(F-L)。
pub fn parse_ssh_config(content: &str) -> Vec<SshConfigCandidate> {
    let mut out = Vec::new();
    let mut state = ParseState::Outside;

    for line in content.lines() {
        // 行内 `#` 之后一律视为注释截断。OpenSSH 实际语义对引号外/内的 `#`
        // 处理有边角差异,但 PRODUCT.md decision 范围内的 5 个字段都不会
        // 在合理输入中含 `#`,naive 截断对用户预期是对的。
        let no_comment = match line.find('#') {
            Some(idx) => &line[..idx],
            None => line,
        };
        let trimmed = no_comment.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let keyword = parts.next().unwrap_or("");
        let value = parts.next().unwrap_or("").trim();

        if keyword.eq_ignore_ascii_case("Host") {
            flush(&mut state, &mut out);
            let aliases = parse_host_aliases(value);
            state = if aliases.is_empty() {
                // 整行都是 wildcard / 否定模式 —— 不开新块,但要"消费"后续
                // 字段行,免得它们漏到下一个有效 Host。InMatch 状态正好就是
                // "丢弃直到下一个 Host"的语义,这里复用。
                ParseState::InMatch
            } else {
                ParseState::InHost {
                    aliases,
                    body: BodyFields::default(),
                }
            };
        } else if keyword.eq_ignore_ascii_case("Match") {
            // PRODUCT.md decision H:Match 块整段忽略,与"全 wildcard Host"
            // 走同一条 InMatch 路径。
            flush(&mut state, &mut out);
            state = ParseState::InMatch;
        } else if keyword.eq_ignore_ascii_case("Include") {
            // PRODUCT.md decision F:MVP 不递归,只 warn。状态不变,后续
            // 行仍归属当前 Host 块(若有)—— 这与 OpenSSH 的 Include 语义
            // 一致(Include 不结束当前 Host 上下文)。
            log::warn!(
                "Include directive in ssh_config is not supported by importer; \
                 hosts in `{value}` will not be imported"
            );
        } else if let ParseState::InHost { body, .. } = &mut state {
            apply_body_field(body, keyword, value);
        }
        // InMatch / Outside 下的其他 keyword:忽略。
    }

    flush(&mut state, &mut out);
    out
}

// ---------------------------------------------------------------------------
// 内部辅助
// ---------------------------------------------------------------------------

enum ParseState {
    /// 还没遇到任何 Host / Match。
    Outside,
    /// 当前在一个有效 Host 块内。`aliases` 是去掉 wildcard 后剩下的别名。
    InHost {
        aliases: Vec<String>,
        body: BodyFields,
    },
    /// 当前在被忽略的块内(`Match` 或全 wildcard 的 `Host`),消费字段直到
    /// 下一个 `Host` 或 EOF。
    InMatch,
}

#[derive(Default, Clone)]
struct BodyFields {
    hostname: Option<String>,
    user: Option<String>,
    port: Option<u16>,
    identity_file: Option<PathBuf>,
}

fn flush(state: &mut ParseState, out: &mut Vec<SshConfigCandidate>) {
    let prev = std::mem::replace(state, ParseState::Outside);
    if let ParseState::InHost { aliases, body } = prev {
        for alias in aliases {
            out.push(SshConfigCandidate {
                alias,
                hostname: body.hostname.clone(),
                user: body.user.clone(),
                port: body.port,
                identity_file: body.identity_file.clone(),
            });
        }
    }
}

/// 把 `Host a *.prod b !bad` 这样的行解析成 `["a", "b"]`。
fn parse_host_aliases(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .filter(|tok| !tok.contains('*') && !tok.contains('?') && !tok.contains('!'))
        .map(|s| s.to_string())
        .collect()
}

/// 在当前 Host 块的 body 上应用一个字段。**首次出现胜出**(匹配 OpenSSH 语义)。
fn apply_body_field(body: &mut BodyFields, keyword: &str, value: &str) {
    if keyword.eq_ignore_ascii_case("HostName") {
        if body.hostname.is_none() {
            body.hostname = Some(value.to_string());
        }
    } else if keyword.eq_ignore_ascii_case("User") {
        if body.user.is_none() {
            body.user = Some(value.to_string());
        }
    } else if keyword.eq_ignore_ascii_case("Port") {
        // 注意:首次"声明"胜出,而不是首次"有效"胜出 —— 但因为 Port 解析
        // 失败时我们填 None(PRODUCT.md decision K),first-wins 的"已声明"
        // 状态在这里和"值非 None"等价。简单起见用 is_none 守卫。
        if body.port.is_none() {
            body.port = value.parse::<u16>().ok();
        }
    } else if keyword.eq_ignore_ascii_case("IdentityFile") && body.identity_file.is_none() {
        let unquoted = strip_surrounding_quotes(value);
        body.identity_file = Some(expand_tilde(unquoted));
    }
    // 其余 keyword:忽略(MVP 只支持 5 个字段)。
}

fn strip_surrounding_quotes(s: &str) -> &str {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn expand_tilde(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }
    PathBuf::from(s)
}

/// 当前用户的默认 `~/.ssh/config` 路径,跨平台。
///
/// 找不到 home 目录(罕见)时返回 `None`。
pub fn default_ssh_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".ssh").join("config"))
}

/// 解析结果及其来源路径,供 UI 用于错误/空状态展示。
#[derive(Debug)]
pub struct LoadResult {
    /// 实际尝试读取的路径。`None` 表示连 home 目录都拿不到。
    pub path: Option<PathBuf>,
    pub outcome: LoadOutcome,
}

#[derive(Debug)]
pub enum LoadOutcome {
    /// 文件成功读取并解析(可能列表为空)。
    Loaded(Vec<SshConfigCandidate>),
    /// 路径不存在 —— 干净状态,UI 显示"未找到"提示而不是 error。
    NotFound,
    /// IO 错误(权限、编码、磁盘 etc.)。`String` 是给用户看的可读消息。
    Error(String),
}

/// 一次性加载默认路径的 `~/.ssh/config`,返回路径 + 结果。
///
/// 设计为同步 + 无 panic:UI 在面板首次打开时调一次,典型 config <10KB,
/// 同步 IO 足够快。fs 读不存在 / 权限失败时分别走 `NotFound` / `Error`,
/// 不向上抛错。
pub fn load_candidates() -> LoadResult {
    match default_ssh_config_path() {
        Some(p) => load_candidates_from(&p),
        None => LoadResult {
            path: None,
            outcome: LoadOutcome::Error("Could not determine home directory".into()),
        },
    }
}

/// 同 [`load_candidates`],但允许调用方显式指定路径 —— 主要给单元测试用
/// (tempfile),也为未来"自定义 config 路径"设置项留接口。
pub fn load_candidates_from(path: &std::path::Path) -> LoadResult {
    let outcome = match std::fs::read_to_string(path) {
        Ok(s) => LoadOutcome::Loaded(parse_ssh_config(&s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => LoadOutcome::NotFound,
        Err(e) => LoadOutcome::Error(format!("{e}")),
    };
    LoadResult {
        path: Some(path.to_path_buf()),
        outcome,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试用快捷构造器,默认全 `None`,只填用例关心的字段。
    fn cand(alias: &str) -> SshConfigCandidate {
        SshConfigCandidate {
            alias: alias.into(),
            hostname: None,
            user: None,
            port: None,
            identity_file: None,
        }
    }

    /// 最朴素的快乐路径:一个带全部 5 个字段的 Host 块,产出一条候选。
    /// 这个测试驱动出最小的"Host 块识别 + 字段解析"主线;后续 case 都在
    /// 它的基础上加状态机分支。
    #[test]
    fn single_host_with_all_fields() {
        let input = "\
Host prodbox
    HostName prod.example.com
    User alice
    Port 2222
    IdentityFile /home/alice/.ssh/id_ed25519
";
        let got = parse_ssh_config(input);
        assert_eq!(
            got,
            vec![SshConfigCandidate {
                alias: "prodbox".into(),
                hostname: Some("prod.example.com".into()),
                user: Some("alice".into()),
                port: Some(2222),
                identity_file: Some(PathBuf::from("/home/alice/.ssh/id_ed25519")),
            }]
        );
    }

    #[test]
    fn empty_file_produces_no_candidates() {
        assert_eq!(parse_ssh_config(""), vec![]);
    }

    #[test]
    fn comments_only_produces_no_candidates() {
        assert_eq!(parse_ssh_config("# top comment\n# another\n"), vec![]);
    }

    #[test]
    fn host_with_only_alias_has_no_hostname_field() {
        // Importer 层(不在本模块)把 `alias` 当 `server.host` 用,这里只保证
        // parser 不臆造 hostname。
        assert_eq!(parse_ssh_config("Host foo\n"), vec![cand("foo")]);
    }

    #[test]
    fn multiple_hosts_in_order() {
        let input = "\
Host a
    User x
Host b
    User y
Host c
    User z
";
        let got = parse_ssh_config(input);
        let users: Vec<_> = got
            .iter()
            .map(|c| (c.alias.as_str(), c.user.as_deref()))
            .collect();
        assert_eq!(
            users,
            vec![("a", Some("x")), ("b", Some("y")), ("c", Some("z"))]
        );
    }

    #[test]
    fn wildcard_star_host_skipped() {
        // PRODUCT.md decision G:`Host *.prod` 是模板而非机器,不进候选区。
        let input = "\
Host *.prod
    User root
Host realbox
    User me
";
        let got = parse_ssh_config(input);
        assert_eq!(
            got,
            vec![SshConfigCandidate {
                user: Some("me".into()),
                ..cand("realbox")
            }]
        );
    }

    #[test]
    fn wildcard_question_host_skipped() {
        let input = "\
Host srv?
    User x
";
        assert_eq!(parse_ssh_config(input), vec![]);
    }

    #[test]
    fn negation_host_skipped() {
        let input = "\
Host !bad
    User x
";
        assert_eq!(parse_ssh_config(input), vec![]);
    }

    #[test]
    fn host_with_multiple_aliases_expands_to_separate_candidates() {
        // PRODUCT.md decision L:`Host a b c` 共享 body。
        let input = "\
Host a b c
    Port 22
    User shared
";
        let got = parse_ssh_config(input);
        assert_eq!(got.len(), 3);
        for (i, alias) in ["a", "b", "c"].iter().enumerate() {
            assert_eq!(got[i].alias, *alias);
            assert_eq!(got[i].port, Some(22));
            assert_eq!(got[i].user.as_deref(), Some("shared"));
        }
    }

    #[test]
    fn host_with_mixed_aliases_filters_wildcards_keeps_literals() {
        // `Host a *.prod b` → 只导出 a 和 b。
        let input = "\
Host a *.prod b
    User shared
";
        let got = parse_ssh_config(input);
        let aliases: Vec<&str> = got.iter().map(|c| c.alias.as_str()).collect();
        assert_eq!(aliases, vec!["a", "b"]);
    }

    #[test]
    fn match_block_ignored_until_next_host() {
        // PRODUCT.md decision H:`Match` 块整段忽略,不应"污染"前一个 Host
        // 的 body,也不应当成新候选。
        let input = "\
Host a
    User u_a
Match user someone
    User SHOULD_NOT_APPEAR
    Port 9999
Host b
    User u_b
";
        let got = parse_ssh_config(input);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].alias, "a");
        assert_eq!(got[0].user.as_deref(), Some("u_a"));
        assert_eq!(got[0].port, None, "Match 块的 Port 9999 不应漏到 a");
        assert_eq!(got[1].alias, "b");
        assert_eq!(got[1].user.as_deref(), Some("u_b"));
    }

    #[test]
    fn match_block_at_eof_does_not_panic() {
        let input = "\
Host a
    User u
Match user x
    User leak
";
        let got = parse_ssh_config(input);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].alias, "a");
        assert_eq!(got[0].user.as_deref(), Some("u"));
    }

    #[test]
    fn include_directive_logged_and_skipped_outside_host() {
        // PRODUCT.md decision F:`Include` 不递归,只 warn,后续解析照旧。
        let input = "\
Include ~/.ssh/work/*.conf
Host a
    User u
";
        let got = parse_ssh_config(input);
        assert_eq!(
            got,
            vec![SshConfigCandidate {
                user: Some("u".into()),
                ..cand("a")
            }]
        );
    }

    #[test]
    fn port_invalid_string_yields_none() {
        // PRODUCT.md decision K:不静默回退 22,UI 把空 port 显示给用户看。
        let input = "Host a\n    Port not-a-number\n";
        assert_eq!(parse_ssh_config(input)[0].port, None);
    }

    #[test]
    fn port_out_of_u16_range_yields_none() {
        let input = "Host a\n    Port 70000\n";
        assert_eq!(parse_ssh_config(input)[0].port, None);
    }

    #[test]
    fn port_valid_yields_some() {
        let input = "Host a\n    Port 2222\n";
        assert_eq!(parse_ssh_config(input)[0].port, Some(2222));
    }

    #[test]
    fn quoted_identity_file_has_quotes_stripped() {
        // OpenSSH 允许带空格路径用引号包裹。
        let input = "Host a\n    IdentityFile \"C:\\Users\\Jiaqi Jiang\\.ssh\\id\"\n";
        assert_eq!(
            parse_ssh_config(input)[0].identity_file,
            Some(PathBuf::from("C:\\Users\\Jiaqi Jiang\\.ssh\\id"))
        );
    }

    #[test]
    fn tilde_in_identity_file_expanded_to_home() {
        // ~/x 展开成 $HOME/x。$HOME 在不同 CI 环境不一样,只断言前缀是 home。
        let input = "Host a\n    IdentityFile ~/keys/id\n";
        let got = parse_ssh_config(input);
        let path = got[0].identity_file.as_ref().expect("IdentityFile set");
        let home = dirs::home_dir().expect("test runner has home dir");
        assert!(
            path.starts_with(&home),
            "expected {path:?} to start with {home:?}"
        );
        assert!(
            path.ends_with("keys/id"),
            "expected {path:?} to end with keys/id"
        );
    }

    #[test]
    fn case_insensitive_keywords() {
        let input = "host a\n    hOsTnAmE example.com\n    user alice\n    PORT 22\n";
        let got = parse_ssh_config(input);
        assert_eq!(
            got,
            vec![SshConfigCandidate {
                alias: "a".into(),
                hostname: Some("example.com".into()),
                user: Some("alice".into()),
                port: Some(22),
                identity_file: None,
            }]
        );
    }

    #[test]
    fn repeated_field_first_wins() {
        // 匹配 OpenSSH 语义:同一 Host 块内同一字段,首次出现的胜出。
        let input = "Host a\n    Port 1\n    Port 2\n    User first\n    User second\n";
        let got = parse_ssh_config(input);
        assert_eq!(got[0].port, Some(1));
        assert_eq!(got[0].user.as_deref(), Some("first"));
    }

    #[test]
    fn inline_trailing_comment_dropped_from_value() {
        // OpenSSH 实际上对行内 `#` 的处理边界比较模糊;我们走"保守"路线:
        // 整行扫描时遇到 `#` 截断,引号外有效。
        let input = "Host a # primary box\n    User alice # admin\n";
        let got = parse_ssh_config(input);
        assert_eq!(got[0].alias, "a");
        assert_eq!(got[0].user.as_deref(), Some("alice"));
    }

    #[test]
    fn leading_indent_tolerated() {
        // OpenSSH 允许任意前导空白。
        let input = "  Host a\n\t  Port 22\n";
        let got = parse_ssh_config(input);
        assert_eq!(got[0].alias, "a");
        assert_eq!(got[0].port, Some(22));
    }

    // -----------------------------------------------------------------
    // default_ssh_config_path / load_candidates_from / load_candidates
    // -----------------------------------------------------------------

    #[test]
    fn default_path_points_under_home_dot_ssh_config() {
        // 跨平台:只要 dirs::home_dir() 拿得到值,结果就应该是
        // `<home>/.ssh/config`。CI runner 始终有 HOME / USERPROFILE。
        let got = default_ssh_config_path().expect("test runner has home dir");
        let home = dirs::home_dir().expect("test runner has home dir");
        assert!(got.starts_with(&home), "{got:?} should start with {home:?}");
        assert!(got.ends_with("config"));
        assert!(
            got.to_string_lossy()
                .replace('\\', "/")
                .ends_with(".ssh/config"),
            "{got:?} should end with .ssh/config"
        );
    }

    #[test]
    fn load_candidates_from_nonexistent_path_returns_not_found() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let path = tmp.path().join("does_not_exist");
        let res = load_candidates_from(&path);
        assert_eq!(res.path.as_deref(), Some(path.as_path()));
        assert!(
            matches!(res.outcome, LoadOutcome::NotFound),
            "got {:?}",
            res.outcome
        );
    }

    #[test]
    fn load_candidates_from_valid_file_returns_parsed_candidates() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().expect("create tempfile");
        writeln!(tmp, "Host a\n    User u\n").expect("write tempfile");
        let res = load_candidates_from(tmp.path());
        match res.outcome {
            LoadOutcome::Loaded(v) => {
                assert_eq!(v.len(), 1);
                assert_eq!(v[0].alias, "a");
                assert_eq!(v[0].user.as_deref(), Some("u"));
            }
            other => panic!("expected Loaded, got {other:?}"),
        }
    }

    #[test]
    fn load_candidates_from_empty_file_returns_loaded_empty() {
        let tmp = tempfile::NamedTempFile::new().expect("create tempfile");
        let res = load_candidates_from(tmp.path());
        match res.outcome {
            LoadOutcome::Loaded(v) => assert!(v.is_empty()),
            other => panic!("expected Loaded(empty), got {other:?}"),
        }
    }
}
