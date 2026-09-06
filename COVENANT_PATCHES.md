# Covenant patch index — draft

Adapted from the [canonical harness patch index](https://github.com/ChristNata/Covenant-Harness/blob/b6e933a4590a2ef848755c4a593a7e9e8f2072d4/docs/master-plans/cross/codex-fork/repo-context/COVENANT_PATCHES.md).
Paths and boundaries incorporate the [approved fork decisions](covenant/IMPLEMENTATION.md);
this is not a byte copy.

This is an index of accepted components and pending integration, **not the
complete certified patch surface**, a runtime certificate or a release manifest.
Final hunk hashes do not exist yet. [UPSTREAM.toml](covenant/UPSTREAM.toml) records
the baseline; the ledger owns stage evidence and checkpoint identities.

| Phase | Reviewed local portion | Remaining acceptance |
| --- | --- | --- |
| F11 | Identity policy and router/parallel/registry/streaming guards | Final product, effective registry/effects and semantic audit |
| F12 | Standalone launch/env/envelope/reply/command-line components | Native image/Job/process ownership, final backend gates and real G4 |
| F13 | Design and proposed patch seam | Qualified guarded mutation and add/update/delete/move race coverage |
| F14 | Auth-home routing, refresh ownership and file replacement | Complete route/sink review and harness-home acceptance |
| F21 | Feature/config/role/metadata/MCP projection stages and catalog data | Remaining lifecycle clamps, provider/catalog/CLI and product integration |
| F22 | Planned certificate contract | Actual inventory command, effective-state evidence and mismatch refusal |

## F11 — tool identity admission

The compiled policy lives in [codex-tools/covenant_admission.rs](codex-rs/tools/src/covenant_admission.rs),
with exports/error wiring in that crate's `lib.rs` and `function_call_error.rs`.
Core integrates it through [router.rs](codex-rs/core/src/tools/router.rs),
[parallel.rs](codex-rs/core/src/tools/parallel.rs),
[registry.rs](codex-rs/core/src/tools/registry.rs) and
[stream_events_utils.rs](codex-rs/core/src/stream_events_utils.rs).

The two identities are unqualified function `exec_command` and custom
`apply_patch`. Classification grants no spawn/patch permit. Other covered
identity/form/namespace combinations return terminal `CovenantDenied` before
lookup, readiness, callbacks, streaming consumers or handlers. Constrained
pre-hook payload creation/rewriting cannot become an alternate authority.

Existing focused evidence is in [registry tests](codex-rs/core/src/tools/covenant_admission_tests.rs),
[raw-wire tests](codex-rs/core/src/tools/covenant_wire_admission_tests.rs) and
[readiness tests](codex-rs/core/src/tools/covenant_readiness_admission_tests.rs).
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
is the proposed seam. The constrained guarded batch path is not implemented;
this row claims no changed patch-runtime hunk or completed test matrix.

Authorization must retain operations, permissions, source/destination paths,
ancestor/target identities and pre-images through committed mutation. A preflight
check followed by the ordinary text-reparsing writer is insufficient. Settled
denial/effective precommit races must leave zero committed delta. Qualification
must refuse unsupported APIs/volumes without an ordinary-writer fallback.
Updates-only work cannot complete add/update/delete/move coverage.

Cancellation before commit admission rolls back. After owner-admitted commit,
settle the actual result without false denial or retrying an unknown outcome.
Native whole-batch/race tests and real G4 discrimination remain required.

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
are reviewed data; extraction alone does not prove catalog immutability.

Remaining work covers provider/catalog construction and reload closure,
startup/session/connection effects, effective specs and the constrained CLI/
release graph. Active CLI/catalog changes are not promoted by this document.
Final Cargo/lock/Bazel integration must preserve approved external pins; the
temporary verification worktree is not the shipping graph.

The final reviewed CLI contract defines exec, native login and inventory access.
The published artifact must exclude public TUI/server/MCP/plugin/Code Mode,
hosted web, dynamic-tool, multi-agent and interactive-input authorities. Retain
upstream exec's required internal app-server library while removing alternate
effects. Each constructor/effect needs inventory and tests; flags alone do not
prove exclusion.

## F22 — runtime inventory and certificate

The planned `codex --covenant-inventory` command and proposed
`codex-rs/cli/src/covenant_inventory.rs` are not implemented or certified here.
Inventory must derive binary/profile digests, inventory ID and required
schema/source/feature evidence from the effective executable. An unclassified
identity or registry mismatch must make inventory nonzero and prevent exec
startup. A static catalog/table fixture is not this certificate.

## Supporting files and preserved scope

| Area | Support and boundary |
| --- | --- |
| F00/F01 | [UPSTREAM](covenant/UPSTREAM.toml), [integration map](covenant/INTEGRATION-FILES.toml), [authority inventory](covenant/AUTHORITY-INVENTORY.toml), their Python validators/tests. Structural validation is not semantic completeness. |
| F10 | [Schema](covenant/schema/decide-v1.json), runtime `decide.rs`, `wire.rs`, `values.rs`, `numbers.rs`, `lib.rs`, wire tests, manifests/lock/toolchain/nextest configuration. |
| F02/F03 | [Windows recipe](covenant/windows-repro.toml), build/MSVC/pair helpers and Python tests, build-output ignore and [artifact workflow](.github/workflows/covenant-release.yml). No product-build/release certification. |
| Compilation/tests | Reviewed core/login manifests, module/test registrations and isolated runtime/login workspaces. Final product dependency/Bazel wiring remains pending. |
| F40 | Eight context files, README map, [integration pointer](FORK_INTEGRATION.md) and [ledger](covenant/IMPLEMENTATION.md), with explicit canonical-source adaptations. |
| Existing user scope | Preserve [sync-stable.yml](.github/workflows/sync-stable.yml), integration rules and the local covenant_docs input pack; the latter is not an emitted release asset. |
| Pending audit/release | F31 semantic audit, final F22 evidence and F33 promotion/provenance/attestation are not implemented by the artifact workflow. F32 adoption remains external. |

## Finalization and rebase

Freeze the candidate tree before calling this index complete. Compare every
intentional source/support change against the recorded upstream commit; account
for all paths, symbols, tests and authority classifications, including support
files outside a single gate row. Resolve moved seams through a reviewed
amendment and preserve the [actual upgrade procedure](FORK_INTEGRATION.md).

F31 must prove semantic gate domination and effect closure on that exact tree,
including discriminating negative mutations. F33 records its hunk hashes and
release evidence afterward. No placeholder digest is a certificate.
