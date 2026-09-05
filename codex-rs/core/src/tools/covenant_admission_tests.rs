use super::*;
use crate::session::step_context::StepContext;
use crate::tools::context::ToolCallSource;
use crate::turn_diff_tracker::TurnDiffTracker;
use codex_protocol::models::SearchToolCallParams;
use pretty_assertions::assert_eq;
use std::sync::Mutex;
use tokio_util::sync::CancellationToken;

const DENIED: &str = "CovenantDenied { code: unknown_tool }";

#[derive(Clone, Debug, Default, PartialEq)]
struct Calls {
    telemetry: usize,
    matches_kind: usize,
    pre_tool_payload: usize,
    handled: Vec<(ToolName, ToolPayload)>,
    diff_factory: usize,
    diff_input: Vec<(String, String)>,
}

struct RecordingRuntime {
    name: ToolName,
    calls: Arc<Mutex<Calls>>,
}

impl ToolExecutor<ToolInvocation> for RecordingRuntime {
    fn tool_name(&self) -> ToolName {
        self.name.clone()
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::Function(codex_tools::ResponsesApiTool {
            name: self.name.name.clone(),
            description: "Records actual registry dispatch for admission tests.".to_string(),
            strict: false,
            defer_loading: None,
            parameters: codex_tools::JsonSchema::default(),
            output_schema: None,
        })
    }

    fn handle<'a>(&'a self, invocation: ToolInvocation) -> codex_tools::ToolExecutorFuture<'a>
    where
        ToolInvocation: 'a,
    {
        self.calls
            .lock()
            .unwrap()
            .handled
            .push((invocation.tool_name, invocation.payload));
        Box::pin(async {
            Ok(Box::new(FunctionToolOutput::from_text(
                "handler reached".to_string(),
                /*success*/ Some(true),
            )) as Box<dyn ToolOutput>)
        })
    }
}

impl CoreToolRuntime for RecordingRuntime {
    fn telemetry_tags(&self, _invocation: &ToolInvocation) -> ToolTelemetryTags {
        self.calls.lock().unwrap().telemetry += 1;
        Vec::new()
    }

    fn matches_kind(&self, _payload: &ToolPayload) -> bool {
        self.calls.lock().unwrap().matches_kind += 1;
        // Deliberately accepts all shapes: admission must enforce its own form
        // contract even if a registered handler would execute a forbidden form.
        true
    }

    fn pre_tool_use_payload(&self, _invocation: &ToolInvocation) -> Option<PreToolUsePayload> {
        self.calls.lock().unwrap().pre_tool_payload += 1;
        None
    }

    fn post_tool_use_payload(
        &self,
        _invocation: &ToolInvocation,
        _result: &dyn ToolOutput,
    ) -> Option<PostToolUsePayload> {
        None
    }

    fn create_diff_consumer(&self) -> Option<Box<dyn ToolArgumentDiffConsumer>> {
        self.calls.lock().unwrap().diff_factory += 1;
        Some(Box::new(RecordingDiffConsumer(Arc::clone(&self.calls))))
    }
}

struct RecordingDiffConsumer(Arc<Mutex<Calls>>);

impl ToolArgumentDiffConsumer for RecordingDiffConsumer {
    fn consume_diff(
        &mut self,
        _turn: &TurnContext,
        call_id: String,
        diff: &str,
    ) -> Option<EventMsg> {
        self.0
            .lock()
            .unwrap()
            .diff_input
            .push((call_id, diff.to_string()));
        None
    }
}

fn recording_registry(name: ToolName, calls: &Arc<Mutex<Calls>>) -> ToolRegistry {
    ToolRegistry::from_tools([Arc::new(RecordingRuntime {
        name,
        calls: Arc::clone(calls),
    }) as Arc<dyn CoreToolRuntime>])
}

fn invocation(
    session: &Arc<Session>,
    turn: &Arc<TurnContext>,
    name: ToolName,
    payload: ToolPayload,
) -> ToolInvocation {
    ToolInvocation {
        session: Arc::clone(session),
        turn: Arc::clone(turn),
        step_context: StepContext::for_test(Arc::clone(turn)),
        cancellation_token: CancellationToken::new(),
        tracker: Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new())),
        call_id: "admission-call".to_string(),
        tool_name: name,
        source: ToolCallSource::Direct,
        payload,
    }
}

