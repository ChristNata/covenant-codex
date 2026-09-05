use super::*;
use crate::session::tests::make_session_and_context;
use crate::tools::ExecutedToolCallRecorder;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::registry::CoreToolRuntime;
use crate::tools::registry::PostToolUsePayload;
use crate::tools::registry::ToolExecutor;
use crate::tools::registry::ToolRegistry;
use crate::tools::router::ToolRouter;
use crate::turn_diff_tracker::TurnDiffTracker;
use codex_extension_api::ToolCallOutcome;
use codex_protocol::error::CodexErrorDetails;
use codex_protocol::models::ExecutedToolCall;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseItem;
use codex_protocol::openai_models::ToolMode;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use pretty_assertions::assert_eq;
use serde_json::json;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Clone, Debug, Default, PartialEq)]
struct Calls {
    parallel: usize,
    readiness_factory: usize,
    readiness_poll: usize,
    telemetry: usize,
    kind: usize,
    handled: Vec<(ToolName, ToolPayload)>,
    started: usize,
    finished: Vec<ToolCallOutcome>,
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
            description: "Fixture runtime".to_string(),
            strict: false,
            defer_loading: None,
            parameters: codex_tools::JsonSchema::default(),
            output_schema: None,
        })
    }
    fn supports_parallel_tool_calls(&self) -> bool {
        self.calls.lock().unwrap().parallel += 1;
        false
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
                "fixture output".to_string(),
                /*success*/ Some(true),
            )) as Box<dyn ToolOutput>)
        })
    }
}

impl CoreToolRuntime for RecordingRuntime {
    fn wait_until_ready<'a>(
        &'a self,
        _session: &'a Arc<Session>,
    ) -> Option<futures::future::BoxFuture<'a, ()>> {
        self.calls.lock().unwrap().readiness_factory += 1;
        Some(Box::pin(async {
            self.calls.lock().unwrap().readiness_poll += 1;
        }))
    }
    fn telemetry_tags(&self, _invocation: &ToolInvocation) -> Vec<(&'static str, String)> {
        self.calls.lock().unwrap().telemetry += 1;
        Vec::new()
    }
    fn matches_kind(&self, _payload: &ToolPayload) -> bool {
        self.calls.lock().unwrap().kind += 1;
        true // The early identity gate must deny even a permissive registered runtime.
    }
    fn post_tool_use_payload(
        &self,
        _invocation: &ToolInvocation,
        _result: &dyn ToolOutput,
    ) -> Option<PostToolUsePayload> {
        None
    }
}

impl codex_extension_api::ToolLifecycleContributor for RecordingRuntime {
    fn on_tool_start<'a>(
        &'a self,
        _input: codex_extension_api::ToolStartInput<'a>,
    ) -> codex_extension_api::ToolLifecycleFuture<'a> {
        self.calls.lock().unwrap().started += 1;
        Box::pin(std::future::ready(()))
    }
    fn on_tool_finish<'a>(
        &'a self,
        input: codex_extension_api::ToolFinishInput<'a>,
    ) -> codex_extension_api::ToolLifecycleFuture<'a> {
        self.calls.lock().unwrap().finished.push(input.outcome);
        Box::pin(std::future::ready(()))
    }
}

struct Fixture {
    runtime: ToolCallRuntime,
    calls: Arc<Mutex<Calls>>,
    recorder: Arc<ExecutedToolCallRecorder>,
}

async fn fixture(name: ToolName) -> Fixture {
    let (mut session, mut turn) = make_session_and_context().await;
    Arc::make_mut(&mut turn.config)
        .features
        .enable(codex_features::Feature::ExecutedToolCallMetadata)
        .expect("synthetic fixture must enable attempted-tool metadata");
    let recorder = Arc::new(ExecutedToolCallRecorder::default());
    session.services.executed_tool_calls = Some(Arc::clone(&recorder));
    let calls = Arc::new(Mutex::new(Calls::default()));
    let handler = Arc::new(RecordingRuntime {
        name,
        calls: Arc::clone(&calls),
    });
    let mut extensions =
        codex_extension_api::ExtensionRegistryBuilder::<crate::config::Config>::new();
    extensions.tool_lifecycle_contributor(handler.clone());
    session.services.extensions = Arc::new(extensions.build());
    let router = Arc::new(ToolRouter::from_parts(
        ToolRegistry::from_tools([handler as Arc<dyn CoreToolRuntime>]),
        Vec::new(),
        ToolMode::Direct,
        BTreeMap::new(),
        /*tool_namespaces_info*/ None,
        &[],
    ));
    let step = StepContext::for_test(Arc::new(turn)).with_tool_router_for_test(router);
    Fixture {
        runtime: ToolCallRuntime::new(
            Arc::new(session),
            step,
            Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new())),
        ),
        calls,
        recorder,
    }
}

fn call(name: ToolName, payload: ToolPayload) -> ToolCall {
    ToolCall {
        tool_name: name,
        call_id: "readiness-call".to_string(),
        payload,
        encrypted_function_args: None,
    }
}

fn output() -> ResponseItem {
    ResponseInputItem::FunctionCallOutput {
        call_id: "readiness-call".to_string(),
        output: FunctionCallOutputPayload::from_text(String::new()),
    }
    .into()
}

