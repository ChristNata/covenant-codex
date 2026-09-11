use super::{SidecarDecision, SidecarError, parse_response};

#[test]
fn parses_only_closed_decision_shapes() {
    assert!(matches!(
        parse_response(br#"{"decision":"ALLOW"}"#),
        Ok(SidecarDecision::Allow)
    ));
    assert!(matches!(
        parse_response(br#"{"decision":"DENY","reason":"blocked"}"#),
        Ok(SidecarDecision::Deny { reason }) if reason == "blocked"
    ));
    assert!(matches!(
        parse_response(br#"{"decision":"ALLOW_WITH_CONTEXT","context":"facts"}"#),
        Ok(SidecarDecision::AllowWithContext { context }) if context == "facts"
    ));
}

#[test]
fn rejects_ambiguous_or_extra_response_fields() {
    for response in [
        br#"{"decision":"ALLOW","reason":"unexpected"}"#.as_slice(),
        br#"{"decision":"DENY"}"#.as_slice(),
        br#"{"decision":"ALLOW","extra":true}"#.as_slice(),
    ] {
        assert!(matches!(
            parse_response(response),
            Err(SidecarError::Malformed)
        ));
    }
}
