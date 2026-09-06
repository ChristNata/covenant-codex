use super::MultitoolCli;
use super::Subcommand;
use clap::ColorChoice;
use clap::CommandFactory;
use clap::Parser;
use pretty_assertions::assert_eq;

const THREAD_ID: &str = "6a5dabf4-21c7-4a20-9c9f-27e2a2031a44";

#[derive(Debug, PartialEq)]
struct CommandRow {
    name: String,
    aliases: Vec<String>,
    visible_aliases: Vec<String>,
    hidden: bool,
}

fn exec_surface() -> (Vec<CommandRow>, String) {
    let mut root = MultitoolCli::command()
        .color(ColorChoice::Never)
        .term_width(100);
    root.build();
    let exec = root.find_subcommand_mut("exec").expect("retained exec");
    let rows = exec
        .get_subcommands()
        .map(|command| CommandRow {
            name: command.get_name().to_owned(),
            aliases: command.get_all_aliases().map(str::to_owned).collect(),
            visible_aliases: command.get_visible_aliases().map(str::to_owned).collect(),
            hidden: command.is_hide_set(),
        })
        .collect();
    (rows, exec.render_usage().to_string())
}

fn parse_exec(arguments: &[&str]) -> Result<codex_exec::Cli, clap::Error> {
    let root = MultitoolCli::try_parse_from(
        ["codex", "exec"]
            .into_iter()
            .chain(arguments.iter().copied()),
    )?;
    assert_eq!(root.interactive.prompt, None);
    let Some(Subcommand::Exec(exec)) = root.subcommand else {
        panic!("expected actual retained exec payload");
    };
    Ok(exec)
}

#[derive(Debug, PartialEq)]
struct PromptObservation {
    prompt: Option<String>,
    model: Option<String>,
    json: bool,
    command_absent: bool,
}

fn prompt_observation(exec: codex_exec::Cli) -> PromptObservation {
    let model = exec.model.clone();
    PromptObservation {
        prompt: exec.prompt,
        model,
        json: exec.json,
        command_absent: exec.command.is_none(),
    }
}

fn assert_retained_operands() {
    let words = ["review", "resume", "fork", "review resume fork", "-"];
    let observed = words.map(|word| {
        prompt_observation(
            parse_exec(&["--json", "--model", "gpt-5.5", "--", word])
                .expect("delimited prompt operand"),
        )
    });
    let expected = words.map(|word| PromptObservation {
        prompt: Some(word.to_owned()),
        model: Some("gpt-5.5".to_owned()),
        json: true,
        command_absent: true,
    });
    assert_eq!(observed, expected);
    assert_eq!(
        prompt_observation(parse_exec(&["--json"]).expect("stdin prompt remains optional")),
        PromptObservation {
            prompt: None,
            model: None,
            json: true,
            command_absent: true,
        }
    );
}

#[cfg(feature = "covenant")]
#[test]
fn covenant_exec_cli_refuses_session_forms_and_removes_nested_catalog() {
    assert_retained_operands();
    let refused = [
        vec!["review", "--uncommitted"],
        vec!["resume", "--last"],
        vec!["fork", THREAD_ID],
    ]
    .map(|arguments| parse_exec(&arguments).err().map(|error| error.kind()));
    let (commands, usage) = exec_surface();
    assert_eq!(
        (commands, refused),
        (
            Vec::<CommandRow>::new(),
            [Some(clap::error::ErrorKind::UnknownArgument); 3]
        )
    );
    insta::assert_snapshot!(usage, @"Usage: codex exec [OPTIONS] [PROMPT]");
}

#[cfg(feature = "covenant")]
#[test]
fn covenant_exec_cli_bare_session_words_are_plain_prompt_operands() {
    assert_retained_operands();
    let words = ["review", "resume", "fork"];
    let observed = words.map(|word| {
        prompt_observation(parse_exec(&[word]).expect("word remains a valid exec prompt"))
    });
    let expected = words.map(|word| PromptObservation {
        prompt: Some(word.to_owned()),
        model: None,
        json: false,
        command_absent: true,
    });
    assert_eq!(observed, expected);
}

#[cfg(not(feature = "covenant"))]
#[test]
fn covenant_exec_cli_ordinary_preserves_session_payloads_and_prompt_operands() {
    assert_retained_operands();
    let review = parse_exec(&["review", "--uncommitted"]).expect("existing review grammar");
    let resume = parse_exec(&["resume", "--last"]).expect("existing resume grammar");
    let fork = parse_exec(&["fork", THREAD_ID]).expect("existing fork grammar");
    assert_eq!(
        (
            review.prompt.as_deref(),
            resume.prompt.as_deref(),
            fork.prompt.as_deref()
        ),
        (None, None, None)
    );
    let Some(codex_exec::Command::Review(review)) = review.command else {
        panic!("review must retain its existing typed payload");
    };
    let Some(codex_exec::Command::Resume(resume)) = resume.command else {
        panic!("resume must retain its existing typed payload");
    };
    let Some(codex_exec::Command::Fork(fork)) = fork.command else {
        panic!("fork must retain its existing typed payload");
    };
    assert_eq!(
        (
            review.uncommitted,
            review.base,
            review.commit,
            review.commit_title,
            review.prompt
        ),
        (true, None, None, None, None)
    );
    assert_eq!(
        (
            resume.session_id,
            resume.last,
            resume.all,
            resume.images,
            resume.prompt
        ),
        (None, true, false, Vec::new(), None)
    );
    assert_eq!(
        (fork.session_id, fork.images, fork.prompt),
        (THREAD_ID.to_owned(), Vec::new(), None)
    );
    let (commands, usage) = exec_surface();
    let expected = ["resume", "fork", "review", "help"].map(|name| CommandRow {
        name: name.to_owned(),
        aliases: Vec::new(),
        visible_aliases: Vec::new(),
        hidden: false,
    });
    assert_eq!(commands, expected);
    insta::assert_snapshot!(usage, @r"
    Usage: codex exec [OPTIONS] [PROMPT]
           codex exec [OPTIONS] <COMMAND> [ARGS]
    ");
}
