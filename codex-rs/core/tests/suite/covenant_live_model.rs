use anyhow::Result;
use codex_core::TurnInputRequest;
use codex_features::Feature;
use codex_login::CodexAuth;
use codex_protocol::openai_models::ToolMode;
use codex_protocol::protocol::EventMsg;
use codex_protocol::user_input::UserInput;
use core_test_support::responses;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use pretty_assertions::assert_eq;

const LIVE_MODEL: &str = "gpt-6-astra";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn covenant_live_model_snapshot_drives_agent_request_and_two_tool_schema() -> Result<()> {
    let server = responses::start_mock_server().await;
    let response_mock = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("resp-1"),
            responses::ev_assistant_message("msg-1", "done"),
            responses::ev_completed("resp-1"),
        ]),
    )
    .await;
    let mut catalog = codex_models_manager::covenant_model_catalog()?;
    catalog.models[0].slug = LIVE_MODEL.to_owned();
    catalog.models[0].tool_mode = Some(ToolMode::CodeModeOnly);
    let mut builder = test_codex()
        .with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing())
        .with_config(move |config| {
            config.model = Some(LIVE_MODEL.to_owned());
            config.model_catalog = Some(catalog);
            assert!(config.features.disable(Feature::CodeModeHost).is_ok());
        });
    let test = builder.build_with_auto_env(&server).await?;

    test.codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "Reply with done".to_owned(),
            text_elements: Vec::new(),
        }]))
        .await?;
    let mut code_mode_warnings = Vec::new();
    loop {
        let event = wait_for_event(&test.codex, |_| true).await;
        if let EventMsg::Warning(warning) = &event
            && warning.message.contains("Code Mode")
        {
            code_mode_warnings.push(warning.message.clone());
        }
        if matches!(event, EventMsg::TurnComplete(_)) {
            break;
        }
    }
    assert_eq!(code_mode_warnings, Vec::<String>::new());

    let request = response_mock.single_request();
    let body = request.body_json();
    let mut tools: Vec<_> = body["tools"]
        .as_array()
        .expect("outbound tools")
        .iter()
        .map(|tool| {
            (
                tool["name"].as_str().unwrap(),
                tool["type"].as_str().unwrap(),
            )
        })
        .collect();
    tools.sort_unstable();
    assert_eq!(body["model"], LIVE_MODEL);
    assert_eq!(
        tools,
        vec![("apply_patch", "custom"), ("exec_command", "function")]
    );
    Ok(())
}
