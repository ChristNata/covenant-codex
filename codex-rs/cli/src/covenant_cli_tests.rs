use super::LoginSubcommand;
use super::MultitoolCli;
use super::Subcommand;
use clap::ColorChoice;
use clap::Command;
use clap::CommandFactory;
use clap::Parser;
use clap::error::ErrorKind;
use pretty_assertions::assert_eq;

#[derive(Debug, PartialEq)]
struct CommandRow {
    name: String,
    aliases: Vec<String>,
    visible_aliases: Vec<String>,
    hidden: bool,
}

#[derive(Debug, PartialEq)]
struct Surface {
    root: Vec<CommandRow>,
    help_targets: Vec<CommandRow>,
}

fn command_rows(command: &Command) -> Vec<CommandRow> {
    command
        .get_subcommands()
        .map(|child| CommandRow {
            name: child.get_name().to_owned(),
            aliases: child.get_all_aliases().map(str::to_owned).collect(),
            visible_aliases: child.get_visible_aliases().map(str::to_owned).collect(),
            hidden: child.is_hide_set(),
        })
        .collect()
}

fn built_command() -> Command {
    let mut command = MultitoolCli::command()
        .color(ColorChoice::Never)
        .term_width(100);
    command.build();
    command
}

fn surface(command: &Command) -> Surface {
    Surface {
        root: command_rows(command),
        help_targets: command_rows(command.find_subcommand("help").expect("generated help")),
    }
}

fn assert_retained_parses() {
    let exec = MultitoolCli::try_parse_from([
        "codex",
        "--strict-config",
        "exec",
        "--json",
        "plugin exec words stay in this prompt",
    ])
    .expect("canonical exec payload");
    let Some(Subcommand::Exec(payload)) = exec.subcommand else {
        panic!("canonical exec must reach its existing payload");
    };
    let login = MultitoolCli::try_parse_from(["codex", "login"]).expect("canonical login");
    let Some(Subcommand::Login(login_payload)) = login.subcommand else {
        panic!("canonical login must reach its existing payload");
    };
    let status =
        MultitoolCli::try_parse_from(["codex", "login", "status"]).expect("existing login status");
    let Some(Subcommand::Login(status_payload)) = status.subcommand else {
        panic!("login status must reach its existing payload");
    };
    let logout = MultitoolCli::try_parse_from(["codex", "logout"]).expect("canonical logout");
    let Some(Subcommand::Logout(logout_payload)) = logout.subcommand else {
        panic!("canonical logout must reach its existing payload");
    };
    assert_eq!(
        (
            [
                exec.interactive.prompt,
                login.interactive.prompt,
                status.interactive.prompt,
                logout.interactive.prompt,
            ],
            (
                exec.interactive.strict_config,
                payload.prompt.as_deref(),
                payload.json,
                payload.command.is_none(),
            ),
            (
                login_payload.action.is_none(),
                login_payload.with_api_key,
                login_payload.with_access_token,
                login_payload.use_device_code,
                matches!(status_payload.action, Some(LoginSubcommand::Status)),
            ),
            logout_payload.config_overrides.raw_overrides,
        ),
        (
            [None, None, None, None],
            (
                true,
                Some("plugin exec words stay in this prompt"),
                true,
                true
            ),
            (true, false, false, false, true),
            Vec::<String>::new(),
        )
    );
    let displays = [
        vec!["codex", "--help"],
        vec!["codex", "help"],
        vec!["codex", "help", "exec"],
        vec!["codex", "help", "login", "status"],
        vec!["codex", "--version"],
    ]
    .map(|argv| {
        MultitoolCli::try_parse_from(argv)
            .expect_err("display action must not start a command")
            .kind()
    });
    assert_eq!(
        displays,
        [
            ErrorKind::DisplayHelp,
            ErrorKind::DisplayHelp,
            ErrorKind::DisplayHelp,
            ErrorKind::DisplayHelp,
            ErrorKind::DisplayVersion,
        ]
    );
}

#[cfg(feature = "covenant")]
#[test]
fn covenant_cli_surface_has_only_canonical_commands_and_truthful_help() {
    assert_retained_parses();
    let mut command = built_command();
    let expected_rows = || {
        ["exec", "login", "logout", "help"]
            .map(|name| CommandRow {
                name: name.to_owned(),
                aliases: Vec::new(),
                visible_aliases: Vec::new(),
                hidden: false,
            })
            .into()
    };
    let actual = surface(&command);
    let long_help = command.render_long_help().to_string();
    // Snapshot the changed admission/help surface, leaving root option policy
    // outside this tranche. Normalize only indentation and blank lines.
    let intro_and_commands = long_help
        .split_once("\nOptions:\n")
        .expect("root options remain available")
        .0
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        actual,
        Surface {
            root: expected_rows(),
            help_targets: expected_rows(),
        }
    );
    insta::assert_snapshot!(intro_and_commands, @"
    Codex CLI
    A subcommand is required. Use exec, login, or logout.
    Usage: codex [OPTIONS] <COMMAND> [ARGS]
    Commands:
    exec Run Codex non-interactively
    login Manage login
    logout Remove stored authentication credentials
    help Print this message or the help of the given subcommand(s)
    ");
}