fn metadata(recorder: &ExecutedToolCallRecorder) -> (bool, ResponseItem) {
    let mut item = output();
    let attached =
        recorder.attach_pending_to_prompt(std::slice::from_mut(&mut item), &mut HashMap::new());
    (attached, item)
}

fn denied(result: Result<ResponseItemEnvelope, CodexErr>) -> bool {
    result.is_err_and(|error| !error.is_retryable()
        && matches!(error.details(), CodexErrorDetails::Fatal(message) if message == "CovenantDenied { code: unknown_tool }"))
}

#[tokio::test]
async fn covenant_readiness_admission_denies_before_callbacks_and_metadata() {
    let function = ToolPayload::Function {
        arguments: "{\"preserve\":1}".to_string(),
    };
    let mut cases: Vec<_> = [
        "unknown_tool",
        "read_file",
        "get_context_remaining",
        "write_stdin",
    ]
    .into_iter()
    .map(|name| call(ToolName::plain(name), function.clone()))
    .collect();
    for namespace in ["", "functions", "extension"] {
        cases.push(call(
            ToolName::namespaced(namespace, "exec_command"),
            function.clone(),
        ));
    }
    cases.push(call(ToolName::plain("apply_patch"), function));
    cases.push(call(
        ToolName::plain("exec_command"),
        ToolPayload::Custom {
            input: "wrong form".to_string(),
        },
    ));
    let mut actual = Vec::new();
    for call in cases {
        let fixture = fixture(call.tool_name.clone()).await;
        let result = fixture
            .runtime
            .handle_tool_call(call, CancellationToken::new())
            .await;
        actual.push((
            denied(result),
            fixture.calls.lock().unwrap().clone(),
            metadata(&fixture.recorder),
        ));
    }
    assert_eq!(
        actual,
        vec![(true, Calls::default(), (false, output())); actual.len()]
    );
}

#[tokio::test]
async fn covenant_readiness_admission_precedes_cancelled_lifecycle() {
    let mut denied_observation = None;
    for name in ["unknown_tool", "exec_command"] {
        let fixture = fixture(ToolName::plain(name)).await;
        // Holding the actual gate prevents a baseline task from reaching inner admission before cancellation.
        let guard = Arc::clone(&fixture.runtime.parallel_execution)
            .try_write_owned()
            .unwrap();
        let token = CancellationToken::new();
        token.cancel();
        let result = fixture
            .runtime
            .handle_tool_call(
                call(
                    ToolName::plain(name),
                    ToolPayload::Function {
                        arguments: "{}".to_string(),
                    },
                ),
                token,
            )
            .await;
        drop(guard);
        let calls = fixture.calls.lock().unwrap().clone();
        if name == "unknown_tool" {
            denied_observation = Some((denied(result), calls, metadata(&fixture.recorder)));
        } else {
            // Readiness may or may not be polled before cancellation; lifecycle and handler facts are deterministic.
            assert_eq!(
                (
                    result.is_ok(),
                    calls.parallel,
                    calls.started,
                    calls.finished,
                    calls.handled,
                    metadata(&fixture.recorder).0
                ),
                (true, 1, 0, vec![ToolCallOutcome::Aborted], Vec::new(), true)
            );
        }
    }
    assert_eq!(
        denied_observation,
        Some((true, Calls::default(), (false, output())))
    );
}

#[tokio::test]
async fn covenant_readiness_admission_preserves_both_wire_calls_and_observers() {
    for (name, wire, payload, arguments) in [
        (
            "exec_command",
            json!({"type":"function_call","name":"exec_command","call_id":"readiness-call","arguments":"{\"keep\":1}"}),
            ToolPayload::Function {
                arguments: "{\"keep\":1}".to_string(),
            },
            json!({"keep":1}),
        ),
        (
            "apply_patch",
            json!({"type":"custom_tool_call","name":"apply_patch","call_id":"readiness-call","input":"preserved patch\n"}),
            ToolPayload::Custom {
                input: "preserved patch\n".to_string(),
            },
            json!("preserved patch\n"),
        ),
    ] {
        let call = ToolRouter::build_tool_call(serde_json::from_value(wire).unwrap())
            .unwrap()
            .unwrap();
        let fixture = fixture(ToolName::plain(name)).await;
        let result = fixture
            .runtime
            .handle_tool_call(call, CancellationToken::new())
            .await
            .expect("unqualified wire call must reach its actual registered runtime");
        let expected_response: ResponseItem = FunctionToolOutput::from_text(
            "fixture output".to_string(),
            /*success*/ Some(true),
        )
        .to_response_item("readiness-call", &payload)
        .into();
        let mut expected_metadata = output();
        expected_metadata
            .append_executed_tool_calls(vec![ExecutedToolCall::new(name.to_string(), arguments)]);
        assert_eq!(
            (
                result.item,
                fixture.calls.lock().unwrap().clone(),
                metadata(&fixture.recorder)
            ),
            (
                expected_response,
                Calls {
                    parallel: 1,
                    readiness_factory: 1,
                    readiness_poll: 1,
                    telemetry: 1,
                    kind: 1,
                    handled: vec![(ToolName::plain(name), payload)],
                    started: 1,
                    finished: vec![ToolCallOutcome::Completed { success: true }]
                },
                (true, expected_metadata)
            )
        );
    }
}
