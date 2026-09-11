# Decide v1 wire contract

`decide_v1` is the versioned object sent from the Covenant Codex
fork to the managed policy decider before an exec process starts
or a patch reaches the patch engine. This document describes the
schema F10 produces and G4 evaluates. The JSON schema at
`covenant/schema/decide-v1.json` is byte-identical to the harness
workspace copy.

## Transport

The fork executes the launcher-supplied absolute path in
`COVENANT_DECIDER_PATH` with arguments `hook`, `decide`,
`--client`, and `codex`.

It writes one UTF-8 JSON envelope to stdin, closes stdin, and
waits at most 1000 ms. The policy-runner process environment is
scrubbed; no shell is involved. The path digest must equal
`COVENANT_DECIDER_SHA256` immediately before creation, hashing
the opened handle.

## Envelope

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `hook_event_name` | string enum: `Exec`, `Patch` | yes | Selects the matching decision kind. |
| `cwd` | string, absolute path | yes | Effective working directory. |
| `command` | string | exec only | Display-only rendering; never reparsed for execution. |
| `path` | string, absolute path | patch only | Display-only primary patch path. |
| `decide_v1` | `DecideV1` object | yes | Frozen authoritative request object. |

`hook_event_name` and `decide_v1.kind` must agree. The fork
rejects a mismatch before invoking the decider.

## `DecideV1`

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `version` | integer constant `1` | yes | Schema version. |
| `kind` | string enum: `exec`, `patch` | yes | Active branch. |
| `exec` | `ExecCall` object | when `kind` is `exec` | Final process-start request. |
| `patch` | `PatchCall` object | when `kind` is `patch` | Final patch request. |

Exactly one branch is present. Unknown fields, a missing active
branch, or an inactive branch are malformed and therefore denied.

### `ExecCall`

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `program` | string | yes | Final executable or shell program. |
| `argv` | array of strings | yes | Final argument vector, preserved as an array. |
| `cwd` | string, absolute path | yes | Final process working directory. |
| `env` | string-to-string object | yes | Complete post-overlay post-scrub process environment: the exact map the spawned process sees. Not an allowlist subset. No `env_allowlist` field. Windows env keys are case-insensitive: the map is one canonical block, colliding keys (`PATH` vs `Path`) are malformed, and G4 evaluates that block. |
| `sandbox` | string | yes | Effective sandbox mode. |
| `network` | boolean | yes | Effective network permission. |
| `tty` | boolean | yes | Whether the final process receives a terminal. |

One immutable Windows environment block is the single source
for both `decide_v1.exec.env` and the spawned process. Keys are
canonicalized case-insensitively; a case-colliding or invalid
entry is malformed and therefore denied. The closed secret-key
set is `COVENANT_DECIDER_PATH`, `COVENANT_DECIDER_SHA256`,
`COVENANT_CHILD_MARKER`, `CODEX_AUTH_HOME`, plus the
provider/login keys F00 recorded at the tag (or that documented
prefix set). Those keys, including mixed-case and collision
spellings, are absent from the block. `env` is set-equal to
that block. G4 must evaluate every pair.

No shell-form reconstruction is allowed. A command, cwd,
environment, sandbox, network, terminal, or argument change
after ALLOW invalidates the result and requires a new decision.

### `PatchCall`

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `paths` | array of absolute-path strings | yes | All resolved source and destination paths. |
| `operations` | non-empty array of `PatchOperation` | yes | Per-file mutations. |
| `permissions` | `PatchPermissions` object | yes | Effective write authority snapshot. |
| `resolved_identities` | non-empty array of `ResolvedIdentity` | yes | Identity information F13 needs to detect ancestor replacement, junction swaps, and reparse-point races. |

| `PatchOperation` field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `path` | absolute-path string | yes | Resolved source or target path. |
| `op` | string enum: `add`, `update`, `delete`, `move` | yes | Mutation kind. |
| `destination` | absolute-path string | for `move` | Resolved destination path. |
| `hunks` | array of `Hunk` objects | yes | Structured content changes, never opaque model text. |
| `pre_image_digest` | string | for `update`, `delete`, and `move`; not for `add` | SHA-256 hex of the pre-image content. |

