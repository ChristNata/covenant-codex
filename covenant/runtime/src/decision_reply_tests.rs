use super::*;
use pretty_assertions::assert_eq;
use std::cmp::Ordering;

const ALLOW: &[u8] = br#"{"decision":"ALLOW"}"#;
const CANARY: &str = "REPLY-PRIVATE-CANARY";

fn complete() -> ReplyCompletion {
    ReplyCompletion {
        stdin: StdinCompletion::WrittenAndClosed,
        stdout: StdoutCompletion::Eof,
        process: ProcessCompletion::Exited { code: 0 },
        cancellation: CancellationCompletion::NotCancelled,
    }
}

fn safe(error: Option<DecisionReplyError>) -> Option<DecisionReplyError> {
    if let Some(error) = error {
        let display = error.to_string();
        let debug = format!("{error:?}");
        assert!(display.len() <= 96 && debug.len() <= 96);
        assert!(!display.contains(CANARY) && !debug.contains(CANARY));
    }
    error
}

fn boundary(reply: &DecisionReply, relation: Ordering) -> Instant {
    let unit = Duration::from_nanos(/*nanos*/ 1);
    match relation {
        Ordering::Less => reply.deadline() - unit,
        Ordering::Equal => reply.deadline(),
        Ordering::Greater => reply.deadline() + unit,
    }
}

#[test]
fn exact_allow_accepts_every_split_and_bytewise_arrival() {
    let mut observations = Vec::new();
    for split in 0..=ALLOW.len() {
        let mut reply = DecisionReply::begin_attempt();
        let deadline = reply.deadline();
        observations.push((
            safe(reply.push_stdout(&ALLOW[..split]).err()),
            safe(reply.push_stdout(&ALLOW[split..]).err()),
            reply.deadline() == deadline,
            safe(reply.finish(complete()).err()),
        ));
    }
    let mut reply = DecisionReply::begin_attempt();
    let deadline = reply.deadline();
    let bytewise: Vec<_> = ALLOW
        .chunks(/*chunk_size*/ 1)
        .map(|chunk| safe(reply.push_stdout(chunk).err()))
        .collect();
    assert_eq!(
        observations,
        vec![(None, None, true, None); observations.len()]
    );
    assert_eq!(bytewise, vec![None; ALLOW.len()]);
    assert_eq!(reply.deadline(), deadline);
    assert_eq!(safe(reply.finish(complete()).err()), None);
}

#[test]
fn every_proper_prefix_is_incomplete_at_finish() {
    let mut observations = Vec::new();
    for length in 0..ALLOW.len() {
        let mut reply = DecisionReply::begin_attempt();
        observations.push((
            safe(reply.push_stdout(&ALLOW[..length]).err()),
            safe(reply.finish(complete()).err()),
        ));
    }
    assert_eq!(
        observations,
        vec![(None, Some(DecisionReplyError::InvalidOutput)); ALLOW.len()]
    );
}