fn function_payload() -> ToolPayload {
    ToolPayload::Function {
        arguments: r#"{"cmd":"fixture command","nested":{"keep":[1,"two"]}}"#.to_string(),
    }
}

fn custom_payload() -> ToolPayload {
    ToolPayload::Custom {
        input: "*** Begin Patch\n*** Add File: fixture.txt\n+keep this input\n*** End Patch"
            .to_string(),
    }
}

async fn assert_registered_denials(cases: Vec<(ToolName, ToolPayload)>) {
    let (session, turn) = crate::session::tests::make_session_and_context().await;
    let (session, turn) = (Arc::new(session), Arc::new(turn));
    let mut actual = Vec::new();
    let mut expected = Vec::new();
    for (name, payload) in cases {
        let calls = Arc::new(Mutex::new(Calls::default()));
        let registry = recording_registry(name.clone(), &calls);
        let terminal = Arc::new(AtomicBool::new(false));
        let result = registry
            .dispatch_any_with_terminal_outcome(
                invocation(&session, &turn, name.clone(), payload.clone()),
                Some(Arc::clone(&terminal)),
            )
            .await
            .map(|result| result.result.log_output())
            .map_err(|error| error.to_string());
        actual.push((
            name.clone(),
            payload.clone(),
            result,
            calls.lock().unwrap().clone(),
            terminal.load(Ordering::Acquire),
        ));
        expected.push((
            name,
            payload,
            Err(DENIED.to_string()),
            Calls::default(),
            true,
        ));
    }
    assert_eq!(actual, expected);
}

#[tokio::test]
async fn covenant_admission_denies_registered_unknown_read_none_and_writer_tools() {
    let names = [
        "unknown_tool",
        "new_read_tool",
        "read_file",
        "list_dir",
        "view_image",
        "get_context_remaining",
        "update_plan",
        "write_stdin",
        "shell",
        "request_permissions",
        "spawn_agent",
        "web_search",
        "mcp__server__exec_command",
    ];
    assert_registered_denials(
        names
            .into_iter()
            .map(|name| (ToolName::plain(name), function_payload()))
            .collect(),
    )
    .await;
}

#[tokio::test]
async fn covenant_admission_denies_namespaces_and_identity_lookalikes() {
    let mut cases = Vec::new();
    for (name, payload) in [
        ("exec_command", function_payload()),
        ("apply_patch", custom_payload()),
    ] {
        for namespace in ["", "functions", "mcp", "extensions"] {
            cases.push((ToolName::namespaced(namespace, name), payload.clone()));
        }
        for changed in [
            format!("functions.{name}"),
            format!(" {name}"),
            format!("{name} "),
            name.to_uppercase(),
            format!("{name}\u{200b}"),
        ] {
            cases.push((ToolName::plain(changed), payload.clone()));
        }
    }
    assert_registered_denials(cases).await;
}

#[tokio::test]
async fn covenant_admission_denies_wrong_forms_even_when_runtime_accepts_them() {
    let search = ToolPayload::ToolSearch {
        arguments: serde_json::from_value::<SearchToolCallParams>(serde_json::json!({
            "query": "exec_command",
            "limit": 1,
        }))
        .unwrap(),
    };
    assert_registered_denials(vec![
        (ToolName::plain("exec_command"), custom_payload()),
        (ToolName::plain("apply_patch"), function_payload()),
        (ToolName::plain("exec_command"), search.clone()),
        (ToolName::plain("apply_patch"), search.clone()),
        (ToolName::plain("tool_search"), search),
    ])
    .await;
}

#[tokio::test]
async fn covenant_admission_denies_unregistered_identity_and_claims_terminal_outcome() {
    let (session, turn) = crate::session::tests::make_session_and_context().await;
    let (session, turn) = (Arc::new(session), Arc::new(turn));
    let terminal = Arc::new(AtomicBool::new(false));
    let result = ToolRegistry::default()
        .dispatch_any_with_terminal_outcome(
            invocation(
                &session,
                &turn,
                ToolName::plain("unknown_sensitive_name_that_must_not_appear_in_denial"),
                function_payload(),
            ),
            Some(Arc::clone(&terminal)),
        )
        .await
        .map(|result| result.result.log_output())
        .map_err(|error| error.to_string());
    assert_eq!(
        (result, terminal.load(Ordering::Acquire)),
        (Err(DENIED.to_string()), true),
    );
}

