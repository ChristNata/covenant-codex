# Covenant patch index — draft

Adapted from the [canonical harness patch index](https://github.com/ChristNata/Covenant-Harness/blob/b6e933a4590a2ef848755c4a593a7e9e8f2072d4/docs/master-plans/cross/codex-fork/repo-context/COVENANT_PATCHES.md).
Paths and boundaries incorporate the [approved fork decisions](covenant/IMPLEMENTATION.md);
this is not a byte copy.

This is an index of accepted components and pending integration, **not the
complete certified patch surface**, a runtime certificate or a release manifest.
[UPSTREAM.toml](covenant/UPSTREAM.toml) records the baseline; the ledger owns
stage evidence and checkpoint identities.

## F33 frozen hunk hashes

These SHA-256 values are hashes of the binary `git diff --binary` output for
each phase's fork-owned source paths against the pinned baseline in
`UPSTREAM.toml`. They bind the promoted patch index to the reviewed candidate
tree.

F11 hunk_sha256 = "cdb48c06a442fb344dcca0e2ec75c63cf483ce344790fc598eea5eba7933222f"
F12 hunk_sha256 = "33b70e3c99f90330b879eb270facbf3a761901307845376f8dfc4c12ecf3c8c4"
F13 hunk_sha256 = "538deff91e84dc68e1331b533902a0aefc0511278344b44bc8789c9d41029eb1"
F14 hunk_sha256 = "0e877776d9070f19b03eb8e71ff2b26e089c0483fe4195eddf61bb85c8b2abc2"

The v0.1.8 F12 hash covers these production source paths:

- `codex-rs/core/src/tools/runtimes/covenant_exec_gate.rs`
- `codex-rs/core/src/tools/runtimes/mod.rs`
- `codex-rs/core/src/tools/runtimes/unified_exec.rs`
- `covenant/runtime/src/decision_reply.rs`
- `covenant/runtime/src/exec_envelope.rs`
- `covenant/runtime/src/launch_contract.rs`
- `covenant/runtime/src/lib.rs`
- `covenant/runtime/src/secret_policy.rs`
- `covenant/runtime/src/sidecar.rs`
- `covenant/runtime/src/windows_command_line.rs`
- `covenant/runtime/src/windows_environment.rs`

The v0.1.8 F13 hash covers these production source paths:

- `codex-rs/core/src/tools/runtimes/apply_patch.rs`
- `codex-rs/core/src/tools/runtimes/covenant_patch_gate.rs`

### v0.1.8 compatibility correction

The historical promoted sidecar fixture remains Option-B evidence. The v0.1.8
candidate corrects the fork's current-Harness request drift: native absolute
paths, the worktree-only `workspace-write` root, disabled patch network/TTY,
decimal-string Win32 identities, complete ancestor identities, and matching
pre-image digests. It also accepts the decider's optional `remediation` field so
a valid DENY is not mislabeled as malformed. Current managed-Harness add/update/delete/move
ALLOW and missing-session-mode DENY are recorded in
[`v0.1.8-compatibility-evidence.json`](covenant/v0.1.8-compatibility-evidence.json).

| Phase | Reviewed local portion | Remaining acceptance |
| --- | --- | --- |
| F11 | Exact five-identity policy and router/parallel/registry/streaming guards | Final fanin registry/effects and semantic audit |
| F12 | Standalone launch/env/envelope/reply/command-line components | Native image/Job/process ownership, final backend gates and real G4 |
| F13 | Guarded patch seam with current-Harness-compatible native request and fail-closed real-decider call | Add/update/delete/move compatibility and remediated DENY are cross-boundary green; post-decision identity/race qualification remains tracked under H3 |
| F14 | Auth-home routing, refresh ownership and file replacement | Complete route/sink review and harness-home acceptance |
| F21 | Constrained profile plus launcher-bound fanin-only MCP projection/catalog and bounded tool exposure | Local two-upstream allow/deny E2E is green; Harness fail-closed lookup and cancellation verification remain F32 handoff, while product/Bazel and Windows release re-audit remain fork gates |
| F22 | Runtime certificate with five compiled identity/form rows | Final executable/inventory equivalence and fanin mismatch refusal |

## F11 — tool identity admission

The compiled policy lives in [codex-tools/covenant_admission.rs](codex-rs/tools/src/covenant_admission.rs),
with exports/error wiring in that crate's `lib.rs` and `function_call_error.rs`.
Core integrates it through [router.rs](codex-rs/core/src/tools/router.rs),
[parallel.rs](codex-rs/core/src/tools/parallel.rs),
[registry.rs](codex-rs/core/src/tools/registry.rs) and
[stream_events_utils.rs](codex-rs/core/src/stream_events_utils.rs).

The identities are unqualified function `exec_command`, custom `apply_patch`,
and exactly `mcp__fanin` function `list_tools`, `get_tool_schema`, and
`invoke_tool`. Classification grants no spawn, patch, or upstream-effect permit.
The fanin gateway is bound to the managed launcher installation and its
Harness-selected namespace separately. Other covered
identity/form/namespace combinations return terminal `CovenantDenied` before
lookup, readiness, callbacks, streaming consumers or handlers. Constrained
pre-hook payload creation/rewriting cannot become an alternate authority.

Existing focused evidence is in [registry tests](codex-rs/core/src/tools/covenant_admission_tests.rs),
[raw-wire tests](codex-rs/core/src/tools/covenant_wire_admission_tests.rs) and
[readiness tests](codex-rs/core/src/tools/covenant_readiness_admission_tests.rs).
The fanin raw-wire positive/negative cases are in the same wire suite.
The ledger records accepted runs and limitations. F31 must still prove the final
effective registry and effect closure; these tests do not certify all startup routes.

## F12 — final exec decision

[covenant/runtime](covenant/runtime/Cargo.toml) owns private `launch_contract.rs`,
`windows_environment.rs`, `secret_policy.rs`, `exec_envelope.rs`,
`decision_reply.rs` and `windows_command_line.rs`, with dedicated sibling tests.
These reviewed components freeze paired JSON/native facts and bound transport.
They do not spawn a protected child or prove final backend environment equality.

[process_manager.rs](codex-rs/core/src/unified_exec/process_manager.rs) remains
the planned routing/origin seam. The final decision must dominate prepared local
Windows creation, with separate plain/legacy/elevated coverage. Remote/foreign
and snapshot execution must refuse before dispatch. Preserve sandbox choice.

Opened-file hashing, guarded non-reparse namespace, suspended-image identity,
no-breakaway Job ownership and kill/reap before settlement still need native
integration/evidence. Trusted fixed preparation is inventoried separately;
denial permits no model payload execution. Accept no ALLOW after 1000 ms;
cleanup has no hard-real-time guarantee. Final F31 and real G4 must discriminate
complete env/argv/security facts and failure effects. Component green is insufficient.

## F13 — final patch decision

`ApplyPatchRuntime::run` in [apply_patch.rs](codex-rs/core/src/tools/runtimes/apply_patch.rs)
is the guarded seam. The fork freezes the parsed request, calls the Covenant
decider, and commits only after an exact `ALLOW`; malformed, missing,
unavailable, or denied decisions leave the target unchanged.

Authorization must retain operations, permissions, source/destination paths,
ancestor/target identities and pre-images through committed mutation. A preflight
check followed by the ordinary text-reparsing writer is insufficient. Settled
denial/effective precommit races must leave zero committed delta. Qualification
must refuse unsupported APIs/volumes without an ordinary-writer fallback.
The current managed Harness accepts the candidate's bounded native add, update,
delete, and move requests when the launcher supplies a valid session mode. The fork now
uses native absolute paths, the current worktree as its sole write root,
`workspace-write` with network/TTY disabled, decimal-string Win32 object
identities, complete ancestor identities, and matching SHA-256 pre-images.
Adversarial replacement/race qualification remains distinct F13 acceptance
work; the compatibility result does not claim those cases.

Cancellation before commit admission rolls back. After owner-admitted commit,
settle the actual result without false denial or retrying an unknown outcome.
The historical real-decider DENY path is covered by E01. The current managed
decider add/update/delete/move ALLOW and remediated DENY compatibility cases are recorded in
the v0.1.8 evidence overlay; the remaining discriminating G4 race pairs are not
claimed by that overlay.

## F14 — native auth ownership

The actual owner is [login/src/auth](codex-rs/login/src/auth/mod.rs), including
`storage.rs`, `manager.rs`, `covenant_auth_home.rs`, `covenant_auth_storage.rs`,
`covenant_auth_refresh.rs` and `covenant_auth_file.rs`. The old `core/src/auth.rs`
plan path is absent on this baseline.

Accepted work covers CODEX_AUTH_HOME routing with unset fallback, native refresh
serialization/ownership across caller cancellation and file replacement/failure
preservation. It does not establish every keyring, API-key, browser/device,
logout/revoke or retained-sink guarantee. Credentials stay in Codex's native
flow; Covenant must not copy them into mutable CODEX_HOME.

[covenant_auth.rs](codex-rs/login/tests/covenant_auth.rs) and its rotation,
cancellation and persistence modules run through the isolated
[login-tests workspace](covenant/login-tests/Cargo.toml). The ledger records the
preserved pins and accepted assertions. Complete route/sink coverage and real
harness home separation remain release prerequisites.

## F21 — constrained packaging and profile

Reviewed owners under [core/config](codex-rs/core/src/config/mod.rs) include
`managed_features.rs`, `covenant_profile.rs`, `covenant_non_features.rs` and
`mod.rs`, plus [mcp.rs](codex-rs/core/src/mcp.rs), with dedicated profile/config/
role/MCP tests. The four [model-catalog.json](covenant/model-catalog.json) records
remain reviewed, bounded data for the explicit `--bundled` diagnostic only.
They no longer gate headless model selection or act as the default runtime catalog.

The fanin-only exception keeps one native local stdio client from the final
config catalog. [covenant_fanin.rs](codex-rs/core/src/config/covenant_fanin.rs)
checks the existing managed decider/marker controls, binds the gateway and
generated config as launcher siblings through
[fanin_binding.rs](covenant/runtime/src/fanin_binding.rs), substitutes only the
validated namespace, and freezes the final server map. Higher-priority managed
MCP requirements may still disable it. The direct router exposes only the
gateway's three bounded meta-tools; missing or oversized specs refuse the plan.
The clamp assigns `approve` only to that frozen three-tool server. This is the
narrow exception required for managed `workspace-write` when the global policy
is `never`; an `auto` control reproduces the denial and the `approve` case passes
the two-upstream live turn. It does not admit another MCP server or tool.
MCP client elicitation is disabled in this headless profile. No public MCP CLI,
other server, resource, app, plugin, HTTP MCP, or executor-owned MCP path is
admitted. The Windows two-upstream E2E and exact executable release audit are
required evidence, not inferred from component tests.

The reviewed CLI contract defines exec, native login, inventory access and the
diagnostic `codex debug models` catalog query. Covenant exposes no other debug
leaf. The default query and headless `codex exec` admission require a fresh,
authenticated ChatGPT/Codex backend `/models` response; no stale cache or
bundled catalog can authorize an ID. Only exact list-visible slugs are admitted,
after a bounded unique-ID and native two-tool-capability check, and exec binds
the admitting metadata snapshot to its internal app-server.
The fork keeps the direct native exec/patch router, with only the optional
fanin namespace added, even when live model metadata advertises
`code_mode_only`; that upstream selector grants no alternate tool authority.
OpenAI API-key catalog selection is not supported. Explicit `--bundled` reports
the pinned four-model diagnostic catalog.
The published artifact must exclude public TUI/server/arbitrary MCP/plugin/Code Mode,
hosted web, dynamic-tool, multi-agent and interactive-input authorities. Retain
upstream exec's required internal app-server library while removing alternate
effects. Each constructor/effect needs inventory and tests; flags alone do not
prove exclusion.

## F22 — runtime inventory and certificate

`codex --covenant-inventory` in
[covenant_inventory.rs](codex-rs/cli/src/covenant_inventory.rs) emits the
executable digest, inventory ID and compiled identity/form evidence. Its five
rows include the exact `mcp__fanin` namespace/function identities; a mismatch
refuses the certificate. The release must compare this against the final
model-visible registry and gateway E2E, since a static identity table alone
cannot prove live server behavior.

## Supporting files and preserved scope

| Area | Support and boundary |
| --- | --- |
| F00/F01 | [UPSTREAM](covenant/UPSTREAM.toml), [integration map](covenant/INTEGRATION-FILES.toml), [authority inventory](covenant/AUTHORITY-INVENTORY.toml), their Python validators/tests. Structural validation is not semantic completeness. |
| F10 | [Schema](covenant/schema/decide-v1.json), runtime `decide.rs`, `wire.rs`, `values.rs`, `numbers.rs`, `lib.rs`, wire tests, manifests/lock/toolchain/nextest configuration. |
| F02/F03 | [Windows recipe](covenant/windows-repro.toml), build/MSVC/pair helpers and Python tests, build-output ignore and [artifact workflow](.github/workflows/covenant-release.yml). No product-build/release certification. |
| Compilation/tests | Reviewed core/login manifests, module/test registrations and isolated runtime/login workspaces. Final product dependency/Bazel wiring remains pending. |
| F40 | Eight context files, README map, [integration pointer](FORK_INTEGRATION.md) and [ledger](covenant/IMPLEMENTATION.md), with explicit canonical-source adaptations. |
| Existing user scope | Preserve [sync-stable.yml](.github/workflows/sync-stable.yml), integration rules and the local covenant_docs input pack; the latter is not an emitted release asset. |
| Pending audit/release | F32 adoption remains external. Current-Harness Patch add/update/delete/move compatibility is green; F13 race qualification and hosted Windows release evidence remain separate gates. |

## Finalization and rebase

Freeze the candidate tree before calling this index complete. Compare every
intentional source/support change against the recorded upstream commit; account
for all paths, symbols, tests and authority classifications, including support
files outside a single gate row. Resolve moved seams through a reviewed
amendment and preserve the [actual upgrade procedure](FORK_INTEGRATION.md).

F31 must prove semantic gate domination and effect closure on that exact tree,
including discriminating negative mutations. F33 records its hunk hashes and
release evidence afterward. No placeholder digest is a certificate.
