use super::FixtureFailure;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use serde_json::value::RawValue;

pub(super) struct MarkerExpectation {
    pub(super) nonce: String,
    pub(super) cwd: String,
    pub(super) model: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MarkerWire {
    nonce: String,
    pid: u32,
    input: Box<RawValue>,
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
enum TranscriptPath {
    Path(String),
    Absent(()),
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct HookInput {
    session_id: String,
    transcript_path: TranscriptPath,
    cwd: String,
    hook_event_name: String,
    model: String,
    permission_mode: String,
    source: String,
}

#[derive(Serialize)]
struct ObservedMarker {
    nonce: String,
    pid: u32,
    input: HookInput,
}

fn starts_object(bytes: &[u8]) -> bool {
    bytes
        .iter()
        .copied()
        .find(|byte| !matches!(byte, b' ' | b'\r' | b'\n' | b'\t'))
        == Some(b'{')
}

pub(super) fn parse_marker(
    bytes: &[u8],
    expected: &MarkerExpectation,
) -> Result<Value, FixtureFailure> {
    if bytes.len() > 16_384 {
        return Err(FixtureFailure::Limit);
    }
    if !starts_object(bytes) {
        return Err(FixtureFailure::Marker);
    }
    let marker: MarkerWire = serde_json::from_slice(bytes).map_err(|_| FixtureFailure::Marker)?;
    if !starts_object(marker.input.get().as_bytes()) {
        return Err(FixtureFailure::Marker);
    }
    let input: HookInput =
        serde_json::from_str(marker.input.get()).map_err(|_| FixtureFailure::Marker)?;
    if marker.pid == 0
        || marker.nonce != expected.nonce
        || input.session_id.is_empty()
        || input.cwd != expected.cwd
        || input.model != expected.model
        || input.hook_event_name != "SessionStart"
        || input.source != "startup"
    {
        return Err(FixtureFailure::Marker);
    }
    // These are the decoded observations, including session/transcript/permission data.
    // Expected fixture values are used only for comparison, never to rebuild the record.
    serde_json::to_value(ObservedMarker {
        nonce: marker.nonce,
        pid: marker.pid,
        input,
    })
    .map_err(|_| FixtureFailure::Marker)
}