#[tokio::test]
async fn covenant_admission_preserves_admitted_call_identity_payload_and_result() {
    let (session, turn) = crate::session::tests::make_session_and_context().await;
    let (session, turn) = (Arc::new(session), Arc::new(turn));
    for (name, payload) in [
        (ToolName::plain("exec_command"), function_payload()),
        (ToolName::plain("apply_patch"), custom_payload()),
    ] {
        let calls = Arc::new(Mutex::new(Calls::default()));
        let registry = recording_registry(name.clone(), &calls);
        let terminal = Arc::new(AtomicBool::new(false));
        let result = registry
            .dispatch_any_with_terminal_outcome(
                invocation(&session, &turn, name.clone(), payload.clone()),
                Some(Arc::clone(&terminal)),
            )
            .await
            .expect("the exact admitted identity and form must reach its runtime");
        assert_eq!(
            (
                result.call_id,
                result.payload,
                result.result.to_response_item("admission-call", &payload),
                calls.lock().unwrap().handled.clone(),
                terminal.load(Ordering::Acquire),
            ),
            (
                "admission-call".to_string(),
                payload.clone(),
                FunctionToolOutput::from_text(
                    "handler reached".to_string(),
                    /*success*/ Some(true),
                )
                .to_response_item("admission-call", &payload),
                vec![(name, payload)],
                true,
            ),
        );
    }
}

#[tokio::test]
async fn covenant_admission_skips_pre_tool_payload_for_both_admitted_forms() {
    let (session, turn) = crate::session::tests::make_session_and_context().await;
    let (session, turn) = (Arc::new(session), Arc::new(turn));
    let mut actual = Vec::new();
    for (name, payload) in [
        (ToolName::plain("exec_command"), function_payload()),
        (ToolName::plain("apply_patch"), custom_payload()),
    ] {
        let calls = Arc::new(Mutex::new(Calls::default()));
        recording_registry(name.clone(), &calls)
            .dispatch_any_with_terminal_outcome(
                invocation(&session, &turn, name, payload),
                /*terminal_outcome_reached*/ None,
            )
            .await
            .expect("admitted call must execute");
        let recorded = calls.lock().unwrap();
        actual.push((recorded.handled.len(), recorded.pre_tool_payload));
    }
    assert_eq!(actual, vec![(1, 0), (1, 0)]);
}

#[test]
fn covenant_admission_denies_excluded_streaming_consumers_before_factory_callback() {
    let names = [
        ToolName::plain("unknown_custom"),
        ToolName::plain("exec_command"),
        ToolName::plain("write_stdin"),
        ToolName::namespaced("functions", "apply_patch"),
        ToolName::namespaced("", "apply_patch"),
        ToolName::namespaced("extensions", "apply_patch"),
        ToolName::plain("functions.apply_patch"),
    ];
    let mut actual = Vec::new();
    let mut expected = Vec::new();
    for name in names {
        let calls = Arc::new(Mutex::new(Calls::default()));
        let registry = recording_registry(name.clone(), &calls);
        let consumer = registry.create_diff_consumer(&name);
        actual.push((
            name.clone(),
            consumer.is_some(),
            calls.lock().unwrap().clone(),
        ));
        expected.push((name, false, Calls::default()));
    }
    assert_eq!(actual, expected);
}

#[tokio::test]
async fn covenant_admission_preserves_unqualified_patch_streaming() {
    let (_session, turn) = crate::session::tests::make_session_and_context().await;
    let name = ToolName::plain("apply_patch");
    let calls = Arc::new(Mutex::new(Calls::default()));
    let registry = recording_registry(name.clone(), &calls);
    let mut consumer = registry
        .create_diff_consumer(&name)
        .expect("admitted custom patch must keep its streaming consumer");
    assert!(
        consumer
            .consume_diff(&turn, "stream-call".to_string(), "unchanged delta")
            .is_none()
    );
    assert!(consumer.finish().unwrap().is_none());
    assert_eq!(
        calls.lock().unwrap().clone(),
        Calls {
            diff_factory: 1,
            diff_input: vec![("stream-call".to_string(), "unchanged delta".to_string())],
            ..Calls::default()
        },
    );
}