| `ResolvedIdentity` field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `path` | absolute-path string | yes | Resolved existing ancestor or target. |
| `kind` | string enum: `file`, `directory`, `symlink`, `junction` | yes | Filesystem kind after resolution. |
| `target` | string | for `symlink` / `junction`; not for `file` / `directory` | Link or junction target. |
| `ancestor_identities` | array of `ResolvedIdentity` (without nested ancestors) | yes | Resolved existing ancestors. Each ancestor requires `win32_normalized`. |
| `win32_normalized` | string | yes | `\\?\` normalized path. Required on the identity and on every ancestor. |
| `volume_serial` | string | yes | Windows volume serial of the opened object. |
| `file_index` | string | yes | Windows file index of the opened object. |
| `pre_image_digest` | string | for `kind == file`; not for `directory` | SHA-256 hex of the pre-image content. |

| `Hunk` field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `old_start` | integer, minimum 0 | yes | Original-file line start. |
| `old_lines` | integer, minimum 0 | yes | Original-file span length. |
| `new_start` | integer, minimum 0 | yes | Result-file line start. |
| `new_lines` | integer, minimum 0 | yes | Result-file span length. |
| `lines` | array of `HunkLine` objects | yes | Ordered structured hunk lines. |

`HunkLine` has `kind` of `context`, `add`, or `remove`, and
`text` as a string. `PatchPermissions` has `sandbox` as a
string, `write_roots` as an array of absolute-path strings,
`network` as a boolean, and `tty` as a boolean.

G4 must include and evaluate `volume_serial`, `file_index`, and
`pre_image_digest`. If it ignores those fields, F13 hardening
is inert.

## Exchanges

An allowed exec exchange:

```json
{"hook_event_name":"Exec","cwd":"C:\\work","command":"git status","decide_v1":{"version":1,"kind":"exec","exec":{"program":"C:\\Windows\\System32\\cmd.exe","argv":["/c","git status"],"cwd":"C:\\work","env":{"PATH":"C:\\managed-bin"},"sandbox":"workspace-write","network":false,"tty":false}}}
```

```json
{"decision":"ALLOW"}
```

A denied patch exchange:

```json
{"hook_event_name":"Patch","cwd":"C:\\work","path":"C:\\work\\a.txt","decide_v1":{"version":1,"kind":"patch","patch":{"paths":["C:\\work\\a.txt"],"operations":[{"path":"C:\\work\\a.txt","op":"update","hunks":[{"old_start":1,"old_lines":1,"new_start":1,"new_lines":1,"lines":[{"kind":"remove","text":"old"},{"kind":"add","text":"new"}]}],"pre_image_digest":"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"}],"permissions":{"sandbox":"workspace-write","write_roots":["C:\\work"],"network":false,"tty":false},"resolved_identities":[{"path":"C:\\work\\a.txt","kind":"file","ancestor_identities":[{"path":"C:\\work","kind":"directory","win32_normalized":"\\\\?\\C:\\work","volume_serial":"12345678","file_index":"1"}],"win32_normalized":"\\\\?\\C:\\work\\a.txt","volume_serial":"12345678","file_index":"42","pre_image_digest":"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"}]}}}
```

```json
{"decision":"DENY","reason":"outside-approved-scope"}
```

The latter response, any response other than the exact ALLOW
object, and any transport failure deny with zero intended side
effect. The fork may retain a redacted reason code, never raw
command text, patch body, environment secret, or sensitive path
by default.

## Version policy

Version changes are coordinated with Covenant. A new version
requires a new schema document, paired fork and harness support,
fixture coverage for both allow and denial, and a newly recorded
inventory digest. Version 1 remains the only accepted version
until that coordinated release exists; neither side may silently
coerce or downgrade an unknown version.