#[test]
fn every_mismatch_and_nonexact_response_is_sticky() {
    let mut cases: Vec<_> = (0..ALLOW.len())
        .map(|position| {
            let mut bytes = ALLOW.to_vec();
            bytes[position] = b'!';
            bytes
        })
        .collect();
    cases.extend([
        [ALLOW, b"\n"].concat(),
        [ALLOW, b"\0"].concat(),
        [ALLOW, ALLOW].concat(),
        br#"{ "decision": "ALLOW" }"#.to_vec(),
        br#"{"decision":"DENY"}"#.to_vec(),
        br#"{"decision":"ALLOW_WITH_CONTEXT"}"#.to_vec(),
        format!(r#"{{"decision":"ALLOW","extra":"{CANARY}"}}"#).into_bytes(),
        vec![0xff, 0xfe],
    ]);
    let mut observations = Vec::new();
    for bytes in cases {
        let mut reply = DecisionReply::begin_attempt();
        let after = boundary(&reply, Ordering::Greater);
        let mut completion = complete();
        completion.cancellation = CancellationCompletion::Cancelled;
        observations.push((
            safe(reply.push_stdout(&bytes).err()),
            safe(reply.push_stdout(&[]).err()),
            safe(reply.push_stdout(ALLOW).err()),
            safe(reply.finish_at(completion, after).err()),
        ));
    }
    let invalid = Some(DecisionReplyError::InvalidOutput);
    assert_eq!(
        observations,
        vec![(invalid, invalid, invalid, invalid); observations.len()]
    );
}

#[test]
fn oversized_chunk_and_separate_overflow_refuse() {
    let mut large = vec![b'x'; 1_048_576];
    large[..ALLOW.len()].copy_from_slice(ALLOW);
    large[ALLOW.len()..ALLOW.len() + CANARY.len()].copy_from_slice(CANARY.as_bytes());
    let mut reply = DecisionReply::begin_attempt();
    let large_observation = (
        safe(reply.push_stdout(&large).err()),
        safe(reply.push_stdout(&[]).err()),
        safe(reply.push_stdout(ALLOW).err()),
        safe(reply.finish(complete()).err()),
    );
    let mut reply = DecisionReply::begin_attempt();
    let separate_observation = (
        safe(reply.push_stdout(ALLOW).err()),
        safe(reply.push_stdout(b"\0").err()),
        safe(reply.push_stdout(&[]).err()),
        safe(reply.finish(complete()).err()),
    );
    let invalid = Some(DecisionReplyError::InvalidOutput);
    assert_eq!(
        (large_observation, separate_observation),
        (
            (invalid, invalid, invalid, invalid),
            (None, invalid, invalid, invalid)
        )
    );
}

#[test]
fn completion_facts_and_failure_priority_are_required() {
    let mut actual = Vec::new();
    let mut expected = Vec::new();
    for stdin in [
        StdinCompletion::WrittenAndClosed,
        StdinCompletion::IncompleteOrFailed,
    ] {
        for stdout in [StdoutCompletion::Eof, StdoutCompletion::IncompleteOrFailed] {
            for process in [
                ProcessCompletion::Exited { code: 0 },
                ProcessCompletion::Exited { code: 1 },
                ProcessCompletion::Exited { code: u32::MAX },
                ProcessCompletion::RunningOrUnavailable,
            ] {
                for cancellation in [
                    CancellationCompletion::NotCancelled,
                    CancellationCompletion::Cancelled,
                ] {
                    let mut reply = DecisionReply::begin_attempt();
                    let before = boundary(&reply, Ordering::Less);
                    let setup = safe(reply.push_stdout_at(ALLOW, before).err());
                    actual.push((
                        setup,
                        safe(
                            reply
                                .finish_at(
                                    ReplyCompletion {
                                        stdin,
                                        stdout,
                                        process,
                                        cancellation,
                                    },
                                    before,
                                )
                                .err(),
                        ),
                    ));
                    let error = match (cancellation, stdin, stdout, process) {
                        (CancellationCompletion::Cancelled, _, _, _) => {
                            Some(DecisionReplyError::Cancelled)
                        }
                        (_, StdinCompletion::IncompleteOrFailed, _, _) => {
                            Some(DecisionReplyError::IncompleteInput)
                        }
                        (_, _, StdoutCompletion::IncompleteOrFailed, _) => {
                            Some(DecisionReplyError::IncompleteOutput)
                        }
                        (_, _, _, ProcessCompletion::RunningOrUnavailable) => {
                            Some(DecisionReplyError::UnsuccessfulProcess)
                        }
                        (_, _, _, ProcessCompletion::Exited { code }) if code != 0 => {
                            Some(DecisionReplyError::UnsuccessfulProcess)
                        }
                        (_, _, _, ProcessCompletion::Exited { code: 0 }) => None,
                        (_, _, _, ProcessCompletion::Exited { .. }) => unreachable!(),
                    };
                    expected.push((None, error));
                }
            }
        }
    }
    // Completion errors precede an otherwise incomplete response body.
    for (completion, error) in [
        (
            ReplyCompletion {
                stdin: StdinCompletion::IncompleteOrFailed,
                ..complete()
            },
            DecisionReplyError::IncompleteInput,
        ),
        (
            ReplyCompletion {
                stdout: StdoutCompletion::IncompleteOrFailed,
                ..complete()
            },
            DecisionReplyError::IncompleteOutput,
        ),
        (
            ReplyCompletion {
                process: ProcessCompletion::Exited { code: 7 },
                ..complete()
            },
            DecisionReplyError::UnsuccessfulProcess,
        ),
    ] {
        let reply = DecisionReply::begin_attempt();
        let before = boundary(&reply, Ordering::Less);
        actual.push((None, safe(reply.finish_at(completion, before).err())));
        expected.push((None, Some(error)));
    }
    assert_eq!(actual, expected);
}

#[test]
fn real_transition_methods_enforce_exact_deadline_boundaries() {
    let mut actual = Vec::new();
    for relation in [Ordering::Less, Ordering::Equal, Ordering::Greater] {
        let mut pushing = DecisionReply::begin_attempt();
        let observed = boundary(&pushing, relation);
        let push = safe(pushing.push_stdout_at(ALLOW, observed).err());
        let mut finishing = DecisionReply::begin_attempt();
        let before = boundary(&finishing, Ordering::Less);
        let observed = boundary(&finishing, relation);
        let setup = safe(finishing.push_stdout_at(ALLOW, before).err());
        actual.push((
            push,
            setup,
            safe(finishing.finish_at(complete(), observed).err()),
        ));
    }
    let expired = Some(DecisionReplyError::DeadlineExpired);
    let mut competing = Vec::new();
    for cancellation in [
        CancellationCompletion::NotCancelled,
        CancellationCompletion::Cancelled,
    ] {
        let mut reply = DecisionReply::begin_attempt();
        let before = boundary(&reply, Ordering::Less);
        let after = boundary(&reply, Ordering::Greater);
        let setup = safe(reply.push_stdout_at(ALLOW, before).err());
        competing.push((
            setup,
            safe(
                reply
                    .finish_at(
                        ReplyCompletion {
                            stdin: StdinCompletion::IncompleteOrFailed,
                            stdout: StdoutCompletion::IncompleteOrFailed,
                            process: ProcessCompletion::RunningOrUnavailable,
                            cancellation,
                        },
                        after,
                    )
                    .err(),
            ),
        ));
    }
    assert_eq!(
        (actual, competing),
        (
            vec![
                (None, None, None),
                (expired, None, expired),
                (expired, None, expired)
            ],
            vec![(None, expired), (None, Some(DecisionReplyError::Cancelled))]
        )
    );
}

#[test]
fn first_refusal_survives_empty_chunks_and_earlier_observations() {
    let mut actual = Vec::new();
    for first in [
        DecisionReplyError::InvalidOutput,
        DecisionReplyError::DeadlineExpired,
    ] {
        let mut reply = DecisionReply::begin_attempt();
        let before = boundary(&reply, Ordering::Less);
        let after = boundary(&reply, Ordering::Greater);
        let deadline = reply.deadline();
        let initial = match first {
            DecisionReplyError::InvalidOutput => reply.push_stdout_at(CANARY.as_bytes(), before),
            DecisionReplyError::DeadlineExpired => reply.push_stdout_at(&[], deadline),
            DecisionReplyError::IncompleteInput
            | DecisionReplyError::IncompleteOutput
            | DecisionReplyError::UnsuccessfulProcess
            | DecisionReplyError::Cancelled => unreachable!(),
        };
        let mut completion = complete();
        completion.cancellation = CancellationCompletion::Cancelled;
        actual.push((
            safe(initial.err()),
            safe(reply.push_stdout_at(&[], before).err()),
            safe(reply.push_stdout_at(ALLOW, before).err()),
            reply.deadline() == deadline,
            safe(reply.finish_at(completion, after).err()),
        ));
    }
    let invalid = Some(DecisionReplyError::InvalidOutput);
    let expired = Some(DecisionReplyError::DeadlineExpired);
    assert_eq!(
        actual,
        vec![
            (invalid, invalid, invalid, true, invalid),
            (expired, expired, expired, true, expired),
        ]
    );
}

#[test]
fn public_wrappers_expire_using_real_elapsed_time() {
    let start = Instant::now();
    let mut pushing = DecisionReply::begin_attempt();
    let middle = Instant::now();
    let mut finishing = DecisionReply::begin_attempt();
    let end = Instant::now();
    let budget = Duration::from_millis(/*millis*/ 1000);
    let pushing_deadline = pushing.deadline();
    let finishing_deadline = finishing.deadline();
    let deadlines_match = pushing_deadline >= start + budget
        && pushing_deadline <= middle + budget
        && finishing_deadline >= middle + budget
        && finishing_deadline <= end + budget;
    let setup = (
        safe(pushing.push_stdout(ALLOW).err()),
        safe(finishing.push_stdout(ALLOW).err()),
    );
    let wait = pushing_deadline
        .max(finishing_deadline)
        .saturating_duration_since(Instant::now())
        + Duration::from_millis(/*millis*/ 5);
    std::thread::sleep(wait);
    let empty_push = safe(pushing.push_stdout(&[]).err());
    let unchanged = (
        pushing.deadline() == pushing_deadline,
        finishing.deadline() == finishing_deadline,
    );
    let finishes = (
        safe(pushing.finish(complete()).err()),
        safe(finishing.finish(complete()).err()),
    );
    let expired = Some(DecisionReplyError::DeadlineExpired);
    assert_eq!(
        (deadlines_match, setup, empty_push, unchanged, finishes),
        (
            true,
            (None, None),
            expired,
            (true, true),
            (expired, expired)
        )
    );
}