#[cfg(feature = "covenant")]
#[test]
fn covenant_cli_surface_rejects_excluded_spellings_and_root_prompt_prefixes() {
    assert_retained_parses();
    let excluded = [
        "agents",
        "e",
        "review",
        "mcp",
        "plugin",
        "mcp-server",
        "app-server",
        "remote-control",
        "app",
        "completion",
        "update",
        "doctor",
        "sandbox",
        "debug",
        "execpolicy",
        "apply",
        "a",
        "resume",
        "queue",
        "archive",
        "delete",
        "migrate-rollouts",
        "unarchive",
        "fork",
        "cloud",
        "cloud-tasks",
        "responses-api-proxy",
        "stdio-to-uds",
        "exec-server",
        "features",
    ];
    let mut cases = vec![
        vec!["codex"],
        vec!["codex", "a bare prompt"],
        vec!["codex", "--", "a bare prompt"],
    ];
    for spelling in excluded {
        cases.push(vec!["codex", spelling]);
        cases.push(vec!["codex", spelling, "--help"]);
        cases.push(vec!["codex", "help", spelling]);
    }
    for prefix in excluded.into_iter().chain(["a bare prompt"]) {
        for tail in [
            vec!["exec", "benign task"],
            vec!["login", "status"],
            vec!["logout"],
        ] {
            let mut plain = vec!["codex", prefix];
            plain.extend_from_slice(&tail);
            cases.push(plain);
            let mut with_option = vec!["codex", "--strict-config", prefix];
            with_option.extend_from_slice(&tail);
            cases.push(with_option);
        }
    }
    let unexpected = cases
        .into_iter()
        .filter_map(|argv| match MultitoolCli::try_parse_from(&argv) {
            Ok(_) => Some((argv, "admitted".to_owned())),
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
                ) =>
            {
                Some((argv, format!("displayed {:?}", error.kind())))
            }
            Err(_) => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(unexpected, Vec::<(Vec<&str>, String)>::new());
}

#[cfg(not(feature = "covenant"))]
#[test]
fn covenant_cli_surface_ordinary_preserves_parser_tree_and_prompt_behavior() {
    assert_retained_parses();
    let mut command = built_command();
    let mut names = vec![
        "agents",
        "exec",
        "review",
        "login",
        "logout",
        "mcp",
        "plugin",
        "mcp-server",
        "app-server",
        "remote-control",
        "completion",
        "update",
        "doctor",
        "sandbox",
        "debug",
        "execpolicy",
        "apply",
        "resume",
        "queue",
        "archive",
        "delete",
        "migrate-rollouts",
        "unarchive",
        "fork",
        "cloud",
        "responses-api-proxy",
        "stdio-to-uds",
        "exec-server",
        "features",
        "help",
    ];
    if cfg!(any(target_os = "macos", target_os = "windows")) {
        names.insert(/*index*/ 10, "app");
    }
    let root = names
        .iter()
        .map(|name| {
            let aliases = match *name {
                "exec" => vec!["e".to_owned()],
                "apply" => vec!["a".to_owned()],
                "cloud" => vec!["cloud-tasks".to_owned()],
                _ => Vec::new(),
            };
            CommandRow {
                name: (*name).to_owned(),
                visible_aliases: if *name == "cloud" {
                    Vec::new()
                } else {
                    aliases.clone()
                },
                aliases,
                hidden: matches!(*name, "execpolicy" | "responses-api-proxy" | "stdio-to-uds"),
            }
        })
        .collect::<Vec<_>>();
    let help_targets = root
        .iter()
        .map(|row| CommandRow {
            name: row.name.clone(),
            aliases: Vec::new(),
            visible_aliases: Vec::new(),
            hidden: row.hidden,
        })
        .collect();
    assert_eq!(surface(&command), Surface { root, help_targets });
    let empty = MultitoolCli::try_parse_from(["codex"]).expect("ordinary implicit TUI");
    let prompt =
        MultitoolCli::try_parse_from(["codex", "a bare prompt"]).expect("ordinary root prompt");
    let prefixed = MultitoolCli::try_parse_from(["codex", "a bare prompt", "exec", "task"])
        .expect("ordinary root prompt before command");
    let Some(Subcommand::Exec(prefixed_payload)) = prefixed.subcommand else {
        panic!("ordinary prefix must retain the real exec payload");
    };
    let alias = MultitoolCli::try_parse_from(["codex", "e", "--json", "task"])
        .expect("ordinary exec alias");
    let Some(Subcommand::Exec(alias_payload)) = alias.subcommand else {
        panic!("ordinary alias must retain the real exec payload");
    };
    let review = MultitoolCli::try_parse_from(["codex", "review"])
        .expect("ordinary excluded canonical command");
    assert_eq!(
        (
            (empty.subcommand.is_none(), empty.interactive.prompt),
            (
                prompt.subcommand.is_none(),
                prompt.interactive.prompt.as_deref()
            ),
            (
                prefixed.interactive.prompt.as_deref(),
                prefixed_payload.prompt.as_deref()
            ),
            (alias_payload.prompt.as_deref(), alias_payload.json),
            matches!(review.subcommand, Some(Subcommand::Review(_))),
        ),
        (
            (true, None),
            (true, Some("a bare prompt")),
            (Some("a bare prompt"), Some("task")),
            (Some("task"), true),
            true,
        )
    );
    let help = command.render_long_help().to_string();
    assert!(help.contains(
        "If no subcommand is specified, options will be forwarded to the interactive CLI."
    ));
    assert!(help.contains("codex [OPTIONS] [PROMPT]"));
}
