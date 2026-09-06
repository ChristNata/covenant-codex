# Covenant fork implementation ledger

Planning checkpoint: 2026-09-06. Source contract: `covenant_docs/master.md`,
its schema, review history, tasks, and repository-context pack. This ledger
records adaptations to the existing checkout; it does not certify an
implementation. All 15 phases below remain uncompleted at this checkpoint.

Subsequent accepted local state: F01 is complete after its focused 17-test suite,
independent correction re-review, and root adjudication. F00 source inventory
and structural validation are complete after 15 tests, actual input validation,
and final independent source-review PASS; runtime enforcement remains pending.
F10's standalone local
wire-contract artifact stage is also complete after 16 focused tests and
independent review; its external schema/sidecar and source/Bazel integration
acceptance remain pending. Checkpoint hashes and exact staged scope are
recorded below. Other phases retain their separately documented in-progress or
external-pending status.

## Session constraints and evidence

- The user authorizes the planned fork work and workers for planning, tests,
  implementation, and review. Planned upstream integration-seam edits are in
  scope; unrelated upstream repairs and unrelated upstream tests are not.
- Keep this checkout and branch. Do not upgrade the upstream baseline, rename
  branches, replace existing custom automation, edit another repository, or
  publish intermediate work. No remote push until all locally achievable
  implementation, tests, and review checkpoints are satisfied. Never represent
  blocked external acceptance as a completed phase to permit a push.
- At inspection, branch/default branch was `covenant-ver`, HEAD was
  `a376161390905484ec8077a520aaf80cee27c9cc`, origin was public fork
  `https://github.com/ChristNata/covenant-codex`, and `covenant_docs/` was
  untracked. Preserve those documents and pre-existing changes.
- `origin/main` and `git merge-base HEAD origin/main` both resolved to
  `3d2ee51ca2d5db578f328aa75e20aa22c0197c9a`. Read-only GitHub API resolution of
  upstream annotated `rust-v0.153.4` tag object
  `042fb41b7c813ac7999105e886b2b7aa715b5081` resolved to that exact commit.
  Local tags were absent. No fetch, rebase, branch change, or push was needed.
- The existing upstream delta comprises `.github/workflows/sync-stable.yml`
  and `FORK_INTEGRATION.md` (190 added lines), rather than only the workflow.
  There were no pre-existing Rust changes relative to that upstream parent.
- `codex-rs/rust-toolchain.toml` pins Rust `1.95.0` with clippy/rustfmt/rust-src.
  LICENSE identifies Apache-2.0 and NOTICE includes upstream attribution;
  preserve their bytes. Toolchain installation/build readiness is not certified
  by reading these files.

## Explicit plan amendments

| ID | Adaptation | Reason and boundary |
| --- | --- | --- |
| A01 | Map `covenant-capital/codex` to `ChristNata/covenant-codex`, and behavioral branch `covenant` to existing `covenant-ver`. | Existing user-owned topology and `FORK_INTEGRATION.md` govern. Keep `main` clean and untouched. URLs, workflow guards, and contributor instructions must use the actual repository. |
| A02 | F01 records the verified existing `rust-v0.153.4` parent; it does not adopt a newer release. | User scope forbids unrelated upstream updates. Record current lineage honestly; the first historical Covenant commit cannot be retroactively replaced with F01. |
| A03 | F00/F14 re-anchor auth from absent `core/src/auth.rs` to `codex-rs/login/src/auth/`. | Scout found `mod.rs`, `manager.rs`, and `storage.rs`. Root approved this bounded amendment; freeze the actual centralized load/write/refresh paths before F14 edits. Do not claim the master auth seam survived unchanged. |
| A04 | Recover missing local context `CLAUDE.md` from sibling harness canonical pack, then explicitly align published context with A01-A03 and current user constraints. | All seven local pack files hash-identical to the corresponding sibling files. The eighth canonical file exists at `../Covenant-Harness/docs/master-plans/cross/codex-fork/repo-context/CLAUDE.md`; do not invent it. Preserve source attribution and distinguish recovered source bytes from amended published documents. Exact byte-copy criteria require this recorded amendment wherever naming/seams/instructions change. |
| A05 | Separate local phase implementation from external/remote phase acceptance. | User prohibits intermediate pushes. F03's live dispatch, F33's real audited release/attestation, and F32's external harness pin cannot be manufactured locally. Prepare implementations and local evidence; keep external acceptance pending until the required inputs/authorization exist. |
| A06 | Add the minimum module declarations, dedicated tests, Cargo/Bazel source-data wiring, and regeneration outputs required by approved integration changes. | Master `Produces` omits routine compilation/test registration files. Integrator owns these shared edits and records exact paths before accepting a checkpoint. This does not authorize unrelated feature work or new executor paths. |
| A07 | F01 uses an offline bootstrap validator and strictly resolves the recorded local tag to the pinned commit. | Root authorizes the implementation worker to fetch only already-verified `rust-v0.153.4` from `https://github.com/openai/codex.git` into that local tag name to supply verification evidence. No branch/checkout/baseline change; no validator network access or missing-tag bypass. |
| A08 | F10 includes a small standalone `covenant/runtime` package/workspace named `codex-covenant` with its own lockfile and nextest local profile. | Root approved isolated wire-contract tests/implementation without altering the drifting upstream dependency baseline. The current stage exposes only decode/serialization; later integration still requires reviewed codex-rs path-dependency/Bazel wiring. JSON Schema integer acceptance must not silently narrow to u64; duplicate-key rejection is additional transport policy. |
| A09 | Format only fork-owned or intentionally changed files; restore only proven CRLF-only checkout differences in F01's three pinned artifacts. | The user's no-unrelated-upstream-change constraint overrides whole-tree formatting. Root specifically authorized exact pinned-blob restoration of LICENSE, NOTICE, and rust-toolchain.toml only after verifying no existing content diff and CRLF normalization equality, plus exact-path index stat refresh with no staged content. No general upstream repair or formatter sweep is authorized. |
| A10 | F21 preserves the internal in-process app-server orchestration required by this pinned `codex exec`, while removing public server entrypoints and alternate authorities. | Root approved the source re-anchor: `exec/src/lib.rs:812` calls app-server-client `lib.rs:330`, then `in_process.rs:473` creates MessageProcessor with plugin startup at line 490; `message_processor.rs:331` constructs `extensions.rs:75-133`. Do not claim the entire app-server library compiles out or rewrite the upstream agent loop. Exact extension/startup constructor clamps require their own F21 design and behavioral tests before edits. |
| A11 | F00 records the fourth upstream wire form `tool_search` separately from function/custom/hosted. | Root approved the source-evidenced inventory form, with one positive synthetic identity regression before implementation. `ToolSpec::ToolSearch` and `ToolPayload::ToolSearch` remain proposed excluded; F11's exact two-identity admission table is unchanged. |
| A12 | F12 authorizes at the final prepared backend boundary; manager remains the mandatory routing/origin gate. The first constrained product supports local Windows exec only. | Root approved fork-only opt-in seams because legacy/elevated code changes env/cwd/security after the manager. Remote/foreign executor and snapshot routes must explicitly deny before dispatch, with behavioral coverage. Preserve sandbox selection and never fall back unsandboxed. Plain, legacy and elevated are separately required stages. |
| A13 | F12 uses a fork-owned no-breakaway Windows Job and kills/reaps remaining members before tool settlement. | Root approves Job-descendant containment, not a claim that WMI/service/brokered effects are contained. Such effects require policy/residual treatment. Preserve ordinary upstream process behavior outside managed invocations. Native tests must prove containment and cleanup ordering, not just a plain adapter return value. |
| A14 | F12 decider identity uses an opened-file hash, guarded immutable namespace and suspended-image path/file identity proof from a local non-reparse installation. | Root accepts this explicit construction proof; QueryFullProcessImageNameW is a name, not a direct mapped-image handle. Installation/dependency trust assumptions and replacement tests remain required. Do not claim a kernel mapped-image-handle comparison. |
| A15 | F12's 1000 ms limit is an authorization deadline with fully owned cleanup before settlement. | No ALLOW after the deadline; no hard-real-time promise that creation/I/O/cleanup finishes in exactly 1000 ms. Concrete transport/request/resource limits still need their test-first design and cannot silently narrow F10's schema. |
| A16 | Fixed trusted sandbox preparation may precede model authorization and is inventoried separately. | Root approves necessary existing token/ACL/proxy/cwd/helper preparation, with zero model payload execution on denial. Do not claim zero infrastructure filesystem effects or classify trusted preparation as the model patch. Expanded backend seams require their bounded source/test ownership before implementation. |

The one-shot path is compatible with the planned exec gate on this baseline:
`Feature::UnifiedExec = false` selects `ExecCommandHandler::one_shot`, which
uses `exec_command_to_completion` and shared process-manager startup. This
removes `write_stdin` without adding an executor or changing the agent loop.

## Phase ownership, decomposition, and acceptance

Status vocabulary: `planned`, `tests-red`, `implemented`, `local-verified`,
`external-pending`, `reviewed`, `complete`. Record command/output evidence and
commit/tree identity when moving status. Passing a stub test is not real-sidecar
acceptance. A skipped, unavailable, or not-yet-authored test is not a pass.

| Phase | Dependencies | Owner and permitted implementation area | Work and checkpoint |
| --- | --- | --- | --- |
| F01 | none | bootstrap implementation: `covenant/UPSTREAM.toml`, `covenant/scripts/validate_bootstrap.py`; separate test owner: `covenant/tests/test_bootstrap.py` | Record A01/A02 repository, exact tag/commit, upstream URL, license, existing toolchain path. Verify ancestry and unchanged LICENSE/NOTICE using existing Git objects; behavioral fixture tests prove invalid/drifted bootstrap state is rejected. No static-value unit tests or F00/F31 inventory validation in this phase. |
| F00 | F01 | scout: `covenant/INTEGRATION-FILES.toml`, `covenant/AUTHORITY-INVENTORY.toml` | Enumerate every relevant constructor, identity, process/filesystem/network/hosted/MCP/hook/generated-code sink, source anchor/disposition, provider/login secret key or closed prefix set, and executable review procedure. Bind both to F01 commit. Resolve A03 auth paths and inventory every newly discovered authority before downstream implementation. Human completeness review is mandatory. |
| F02 | F01 | build: `covenant/windows-repro.toml`; new build helper only if necessary | Record Windows MSVC target, existing Rust/Cargo identity, actual package/bin invocation, windows-2022 runner and SDK/MSVC recipe, no Node shim, SHA-256 method, byte or digest-recorded reproducibility. Verify an actual produced exe before build acceptance; no two-build ritual. |
| F03 | F02 | release: `.github/workflows/covenant-release.yml`, narrowly scoped release helpers/tests | Implement manual artifact-only build/hash/upload with every `uses:` pinned to an exact SHA. Validate artifact refusal behavior locally; live workflow_dispatch acceptance remains external-pending before first permitted push. Never publish from this path. |
| F10 | F01 | schema/runtime: `covenant/schema/decide-v1.json`, isolated `covenant/runtime/` per A08; preserve existing `covenant_docs/schema/decide-v1.json` | Preserve byte identity of shared schema; implement/evaluate schema semantics for complete env, argv array, patch operations/identities, conditional digest/target requirements. Dedicated Rust corpus compares runtime acceptance against a real Draft 2020-12 validator. Harness-side schema synchronization remains external-pending. |
| F11 | F00, F02 | admission: dedicated Covenant admission module, `core/src/tools/registry.rs`, dedicated admission/agent fixtures | Closed exact `exec_command` function + custom `apply_patch` table. Unknown/namespace/read/none/writer identities deny terminally before handlers; bypass ordinary pre-tool hooks after admission. Hostile config/env cannot extend table. Complete deterministic repository-read turn with only admitted tools before table acceptance. |
| F12 | F00, F02, F10 | exec: dedicated Covenant gate/client modules, `core/src/unified_exec/process_manager.rs`, `covenant/sidecar-fixture.toml`, dedicated exec fixtures | Immutable startup launch contract; opened-handle image hash/no-replace protection; exact ALLOW/deadline/resource settlement; one Windows env block shared by decision and spawned process with case-insensitive closed-key scrub; auth argv protection; zero surviving descendants at tool-result settlement. Keep real-sidecar acceptance external-pending until E01. |
| F13 | F12 | patch: `core/src/tools/runtimes/apply_patch.rs`, shared gate changes serialized after F12, dedicated patch fixtures | Freeze parsed operations, permissions and resolved Windows ancestor/target identities; authorize before any mutation; retain/recheck identity and pre-image through writes. Denial and replacement/junction/reparse races must leave zero committed delta. Real-sidecar independent volume/file/digest discrimination requires E01. |
| F14 | F00, F02, A03 frozen map | auth: `codex-rs/login/src/auth/` exact scout-selected I/O/refresh files, dedicated auth-home module/tests | Codex-owned auth root with upstream unset fallback; refresh serialization and atomic replacement preserve newest valid generation on concurrent rotation/failure/cancel; no secret retention; mutable-home deletion does not remove login. Do not edit unrelated auth behavior. Real harness home separation additionally requires E02. |
| F21 | F00, F02; integrate F11 before effective-registry acceptance | packaging: `codex-rs/Cargo.toml`, CLI/Cargo dependency wiring as explicitly reviewed, `features/src/lib.rs`, `cli/src/main.rs`, `core/src/config/mod.rs`, `core/src/tools/spec_plan.rs`, `covenant/model-catalog.json` | Smallest compile/profile mechanism that excludes prohibited executable surfaces and hook/MCP startup effects. Clamp every config/CLI/model layer and pinned catalog fail-closed. Avoid mass implementation deletion. Any unavoidable extra reachable hook/MCP source edit requires recorded scout amendment before code. |
| F22 | F11, F21 | inventory: `cli/src/covenant_inventory.rs`, `cli/src/main.rs`, dedicated CLI fixtures | Emit runtime PinCertificate fields/evidence from effective state. Real registry mismatch makes inventory nonzero and exec refuse startup. Verify executable digest and observed tool/features agree; never label a build-time fixture as runtime certificate. |
| F31 | F00, F11, F12, F13, F14, F22, F40 | audit: `.github/workflows/covenant-reaudit.yml`, `covenant/tests/sink_instrumentation.rs`, minimal test/build wiring | Check commit-bound inventory, every gate placement, effective admission effects and packaging; execute negative mutations per inventoried family including alias/wrapper/direct/generated/hosted routes. Ungated golden fixtures must fail and unchanged patched tree must pass. Static symbol grep alone cannot certify semantic domination/completeness. |
| F33 | F03, F31, F22; E01 green | release, exclusive reuse of F03 workflow and `COVENANT_PATCHES.md` after F40 | Prepare promote job: record hunk hashes, immutable commit, re-audit that tree, pinned Windows build/hash, real inventory, provenance and build attestation, five release assets. Manual artifact job stays nonpublishing. No live release without actual green prerequisites and final remote authorization. |
| F32 | F33; E01, E02, E03 | external harness handoff; no external file edits authorized in this session | Prepare exact adoption evidence/instructions only in this fork. Actual `backend/covenant-utils/src/pinned_versions.rs` row requires landed S2, four-field source/version/url/sha256, verified real release attestation and separation tests. No fake pin, parallel constant, extra fields, Actions-artifact URL, or local claim of harness adoption. |
| F40 | F01; A01-A04 | context: eight published context files, upstream README pointer, narrowly necessary fork integration index update | Recover eighth source, align actual topology/auth seams, preserve applicable upstream/user instructions and upstream README content. Publish complete patch index for F11-F14/F21/F22. Record authored amendments; never claim all copies remained byte-identical if edited. No static-copy tests; inspect links/content and use direct hash evidence where byte identity is actually required. |

## Test-first work packages

Use a test worker before each behavioral implementation worker, then a separate
review worker. A root-assigned integration worker integrates shared files;
the root adjudicates scope and evidence without performing code integration.
First run the newly introduced
behavioral test against the pre-implementation state and record the meaningful
failure. A compile error caused only by an uncreated module is insufficient
behavioral evidence; prefer exercising an existing binary/public seam with an
observable wrong result or a deliberately failing behavioral fixture. Static
metadata/document changes use direct review/validation, not mirroring tests.

Selectors below are reserved test names to implement, not claims they exist.
All Rust test runs use `just test`, never `cargo test` or the complete upstream
suite. Run from `codex-rs`; record the actual final selector after registration.

| Package | Behavioral red/green evidence | Narrow selector |
| --- | --- | --- |
| Schema/client contract | Invalid version/branch/payload rejection; conditional patch digests/targets; exact decision parsing, bounded response/deadline and malformed/extra-key rejection. | Dedicated Covenant crate/module selector established with implementation; `just test -p codex-core -E 'test(covenant_gate)'` if client remains in core. |
| Admission | Unknown/namespaced/wrong-form/denied identities never invoke handler; admitted operations bypass hook; real fixture-provider repository-read turn completes. | `just test -p codex-core -E 'test(covenant_admission)'` |
| Exec | Every failure path settles with zero command descendants and no runner tasks/handles; executable-swap attacks; actual descendant env equals authorized scrubbed block; background child settlement. | `just test -p codex-core -E 'test(covenant_exec)'` |
| Patch | Add/update/delete/move complete-object outcomes; denied multipath patch commits nothing; target/ancestor/junction/file-id/pre-image races cannot escape authorization. | `just test -p codex-core -E 'test(covenant_patch)'` |
| Auth | Distinct-home I/O and unset fallback; cancellation/failure atomicity; rotating-token concurrent refresh selects newest authorized generation; no sentinel leaks. | `just test -p codex-login -E 'test(covenant_auth)'` (confirm actual package name with F00). |
| Packaging | CLI excluded-entry refusal; hostile user/env/profile/-c/--enable/model overrides; fixture request has no excluded tools; hook/MCP markers never execute. | `just test -p codex-cli -E 'test(covenant_packaging)'` and only specifically named core integration fixtures required for outbound capture. |
| Inventory | Effective runtime certificate from produced exe; injected unknown registry identity blocks inventory and exec. | `just test -p codex-cli -E 'test(covenant_inventory)'` |
| Real sidecar | Complete-env pair using a runtime-selected key; independent volume_serial/file_index/pre_image_digest pairs; auth-home Read pair; display string cannot override decide_v1. | Dedicated `covenant_sidecar` selector; must fail explicitly when E01 is missing in certification mode, never silently skip green. |
| Re-audit | Unchanged patched tree passes; gate removal/relocation, commit mismatch, effect drift and each ungated golden mutation fail. | Dedicated `covenant_reaudit` target/selector, wired by audit worker; do not run unrelated upstream tests. |
| Release | Missing exe, bad digest, certificate/provenance mismatch, untrusted or wrong-commit attestation, absent real-sidecar evidence all prevent promotion/adoption; manual artifact path cannot publish. | Dedicated release helper test target finalized by release worker; no live publication in local tests. |

Prefer existing integration fixture helpers and dedicated sibling `_tests.rs`
modules; no process-global environment mutation in unit tests. New agent tests
belong in dedicated `core/tests/suite/covenant_*.rs` modules with only minimal
registration in `suite/mod.rs`, or an explicitly registered focused integration
binary when that avoids compiling the aggregate upstream suite. Test worker must
choose after inspecting Cargo target layout. Use actual executable observations
and real process/file effects; object equality over field-by-field assertions.

Run targeted checks only once appropriate evidence passes, then scoped `just
fix -p <changed-project>` for substantial Rust changes and format only fork-owned
or intentionally changed files per A09. For the isolated runtime use `cargo fmt
--manifest-path ../covenant/runtime/Cargo.toml`; do not run the whole-tree
formatter into unrelated upstream files. Do not
rerun tests after fix/fmt absent a new substantive change or failure requiring
resolution. Inspect formatting/lint diff and preserve unrelated user/upstream
files. Dependency changes require `just bazel-lock-update`; compile-time data
requires corresponding Bazel data declarations. Record a tool/build blocker
without repairing unrelated upstream code or claiming tests passed.

## Sequential checkpoints and parallel ownership

1. Planning/scout review: root accepts A01-A06, exact F00 map, authority scope,
   external constraints, and next work package. Current ledger is planning only.
2. Bootstrap: F01 first; F00, F02, F10, and F40 can use disjoint file owners.
   F03 follows F02. Static artifacts are reviewable before any behavioral code.
3. Behavioral wave: after F00/F02, F11, F12, and F14 can run independently;
   F21 may proceed on its separate config/CLI/spec files. F13 waits for F12's
   shared client contract. Do not parallel-write shared gate or module files.
4. Constrained executable checkpoint: integrate F11/F12/F13/F14/F21; run only
   new fork behavior selectors and actual exe probes; require all local safety
   invariants. F22 then owns CLI main exclusively after F21.
5. Audit checkpoint: F31 consumes the completed patch/context/index/inventory
   and executes its positive/negative tests. Independent review checks security
   invariants, scope, missing tests and false-green risks; fix findings locally.
6. Promotion preparation: F33 owns release workflow after F03 and patch index
   after F40; prepare real release job and validate refusal paths. E01 blocks
   certification even if mock tests pass. F32 remains external-pending.
7. Final remote checkpoint: reconcile user's no-intermediate-push rule with
   live-CI/release requirements using explicit user direction if still needed.
   A local implementation is not an F03 run, F31 hosted green result, F33
   Release, or F32 harness adoption. Do not upload commits/tags to simulate
   completion. After authorized final push, only actual remotely verified
   results may close their corresponding phase acceptance.

The root-assigned integration worker exclusively owns shared `lib.rs`/`mod.rs`, Cargo workspace manifests,
lockfiles and `BUILD.bazel` registration edits. The F12 owner controls shared
gate code through F13 integration. F21 -> F22 serialize `cli/src/main.rs`.
F03 -> F33 serialize release YAML; F40 -> F33 serialize patch index. Tests and
implementation workers must not write the same module simultaneously.

## Concrete external blockers

Read-only inspection found sibling checkout `../Covenant-Harness` at
`b6e933a4590a2ef848755c4a593a7e9e8f2072d4`, remote
`ChristNata/Covenant-Harness`. `../Covenant-Harness-Test` is a separate
documentation/test-project tree and was not used as the runtime implementation.
No credential stores were inspected, and neither checkout was modified.

| ID | Observed gap | Required input/evidence before acceptance |
| --- | --- | --- |
| E01 | Harness `backend/covenant-cli/src/cli.rs` BashGuardClient has only ClaudeCode/Opencode/Cursor. `hooks/decision.rs` has no decide_v1/full-env/Windows identity fields. No sidecar-fixture pin exists. Paginated GitHub release inspection returned 15 draft installer-only releases, with no standalone G4 sidecar asset. | A real Windows `covenant-cli` Release asset supporting `hook decide --client codex`, URL + SHA-256 + attested F10 schema/hash and complete-env/volume/file/digest/auth-Read semantics id; policy fixtures producing all discriminating decision pairs. F12/F13 real integration and F33 promotion stay pending. A local stub is never a substitute. |
| E02 | No `dispatch_run/codex_adapter.rs` in inspected harness checkout; source search found no CODEX_AUTH_HOME/COVENANT_DECIDER_PATH contract. | Landed CRN C3+C6 adapter/home-probe implementation, stable auth root versus selected mutable-home routing, immutable launch env, and real-adapter separation test evidence; aligned C2 auth deny and P6 canary. F14 local behavior can be tested independently; F32 adoption cannot. |
| E03 | `backend/covenant-utils/src/pinned_versions.rs` has no CODEX_PIN or official/fork discriminant. | Landed S2 four-field Codex pin schema plus C7 runtime inventory consumer. Actual harness pin write remains outside this fork-only session until explicitly authorized, after real F33 attestation verification. |
| E04 | Actual fork has no GitHub Releases. Planned `covenant-capital/codex` did not resolve through gh; actual public fork is ChristNata/covenant-codex. | Real audited immutable fork commit, Windows build, exact five Release assets and GitHub build-provenance attestation. Never invent a digest, URL, run id, tag or provenance claim. |
| E05 | User forbids intermediate remote pushes; F03/F33/F32 live acceptance inherently follows remote availability. | User-directed final remote sequence once local work and prerequisite evidence are complete, or a clear pending external handoff. This ordering constraint cannot be solved by relabeling local checks as hosted/release evidence. |

Existing `.github/workflows/sync-stable.yml` has only manual and daily scheduled
triggers (12:09 Asia/Jakarta). A branch push does not directly trigger it. The
existing job can independently force-update pristine main, create an upgrade PR,
and post an `@codex` comment using its configured token; this implementation
does not invoke or modify that automation. Upstream `rust-release.yml` triggers
on `rust-v*.*.*` tags; avoid using that namespace for Covenant promotion.
Upstream full-CI branch pushes match `**full-ci**`, not `covenant-ver`.

## Evidence updates

- Initial planning: repository/provenance/context/external metadata inspected;
  this ledger is the only file written by the planning worker. No source code,
  dependency installation, build, test, commit, push, workflow run, release, or
  harness pin change was performed by that worker.
- Implementation owners append phase-specific red/green/review command evidence
  and actual file ownership through the root-assigned integration worker as work
  proceeds; the root adjudicates and coordinates.

### F01 test-author checkpoint

The test-author owns only `covenant/tests/test_bootstrap.py` and these ledger
updates. Tests use Python stdlib unittest and disposable local Git repositories.
They do not change real refs, run upstream tests, invoke network operations, or
implement production validation.

The implementation contract is:

```text
python covenant/scripts/validate_bootstrap.py --repo ROOT --manifest PATH
```

The flat TOML manifest requires string fields `upstream_url`, `tag`, `commit`,
`license_spdx`, `rust_toolchain_path`, `rust_channel`, `fork_url`, and
`fork_branch`. The upstream URL identifies `https://github.com/openai/codex`;
the actual origin and current branch match the recorded fork topology. Only a
harmless trailing `.git` URL normalization is permitted. Tags must be valid
stable `rust-v*` tag names, resolve locally (annotated or lightweight) to the
exact 40-hex commit, and that commit must be an ancestor of HEAD. Reject
revision expressions/options in the tag/commit input before Git resolution.

LICENSE and NOTICE must match existence and exact bytes in the pinned tree,
including the valid case where upstream has no NOTICE. The toolchain path must
be repository-relative, remain inside the repository, exist in the pinned tree,
and its current content and recorded channel must agree with that pinned file.
Success exits 0 and identifies the verified tag and commit on stdout. Refusal
exits 1 with the failing field/artifact and an actionable `remediation:` line on
stderr, with no Python traceback for malformed user input.

`python covenant/tests/test_bootstrap.py` exercises production by default;
`--validator PATH` explicitly selects a temporary baseline adapter for test
assessment. Thirteen test methods cover allowed ancestry/tag forms, attribution
existence/bytes, malformed or missing metadata, invalid/unresolvable/nonancestor
pins, absent/mismatched tags, substituted repository/topology, and toolchain
drift/path escapes. They assert runtime outcomes against synthetic inputs, not
the fixed values of the real manifest.

Discovery command: `python covenant/tests/test_bootstrap.py
BootstrapTests.test_accepts_annotated_upstream_tag_with_later_fork_commit`
ran one test and failed because the production script does not yet exist
(Python exit 2). This records a missing entrypoint only; it is not behavioral
red evidence for any validation rule.

Behavioral baseline assessment: a temporary, explicitly labeled permissive
adapter parsed the same CLI/manifest and reported success for every request.
`python covenant/tests/test_bootstrap.py --validator <temporary adapter>` ran
all 13 tests in 14.087 seconds: two acceptance-only tests passed and the suite
reported 20 failing assertions/subtests across adversarial cases, exiting 1.
For example, an absent local tag, a tagged nonancestor commit, substituted
origin, missing LICENSE/NOTICE, and an outside-repository toolchain path were
incorrectly accepted by that adapter and rejected by the tests. This proves
test discrimination against a permissive baseline, not production security.
The adapter lived only in a TemporaryDirectory and was removed automatically.
Production implementation remains absent and no green production result is
claimed. The durable evidence summary is this checkpoint; detailed process
output was returned to the root by the test-author tool run.

### F01 implementation checkpoint (complete locally; checkpoint committed)

The separate implementation worker's full evidence is
`.git/covenant-session/f01-evidence.md`. It added the eight-field manifest and
offline validator, then ran `python -m unittest discover -s covenant/tests -p
test_bootstrap.py`: 13 tests passed in 26.714 seconds. An actual checkout
validation passed after the A07 tag acquisition and A09 exact-byte checkout
restoration. The validator itself never fetches and refuses malformed metadata,
lineage/topology mismatch, attribution drift, and pinned toolchain disagreement
with bounded actionable errors.

Only the already-verified `rust-v0.153.4` tag was fetched without force or branch
movement; it peeled to `3d2ee51ca2d5db578f328aa75e20aa22c0197c9a`. LICENSE, NOTICE,
and the toolchain had CRLF checkout bytes despite LF upstream blobs. The worker
proved CRLF-only equality and no existing content diff before restoring exact
blobs. Git content and cached diffs remained empty; root-approved stat refresh
cleared those three paths from status without staging content. Scoped Ruff
checks passed and formatting touched only the new validator. Tests were not
repeated after formatting. F01 remains pending independent review/root
adjudication; F00 authority completeness and build/release readiness are not
claimed.

Independent review subsequently identified false-green cases involving grafted
ancestry, effective-origin configuration (multiple URLs, rewrites, push URL),
HEAD/index versus working-tree attribution/toolchain state, and a branch/tag name
collision. The initial 13-test green result is superseded as an acceptance
checkpoint. A separate worker is adding the regression cases; the F01
implementation worker will correct them before independent re-review. F01 is
not complete. Detailed probe artifacts are under
`.git/covenant-session/f01-independent-probes.*`.

Correction/re-review checkpoint: the separate regression author established 17
tests with 14 behavioral failures against the old validator. The implementation
worker corrected only production code; all 17 tests then passed in 49.144
seconds, along with actual-checkout exact-pin validation and scoped Ruff.
Independent re-review by fork_master found all four prior findings resolved and
no new blocking logic/scope issue. Report:
`.git/covenant-session/f01-rereview.md` (PASS, with source/test line references).
Two narrow additional probes verified rejection of nonregular index mode and a
single unmerged index entry; the suite was not broadly repeated. Tracked working
and cached upstream diffs remain empty. F01 state is `reviewed` and
`local-verified`, awaiting root adjudication before completion; no commit or
push has been made. F00 stays structural `tests-red`, and F10 remains pending
real validation implementation rather than accepting its permissive scaffold.

Root subsequently accepted the F01 independent PASS and authorized two local
checkpoint commits. The delegated integration worker inspected an empty index,
staged explicit paths only, checked staged names and `git diff --cached --check`
before each commit, and created:

| Local checkpoint | Commit | Exact committed scope |
| --- | --- | --- |
| Planning ledger | `11c42edb4d431030f2746709c98e8a212d327f11` | `covenant/IMPLEMENTATION.md` only; message `docs(covenant): record implementation phases and scope`. |
| F01 bootstrap | `3219b595a84a8dc7a996f4ec1ef7c6c3aefe6df7` | `covenant/UPSTREAM.toml`, `covenant/scripts/validate_bootstrap.py`, and `covenant/tests/test_bootstrap.py` only; message `feat(covenant): validate the pinned upstream bootstrap`. |

F01 status is now `complete` for the adapted local bootstrap scope. No test rerun
was needed after formatting/review, and no commit was pushed or remotely tagged.
The index was empty after both commits. F00 tests/validator work, runtime/schema
work, and the user's `covenant_docs/` were excluded and remained untracked.
This hash/status ledger update is intentionally left for the next checkpoint,
avoiding a self-referential commit chain. F00/F10 completion and downstream
build/gate/release/harness acceptance are not implied by these commits.

### F10 test-author checkpoint

Design source: `.git/covenant-session/f10-plan.md`, with root-approved A08/A09
and the following numeric fidelity correction. Canonical Draft 2020-12 integer
fields allow integral values such as `1.0` and have no u64 maximum. The test
corpus includes `18446744073709551616`; the eventual wrapper must preserve
schema acceptance with lossless number handling, rather than silently tightening
the shared schema. Numeric representation need not be lexically identical on
serialization. OS range/identity/path/environment semantics belong to F12/F13.

The test-author copied the schema bytes to `covenant/schema/decide-v1.json`.
Both copies have SHA-256
`d06c6927784899cea94940854e6ab96f4ca622e49f7648e1fb3584ac1272bb48`.
No schema field, ID, restriction, or formatting changed. The test's compile-time
include reads that exact fork artifact; future Bazel registration must list it
as compile data before claiming Bazel integration works.

Owned scaffolding: standalone manifest/lock, local target-directory ignore, local nextest profile,
`src/lib.rs`, `tests/decide_v1.rs`, and sibling `tests/decide_v1_tests.rs`.
Production dependencies are pinned serde/serde_json only. Dev dependencies are
pretty_assertions and jsonschema 0.29.0, with default HTTP/file resolvers disabled.
The validator explicitly uses Draft 2020-12 and only internal schema references.
serde_json arbitrary_precision preserves schema-permitted integer ranges.

The root explicitly approved a temporary, prominently labeled permissive
`DecideV1::decode` + Serialize wrapper around serde_json::Value solely to obtain
observable behavioral red evidence. It does not validate requests and MUST be
replaced by the implementation worker before any F10 completion/checkpoint.
No typed DTO/build/accessor API, process gate, auth behavior, source integration,
or baseline Cargo/Bazel/lockfile change was implemented by this test stage.

Focused run from `codex-rs`, after sourcing the session environment:

```text
just test --manifest-path ../covenant/runtime/Cargo.toml -p codex-covenant --locked
```

The standalone crate compiled successfully (44.78 seconds). Nextest run
`6b6f40a8-bd11-4d03-8363-f74ed0a2abbe` ran 11 tests in 0.717 seconds:
2 passed, 9 failed, 0 skipped. Complete exec/patch roundtrips and
schema-permitted empty/optional collections passed. Schema-invalid envelope,
unknown fields, env/argv types, min-length/min-items, conditional patch
identities/digests, nonintegral/negative counts, duplicate keys, and secret-bearing
invalid requests failed against the permissive parser as intended. Every shared
valid/invalid corpus expectation was first checked against the actual JSON
Schema validator, so the red result is a runtime enforcement failure, not an
invalid test fixture or missing entrypoint.

Duplicate-object-key cases are explicitly labeled additional transport policy:
JSON Schema receives decoded objects and cannot detect duplicate source keys.
The tests keep schema-permitted case-colliding/empty env keys, empty argv,
optional destination/digest/target fields, and empty hunk/root/ancestor arrays
accepted at this wire layer. Complete descendant env, credential scrub, absolute
path resolution and real Windows file identities remain outside F10 evidence.

After this red run, only literal-argument comments and standalone `cargo fmt`
formatting changed the Rust files. Tests were not repeated after formatting.
`git diff --name-only` remained empty for tracked upstream files, and status
showed only the existing untracked `covenant_docs/` plus new `covenant/`.
The standalone build target directory is ignored; its own Cargo.lock is retained.
F10 status is `tests-red`, pending real validation implementation and independent
review, with no source integration or release readiness claimed.

### F00 structural test-author contract

Root approved `.git/covenant-session/f00-plan.md` for the next test-first stage.
Its scope is strictly offline structural validation: exact locally available
commit binding across three inputs, normalized repository-contained source paths
and literal anchors in both pinned and working views, closed classifications,
foreign-key consistency, and declared family/secret coverage. Passing this
validator cannot prove complete discovery or semantic gate domination; those
remain mandatory human F00 review and F31 behavioral audit obligations.

```text
python covenant/scripts/validate_integration.py --repo ROOT \
  --upstream UPSTREAM.toml --integration INTEGRATION-FILES.toml \
  --inventory AUTHORITY-INVENTORY.toml
```

Success exits 0 and reports the pinned commit plus checked row counts. Invalid
input exits 1 with a bounded field/row category and `remediation:` instruction;
no traceback, source-content echo, network access, or execution of manifest
commands. The upstream file requires its commit here; F01 separately owns
tag/origin/license/toolchain verification and is not duplicated by F00.

The test-author owns only `covenant/tests/test_integration.py` and this ledger.
Fixtures are disposable Git repositories with generated provider-key/tool IDs,
declared synthetic source symbols, full required seam/family rows, and explicit
optional-scout absences. The tests do not populate real inventory artifacts,
implement the structural validator, modify real refs, or run upstream tests.

F00 test-author evidence: the default production entrypoint discovery ran one
valid-fixture test and failed because `validate_integration.py` is absent
(Python exit 2); this is not behavioral proof. An explicitly selected temporary
permissive adapter then ran all 14 tests in 15.913 seconds. Two valid-fixture
tests passed and 53 adversarial assertions/subtests failed, with exit 1, covering
commit mismatches/object types, missing and unsafe source paths/anchors, seam
accounting, duplicate/unknown classifications, foreign keys, declared family
coverage, secret-key coverage/collisions, forbidden manifest commands, malformed
inputs and wrong types. The adapter was removed with its TemporaryDirectory;
no production scaffold or real inventory files were written.

Run command for the implementation worker is `python -m unittest discover -s
covenant/tests -p test_integration.py`. The test file also accepts an explicit
`--validator PATH` only for labeled baseline assessment. Scoped Ruff check
passed, then Ruff formatted only this new test file; tests were not repeated
after formatting. F00 status is `tests-red`, not source-completeness verified.

### Pending F12/F13 design amendments

The deeper source scout found that the process-manager request environment is
not final across every backend: snapshot/remote and Windows restricted/elevated
paths can overlay it later. Tests or implementation for plain local exec cannot
be treated as sandbox/remote coverage, and the original three-file F12 list does
not establish sufficient source authority for every backend. Root will decide
the exact supported backend scope after implementation prerequisites.

Zero-descendant settlement requires deliberate fork-specific Windows Job
semantics: current upstream supervision assigns jobs after start and preserves
descendants on normal exit. The patch engine reparses raw patch text and writes
sequentially; F13 race/identity invariants need guarded mutation, not only a
preflight stat/hash and policy call. These are pending design amendments, not
completed engineering decisions or authorization to expand source edits now.
Detailed evidence is in `.git/covenant-session/scout.md` and
`.git/covenant-session/f12-design-review.md`.

### F10 implementation and independent regression stage

The implementation worker replaced the permissive scaffold with a private typed
decoder, immutable validated wrapper, content-free error, and exact decimal
integer validation. Its original 11 tests passed in Nextest run
`51f0d4bd-24e6-4572-a44c-5cfc3eaaff54` (0.752 seconds, 0 skipped), but the
worker identified two untested serde shape gaps before completion and held
production corrections for independently authored red tests. This initial
green run is not F10 acceptance.

The test author added five regression methods: externally tagged object
substitutions for all five string-enum positions, positional array
substitutions for all ten schema-object positions, raw serde private-number
object spoofing, ordinary decimal/exponent version semantics, and exact
mathematical numeric boundaries. The schema oracle establishes rejection of
the shape cases before the production assertions. Raw spoof objects are
serialized directly to original request bytes without a Value reparse that
could erase their hostile shape.

The pinned dev oracle uses floating-point numeric const comparison; it can
round near-one decimals or fail on huge exponents. Its numeric limitations do
not amend the canonical schema. The ordinary-number corpus retains the real
Draft 2020-12 oracle; separately labeled exact mathematical cases establish
near-one rejection, exponent equality, very large integer acceptance, negative
zero, and nonintegral rejection without that float oracle.

Focused `just test --manifest-path ../covenant/runtime/Cargo.toml -p
codex-covenant --locked` compiled in 3.38 seconds and produced Nextest run
`55872857-fe4b-40c3-a8f5-e17bbd6692a1`: 16 tests, 14 passed, 2 failed,
0 skipped in 1.204 seconds, exit 1. All five enum-object cases and nine derived
struct-array cases were incorrectly accepted; the environment-array case
correctly rejected. The original 11 tests and three new numeric/spoof methods
passed. These are real production behavioral failures, not scaffold or missing
entrypoint evidence. Root accepted this red stage; F10 remains `tests-red`
pending corrections and independent review. Only the test file and this ledger
were edited by the regression author; scoped test formatting follows this run.
Evidence is recorded in `.git/covenant-session/f10-regression-red.md`.

### F10 corrected implementation and independent review

Production corrections passed all 16 focused tests, 0 skipped, in Nextest run
`99bce712-7697-450d-b467-b871cb6e3a47` (1.107 seconds; compile 4.65 seconds).
Scoped `just fix` passed without warnings or edits; the implementer then
formatted only five production modules. The independent test-file hashes and
root Cargo/Bazel hashes remained unchanged. No test suite was repeated solely
after formatting.

Independent review read all 491 production lines and the schema/test evidence
and found no blocking defect. Every closed object passes through map-only
decoding, each enum requires a string, optional fields distinguish omission
from null, and the public wrapper cannot be built unchecked or mutated. Exact
numeric-token and decimal-scale checks preserve schema integer semantics
without float or u64 narrowing. Complete payload serialization, duplicate-key
refusal, and bounded content-free errors remain intact. Root accepted the
independent PASS and marked the F10 standalone local artifact stage `complete`.
This does not complete sidecar, native path/env, process, patch, source/Bazel
integration, or external harness acceptance.

The documented just invocation uses the existing `codex-rs` Rust 1.95.0 pin;
independent offline locked Cargo metadata confirms only the standalone package
and its separate target/lock. The crate has no current Bazel target. Its test
`include_str!` of the canonical schema must receive explicit Bazel compile_data
when integration adds BUILD/dependency wiring; that pending integration is not
claimed green. Schema copies still hash to
`d06c6927784899cea94940854e6ab96f4ca622e49f7648e1fb3584ac1272bb48`.
Full review is `.git/covenant-session/f10-review.md`.

The delegated integration worker created local checkpoint
`a3231e1bd82ffd9df63c54d50e6dbab9b228b0a0` with message
`feat(covenant): validate the decide v1 wire contract`. Its 13 explicit paths
contain only this ledger, the canonical schema copy, standalone manifest/lock,
local nextest profile and target-only ignore, five production modules, and two
test files. Staged whitespace/names checks passed; the index was empty after
commit and runtime target files remain ignored. F00/F14 work and user documents
were excluded. No remote push occurred. This hash annotation stays unstaged for
the next ledger checkpoint, avoiding a self-referential commit chain.

### F00 implementation handoff awaiting independent source review

The implementation worker reports 15 focused tests green in 48.843 seconds,
including the A11 test that first failed against production for unknown form.
Scoped Ruff passed. After the A10 source-map additions, the real structural
validator passed again with 15 seams, 54 tools, and 132 authorities (54 tool,
33 constructor, 45 sink); inputs reference 107 source files, 21 key names, and
12 literal audit queries. Evidence is `.git/covenant-session/f00-evidence.md`.

F00 status is `implemented`, pending independent structural and human source
review. Dynamic provider/MCP secret-name closure remains unresolved for F12/F21;
finite declared-key coverage does not prove complete credential scrubbing.
Literal source validation does not prove complete authority discovery, callable
reachability, gate order, final environments, or semantic domination. No F00
completion or downstream source-edit authorization follows from this green run.

### F02 recipe and build-output plan

Root approved the direction of `.git/covenant-session/f02-plan.md`: a strict
`windows-repro.toml` plus a small fork-owned Python build/digest helper, with
tests authored before implementation. The current product invocation remains
the existing codex-cli/codex binary, pinned Rust 1.95.0, locked release build,
and x86_64-pc-windows-msvc target; no new frontend or copied exec loop is planned.
Measured rustc/cargo commit identities and the existing static-CRT/stack/release
configuration are recorded in the plan. Reproducibility is digest-recorded.

Tests will cover source-bound input/path validation, literal argv, exact tool
versions, stale/missing/wrong artifacts, failed builds, byte-exact SHA-256 and
the boundary between an unattested build-output receipt and promotion evidence.
The helper must preserve stale/existing files and cannot fall back to an
unlocked build or unrelated upstream repair. F02 status remains `planned`;
no actual constrained codex.exe has been produced or accepted.

Root's auth isolation evidence provides a possible later necessary integration
route: retain every external package/version/source while correcting stale
local path-package lock metadata and pruning unreachable records in an owned
workspace. This is not a general dependency upgrade and does not yet establish
product binary selection or profile/config/build-resource equivalence. Any
F21-required root dependency/lock changes still need their own bounded reviewed
integration. F02 planning makes no such changes. Actual constrained-product,
live F03, real G4, F31 and F33 acceptance remain separate pending gates.

F02 test-author evidence: `covenant/tests/test_windows_build.py` defines 10
focused methods against the plan's main/command-runner contract. Production
entrypoint discovery failed before running any tests because build_windows.py
is absent; this is not behavioral red evidence. A temporary, explicitly
permissive baseline then ran all 10 methods in 1.203 seconds: 2 methods passed,
8 methods failed with 35 failing assertions/subtests, 0 skipped. It deliberately
ignored recipe binding, version checks, build failures, Cargo artifact events,
output collisions and output bounds; the tests exposed those wrong behaviors.
Synthetic producer bytes are only helper-fixture evidence, never a real Rust
or constrained-product build. The temporary adapter was removed automatically.

Scoped Ruff check passed, then only the new test file was formatted; no tests
were repeated solely after formatting. F02 status is `tests-red`, pending
production implementation and independent review. Exact contract and evidence:
`.git/covenant-session/f02-plan.md`, `f02-tests-evidence.md`, and `f02-red.log`.
No build helper/recipe, root dependency edit, actual product build, commit, or
push was made by this authoring stage.

F00 source-review update: the correction candidate now reports 178 authorities,
45 literal queries and 118 source files, with the structural validator green.
Independent re-review in `.git/covenant-session/f00-rereview.md` still requires
three narrow metadata additions for startup git_head_sha process creation,
persist_agent_identity_record credential mutation, and auth/revoke network/error
handling. F00 remains review-pending; structural green does not close those
source findings or establish secret-name closure.

### F00 final independent acceptance

Root accepted F00 source inventory and structural verification after all three
final source findings were corrected and independently confirmed in
`.git/covenant-session/f00-rereview.md`. The unchanged 15-test suite remains
green; actual structural validation of the final inputs also passed. Current
totals are 15 seams, 54 tool identities, 181 authorities (54 tool, 46 constructor,
81 sink), 50 literal queries and 119 existing source files.

Reviewed final SHA-256 values were independently matched before staging:
INTEGRATION-FILES `93ca5f3c7df9dc5036582c2fb777d5838f1dc409dfb262261c543a60332a06a2`,
AUTHORITY-INVENTORY `e72531ff53a9f0798c6e7ee6c703853d1d6aa020085a197d0f40694d59630b1d`,
validator `c150893643d258c181cbbb41fc6c80836da753dcc3db8620276cd3047ede91de`,
test `daad029ba3067cebb2ca5aac4e63308ffb4a775c8240bc1b84dd38e15b5bb722`.
No unchanged tests were repeated for the metadata corrections or checkpoint.

F00 is `complete` for the reviewed source inventory/structural-verifier stage.
This accepts source anchors and proposed downstream ownership, not implemented
admission/exclusions, complete credential scrubbing, native authority protection,
or F31 semantic domination. Dynamic provider/MCP names and other recorded policy
decisions remain explicit F12/F21 design prerequisites. Release and external
acceptance remain pending.

F00 local checkpoint: `820438291b79b6d5dcf5481a4c8e4940b0ebfd91`,
`feat(covenant): record and validate upstream authority inventory`. Exactly
the two manifests, structural validator, focused test file and this ledger were
staged. Reviewed file hashes and staged whitespace/names checks passed; index
was empty after commit. Concurrent F02/auth/runtime/user paths were excluded.
No unchanged tests were repeated and no push occurred. This hash annotation
stays for the next ledger checkpoint.

### F03 planning and approved F12 design boundary

F03 planning is recorded in `.git/covenant-session/f03-plan.md`, with exact
Actions commits verified against their authoritative Git refs and action
metadata. It proposes a manual-only windows-2022 build/upload job using F02,
read-only repository permission, explicit files and a checked exe/digest pair.
It contains no F03 tests or implementation. Native setup/linker observations
and any newly needed helper behavior require bounded follow-up review. Live
dispatch remains pending until the workflow is on covenant-ver and the user's
final-push condition permits publication; no workflow was invoked.

Root read/adjudicated the F12 source findings summarized by A12-A16; this ledger
owner read `.git/covenant-session/f12-design-review.md` in full before recording
them. Necessary fork-only final-backend integration is approved, with local-only
Windows support, explicit unsupported-route denial, separately proven sandbox
branches, native Job settlement, guarded decider identity and honest deadline/
preparation boundaries. F12 remains `planned`, not implemented or tested.
Concrete size limits, typed API design and stage-specific native tests remain
the next F12 prerequisites; real G4 acceptance remains external-pending.

### F14 first Windows auth-home routing checkpoint

Root accepted only the first routing tranche after
`.git/covenant-session/f14-routing-review.md` reported independent PASS. The
82-line production change registers a Windows-only private module and chooses
CODEX_AUTH_HOME at the common storage factory. It freezes the override result
on first use, creates/canonicalizes a present absolute root, and preserves each
caller's supplied home when the override is absent. Failed initialization
returns an error backend rather than falling back. Backend selection itself
remains unchanged. This is pathname routing, not held filesystem identity or
refresh-transaction safety.

The six frozen tests exercise real public codex-login APIs in isolated child
processes with temporary File-mode credentials. Original-source Nextest run
`e505d47f-26dd-4195-9ae0-414fe50fa3f7` produced one pass, five behavioral
failures, zero skipped; two failures occurred before later assertions and were
labeled accurately. The corrected source then passed all six, zero skipped,
in run `96225459-a678-459e-a215-951e56422e30` (0.159 seconds; compile 16.96
seconds). All six reached final assertions in the green run. Scoped production
formatting followed; no unchanged tests were repeated for review/checkpoint.

The owned `covenant/login-tests` workspace directly compiles the actual login/
protocol path crates and unchanged integration test. Independent comparison
found zero new/changed external name/version/source/checksum identities;
423 unreachable external packages were pruned and 32 stale local path-package
versions corrected to their existing source manifests. The original root lock
remains unchanged at SHA-256
`cc69db68df16e6d243d5151ff85d5c4d34ca562f942b8be9016c00d038b512d5`.
The test hash remains
`d0a7e522cc18ec83df5dbeac3074a9ee7288a19a3de9b0ddbf9583b152f4a954`
and owned lock hash
`b0128b029ce9cb0967050c3bc271b9565cd007d32e664f3aefb18cf59e8074a0`.

F14 is partial: valid File-mode routing is locally accepted. Invalid-root,
new-root/alias, mocked keyring/Auto/Ephemeral, browser/device flow, refresh and
competing-writer ownership, atomic replacement, cancellation, secret-canary,
encrypted-store decisions and external harness separation remain later stages.
Fixture redaction needs strengthening before secret-canary acceptance. No
root-workspace build health, Bazel test wiring or real harness acceptance is
inferred from this focused run. The new module uses normal source-module
resolution and introduces no compile-time data include.

The test workspace's local `/target/` ignore excludes default generated output;
the exercised codex-rs/target-login-isolation cache is already excluded by the
existing codex-rs target-* ignore. No generated cache or root dependency file
belongs to this checkpoint.

Routing-only local commit:
`87963dd5ece63ee92d9b7bbd1dbf948c6524a143`,
`feat(covenant): route Windows auth storage to a dedicated home`. Its nine
explicit paths contain the three routing production files, unchanged six-test
snapshot, isolated manifest/lock/nextest profile/target ignore, and ledger.
The staged test blob was hashed immediately before commit and matched the
reviewed test SHA above, keeping subsequent rotation registration outside this
checkpoint. Staged names/whitespace checks passed and the index was empty after
commit. No F02, rotation test, root lock, user document or generated cache was
included; no test repeat or push occurred. This hash annotation is reserved for
the next ledger checkpoint. F14 remains partial.

Root reports F02's frozen ten tests now green against its implementation; its
independent review is still pending. No F02 artifact/product or F03 live-CI
acceptance is inferred from that report. F03 plan handoff is complete and no
workflow/test implementation has begun in this planning stage.

### F02 independent review requires a correction

The helper implementation's frozen ten-test run was green (10/10, zero skipped,
latest 0.841s), with scoped Ruff checks and no root lock/source change. Full
independent source review found one previously uncovered input false green:
`build_windows.py:197-199` accepts workspace.members as a string, mixed array,
or table when Python membership finds "cli". Those shapes cannot establish the
declared Cargo workspace membership. Status is `review-failed/R1-pending`, not
complete. Detailed findings and reviewed boundaries are in
`.git/covenant-session/f02-review.md`.

The test author added one method while preserving all original assertions.
Real production red: one targeted test, three expected refusal assertions fail
with actual success JSON, zero skipped, 0.388s, exit 1. Evidence is
`.git/covenant-session/f02-review-red.log`. Scoped Ruff check passed and format
completed; there was no unchanged suite repeat. A different implementation
worker must correct collection/element validation before independent re-review.

The actual authorized `--check` against this checkout returned exit 0 with the
source-bound locked Rust 1.95.0 Windows codex-cli/codex invocation and null
artifact. This was configuration validation only: no product build, output
digest, F21 constraint, live CI, G4 or promotion claim. Root-approved observed
path containment remains trusted-workspace bookkeeping, not F12/F13 race
protection. No production file, root lock, Git index, commit or remote changed
during this review.

### F11 approved plan and test-stage direction

Root approved a default-off core `covenant` compiled feature forwarded through
the existing CLI; F21 must make the published/default fork build constrained
without a runtime disable. Typed CovenantDenied maps to the existing
nonretryable Fatal and ends the turn. The same policy must cover the raw router,
pre-readiness, streaming and final registry, preserving raw namespaces only in
constrained mode. SC8 evidence must inspect the actual `item.completed` content
and `turn.completed` usage events. The test author is exploring an actual-source
core test workspace with minimal feature/test registration, preserving the root
lock and external pins. Pure policy placement remains under review against the
rule to resist adding core code, with existing codex-tools a candidate. These
are approved design/test-stage directions, not implementation or green results.

F02 R1 correction independently re-reviewed: **PASS for the local helper
tranche, pending root acceptance**. The sole production delta is +9/-1 validating
the workspace table, members list and every string element before membership.
An in-memory old-block reconstruction matched the previously reviewed helper
hash exactly; the frozen eleven-test hash and recipe/root lock are unchanged.
The independent implementer's focused run passed 11/11, zero skipped, 2.191s;
actual non-building checkout check again returned recipe-validated/artifact=null;
scoped Ruff passed and formatting changed nothing. Reviewer repeated no tests
or check, made no source/test edits and created no checkpoint. Evidence:
`.git/covenant-session/f02-correction-evidence.md` and updated `f02-review.md`.
Full constrained artifact, native equivalence, live CI and external acceptance
remain pending; this verdict does not supersede their prerequisites.

### F02 accepted local helper checkpoint

Root accepted the F02 recipe/build-output helper tranche after the frozen
eleven-test run (11 passed, zero skipped), actual non-building checkout check
and independent R1 re-review PASS. This local stage is complete. Actual
constrained product build acceptance remains pending F21/product-graph readiness;
no synthetic output is treated as that artifact. The reviewed explicit files are
`covenant/scripts/build_windows.py`, `covenant/windows-repro.toml`,
`covenant/windows-build/.gitignore`, `covenant/tests/test_windows_build.py` and
this ledger. Current reviewed hashes match the correction evidence. All
concurrent core/auth/user files remain outside the checkpoint; no tests repeat
or push is authorized by this checkpoint.

Root approved F03's next test-author contract in
`.git/covenant-session/f03-verifier-plan.md`: a read-only pair verifier and exact
two-file upload. The F02 receipt remains development evidence on disk/log;
native preparation awaits the final product graph. Only behavioral tests may
be authored now, followed by a separate implementation worker; no workflow,
native setup, actual product build or hosted acceptance is implied.

F11 verification workspace amendment: root approved a temporary full-source
verification worktree. Only that copy's local lock path metadata may be
reconciled with identical external pins; the original root lock remains
unchanged. This is a verification-environment choice, not shippable source
repair or a general dependency upgrade. Feature/behavior acceptance still needs
the assigned test-first and independent review stages.

F02 local checkpoint completed as
`73ba1a6ac317bdb83b897e08f61ac48fc919ce11`,
`feat(covenant): record verified Windows build outputs`. It contains exactly
the four reviewed F02 files plus ledger. Staged names/whitespace checks passed
and the index was empty after commit. Concurrent core/auth/user changes were
excluded; no unchanged tests were repeated and no push occurred. This hash
annotation remains for the next ledger checkpoint. Product acceptance is still
pending, as recorded above.

### F03 artifact-pair test stage: behavioral red

`covenant/tests/test_windows_artifact.py` now freezes seven behavior methods for
the approved read-only pair verifier. Cases cover byte-exact success, each
missing member, digest mismatch/malformed/oversized data, empty/nonregular
members, complete pairs outside the allowed tree or at its namespace root,
traversal, and an actual local Windows junction. Input/sentinel bytes remain
unchanged across validation. No static workflow-value tests were introduced.

Entrypoint discovery ran zero tests and found the absent verifier; this is not
behavioral red. A clearly labeled temporary permissive verifier then produced
one passing method and six failing methods through 21 intended refusal
assertions/subtests, seven total methods, zero skipped, 4.222s, exit 1. The
junction fixture actually ran. The temporary module was removed; only its
ignored evidence driver/log remains. Scoped Ruff check passed and formatting
completed, with no test repeat solely after formatting. Frozen test SHA-256:
`6d2d4f570c26171d5bd44c3e7729e378fcbd7573e29ada3782a85f77cada55ac`.

Evidence/contract: `.git/covenant-session/f03-tests-evidence.md`,
`f03-red.log`, and `f03-verifier-plan.md`. Production belongs to the next
separate worker; no verifier/workflow/native setup or product build has been
implemented by this stage. Exact two-file upload is approved; receipt remains
development evidence on disk/log. Local helper/workflow and live acceptance
remain distinct, and no hosted run, G4, release or push is claimed.

F03 pair-verifier implementation independently reviewed: **PASS for the local
read-only helper, pending root checkpoint acceptance**. The separate worker's
frozen suite passed 7/7, zero skipped, 5.028s, including the actual Windows
junction. Full source review confirms the exact pair/digest, bounded reads,
contained ordinary paths, no mutation and content-free errors. The seven-test
file, reused F02 helper and F02 tests retain their frozen hashes. Scoped Ruff
passed with no formatting changes; reviewer ran no unchanged tests or builds.
Evidence: `.git/covenant-session/f03-verifier-implementation-evidence.md` and
`f03-verifier-review.md`. No workflow/native setup/hosted acceptance is implied.

F12 first-foundation proposal is now recorded in
`.git/covenant-session/f12-foundation-plan.md`. It proposes separate test-first
launch-control, Windows environment, and bounded envelope/reply stages inside
codex-covenant, with real Windows ordinal comparison and explicit Unicode,
pseudo-drive-variable and resource policies. The default OpenAI header source
names and arbitrary configured provider keys still require root-approved F21
closure/policy amendments. Its new target dependency and limits are proposals,
not approved implementation or tests. No source/manifest/lock/build changed
during planning; native identity/Job/backends/elevated/G4 remain separate gates.

Root accepted the F03 verifier helper tranche after its frozen seven tests and
independent PASS. Local checkpoint scope is exactly
`covenant/scripts/verify_windows_artifact.py`,
`covenant/tests/test_windows_artifact.py` and this ledger. The helper stage is
complete locally; workflow/native preparation/live F03 acceptance remain
pending. No unchanged test repeat, unrelated staging or push belongs here.

Root approved the F12 foundation caps, drive pseudo-variable preservation,
actual Windows ordinal comparison with fail-closed errors, and immutable paired
JSON/native environment block as proposed. Root also approved adding
OPENAI_ORGANIZATION and OPENAI_PROJECT to the reviewed F00/F12 scrub policy,
with concrete source-query evidence. F21 must reject effective credential or
header environment names outside the reviewed compiled set before construction
at every override/reload; preserve built-in defaults via those added names.
Arbitrary auth.command/MCP/plugin authorities remain excluded. This is a
closure prerequisite, not evidence that F21 enforcement already exists.

Root approved a later isolated Windows-only windows-sys=0.61.2 dependency with
Win32_Globalization for 12b, preserving all other standalone pins. Dependency
edits wait for that stage. Current 12a authorization is tests first plus a
clearly labeled permissive launch-contract scaffold and minimal registration;
no actual validation implementation, native process/hash/Job proof or spawning.

F03 helper local checkpoint completed:
`4a5a56b4202c0e706cd88ae5aacd56faa0afb3be`,
`feat(covenant): verify Windows artifact digest pairs`. Exactly verifier, frozen
seven-test file and ledger were staged; names/whitespace checks passed and the
index was empty after commit. No 12a or concurrent source/user changes entered
that checkpoint and no push occurred. Workflow/native/live F03 remain pending.

The approved F00/F12 header-name amendment is now applied to the inventory:
two added scrub keys, two anchored provider_login rows and two exact source
queries, preserving prior rows. Pinned/current model-provider-info source
anchors are lines 406/408; the actual env-header reader is lines 278-286. Actual
structural validator passed with 15 seams, 54 tools and 181 authorities. Narrow
source review/evidence is in `f12-scrub-amendment-evidence.md`. This does not
implement scrubbing or certify F21 credential-name closure; that remains a
mandatory runtime configuration restriction before construction and on reload.

### F12 12a launch-contract test stage: behavioral red

Eight Windows-only sibling tests now freeze named StartupControls, immutable
LaunchContract, borrowed path/expected-digest getters and a content-free unit
LaunchContractError. They cover ownership, exact digest decoding, missing or
malformed controls, native lexical paths, invalid Unicode/NUL, UTF-16 bounds
and bounded errors. A prominently permissive compileable scaffold accepts all
inputs and substitutes a zero digest solely to establish meaningful red; it is
not validation and must be replaced before any checkpoint.

Focused Nextest run `6d55a797-88cf-4398-9c14-5461c016a282`: compile 12.45s,
eight selected tests ran in 0.174s, one passed/seven failed. The sixteen F10
tests were deliberately filtered out; no selected foundation test was ignored.
Failures are incorrect Ok/digest behavior, not missing module or compile errors.
Only the three owned Rust files were formatted; no tests repeated afterward.
Full API/red/hash handoff is `f12a-tests-evidence.md`, with log `f12a-red.log`.

Manifest/runtime lock/F10 schema/root lock remain unchanged. There is no native
spawning, file hashing, identity/Job, complete environment, backend, G4 or actual
CLI-once-capture proof. 12b dependency edits wait for that stage. Root's future
12c obligation to define argv[0] and bind resolved program to exact authorized
argv is recorded in the foundation plan; it is not folded into 12a.

### F14 successful native rotation: accepted local tranche

Root accepted the separately implemented successful-rotation tranche after
`f14-rotation-review.md` returned independent PASS. The three frozen public
tests exercise actual codex-login in independent processes, redirected
temporary File stores and a synthetic one-use native refresh authority. The
baseline produced two requests/one reuse in each two-caller race and five
requests/four reuses in the mixed race. Focused implementation run
`d3df768f-db01-4e73-8982-0e1825530d1f` passed all three selected races in 0.713s;
the six routing tests were explicitly excluded there. Final full fork-auth run
`aea8fb97-678e-41a2-99ba-8cedcef389b6` passed nine tests, zero skipped, in 0.612s
after compilation. Every authority round reported one accepted request, zero
reuse and complete document/cache agreement; the mixed race completed a second
generation. This is the nine-test fork target, not an upstream suite.

The production tranche freezes the validated optional home for persistent
backend selection and adds a permanent sibling file lock with an owned RAII
guard. The existing configured File/Keyring/Auto choice remains; Ephemeral is
excluded. Both native refresh entries use the same guarded authoritative
load/account check/request/merge-save transaction. Same-account changed token
generations are adopted without another request. There is no missing-store
cached-document fallback in this refresh transaction. The per-manager
semaphore and non-opted-in control flow remain in place.

Full F14 remains partial: caller cancellation after authority consumption,
atomic replacement and concurrent-reader visibility, metadata mutation/cache
fallback, login/logout/revoke ordering, permanent-error recovery and backend/
identity failure cases require further test-first stages. No secret-sink,
browser/device, real harness, Bazel, constrained product or release acceptance
is inferred from successful File-store races.

The test-first local checkpoint is
`bcfdf23c5e2aa4dce8638421c54e8e9ecd6298e9`,
`test(covenant): reproduce concurrent native auth rotation`: exactly the frozen
four public-test paths, 746 added lines. Their ten combined test/production
hashes and both lock hashes matched the independently reviewed evidence before
staging. Source is frozen while the following separate production checkpoint
stages only its six auth paths plus this ledger (245 source change lines).
No tests were repeated after formatting, no original root lock changed and no
F11/F12/runtime/user file or remote action is included. Evidence is
`f14-rotation-tests-evidence.md`, `f14-rotation-implementation-evidence.md` and
`f14-rotation-review.md` under `.git/covenant-session/`.

Root separately accepted the two-header F00/F12 scrub amendment after
`f12-scrub-amendment-review.md` PASS. That inventory change remains outside
these auth commits and awaits a suitable F12 checkpoint. F21 configuration
closure is still a required implementation prerequisite.

F14 successful-rotation production checkpoint completed as
`315fb195c2120d6c3c90878fda117b2cea43ea83`,
`feat(covenant): serialize native auth token rotation`, following the frozen
test checkpoint above. Exactly six reviewed auth source paths plus ledger
were committed, with clean staged names/whitespace and an empty index afterward.
The shared auth fixtures were then released to the separate cancellation test
author. This hash annotation remains for the next ledger checkpoint; full F14
and remote publication remain pending.

### F12 12a lexical startup controls: accepted local tranche

Root accepted the replacement launch-contract implementation after independent
`f12a-review.md` PASS. Actual Nextest run
`6834d7e3-3d5f-4107-8732-de62b13789e1` passed all eight selected foundation tests
in 0.087s; sixteen F10 tests were deliberately excluded, with no selected skips.
The earlier permissive scaffold's one-pass/seven-fail behavioral red is retained
above. The final 113-line source validates owned controls, exact digest bytes,
lossless Unicode/NUL and UTF-16 limits, lexical drive-rooted path shape, and
opaque marker presence. It has no environment reread, filesystem/process access,
mutable getters or secret-bearing output. The permissive scaffold is removed.

Library-only Clippy passed without warnings and exact owned-source rustfmt/check
passed after green, with no unchanged test repeat. Source, frozen tests/lib,
manifest/lock/schema/root-lock hashes match `f12a-implementation-evidence.md`.
Checkpoint scope is those three runtime source/test/registration paths, the
separately accepted F00 two-header scrub/query amendment, and this ledger only.
No dependency/manifest/root-lock/schema change is part of 12a.

This accepts lexical startup controls, not actual CLI capture ordering, path
existence/local-volume/reparse safety, native executable hash/identity, process
or Job containment, environment completeness or real G4. F21 finite credential
name enforcement remains pending. The next 12b stage is independently authored
tests with a clearly temporary permissive scaffold, not environment validation
implementation. Its isolated windows-sys 0.61.2 Globalization dependency is
approved separately with all existing standalone pins preserved; root/Bazel
integration remains a later product-stage obligation.

F12a local checkpoint completed as
`76e21781a8bcaa9f3e8ef888a5be9b57325f31ca`,
`feat(covenant): freeze validated startup launch controls`: exactly the three
reviewed runtime paths, accepted inventory amendment and ledger, with clean
staged names/whitespace and an empty index. No manifests, locks, schema,
concurrent F11/auth files or user docs were included; no test repeat or push.
This annotation remains for the next ledger checkpoint.

### F11 first admission tranche: green and independently reviewed

The private codex-tools two-row identity policy and six approved production
seams now pass all eight frozen actual-registry tests. Nextest run
`237c8a3d-ab45-435c-bc08-db5c0c357378` compiled in 13m26s and ran in 1.217s:
eight passed, zero selected skips, 2,341 upstream tests deliberately excluded.
Exact-file formatting/check changed no bytes; all production overlays, frozen
tests/feature and original/temporary lock hashes matched. Independent
`f11-first-review.md` is PASS for this precise partial contract, accepted by
root. Scoped non-mutating library Clippy then completed exit 0 in 11m14s with
no warnings/errors or source changes, satisfying root's local checkpoint gate.
Evidence: `f11-first-implementation-evidence.md` and its recorded hash/log files.

F11 remains incomplete. The current constrained wire path is unusable until
the router companion preserves raw namespaces: upstream default normalization
would make the final gate reject even valid unqualified calls. Raw pre-parse
admission, parallel pre-readiness/metadata checks, post-hook/constructor clamps,
ordinary-mode controls, effective advertised specs and actual agent/E2E coverage
remain test-first follow-ons. This is identity admission, not F12/F13 native
effect authorization or a shippable constrained product. The superseded
covenant/core-tests experiment is excluded from checkpoints.

### F12 12b Windows environment test stage: frozen behavioral red

Root approved the narrow FrozenWindowsEnvironment raw-pair constructor and
immutable map/native-block getters, unit content-free error and real Windows
ordinal oracle. Eleven sibling tests now cover complete content/order,
validation/collision before scrub, 27-name/AWS policy canaries, pseudo drives,
lossless text and exact UTF-16/entry/aggregate bounds. Final substantive Nextest
run `e94ddf55-c2dc-44a3-98b7-d610e34cbdfe`: eleven selected tests, two pass/nine
intended behavior failures in 0.390s; 24 prior F10/12a tests deliberately
excluded. Scoped formatting/check followed, with no unchanged test repeat.
The permissive scaffold is explicitly nonshipping and has been handed to a
separate implementer; no environment validation or native process claim exists.

The isolated Windows-only windows-sys 0.61.2 Globalization addition retains all
101 existing external package records exactly. An offline resolver's five
unrelated JS/wasm downgrades were rejected/restored; subsequent locked Windows
metadata passed with only the approved new package/root edge. Original root
lock/schema remain untouched; no Bazel success is claimed. Full frozen hashes,
lock comparison and red details are in `f12b-tests-evidence.md`. F21 arbitrary
provider-name closure and actual final backend equality remain mandatory.

Root settled the future 12c argv binding: a named input will contain one
lossless absolute resolved application path plus an argument tail excluding
argv[0]. DecideV1 argv must be derived as that exact program spelling followed
by the tail. The native adapter derives argv[0] and lpApplicationName from the
same frozen program, with no independent identity override/command string.
Final audited Windows quoting length still needs validation. API/accessor
review and tests remain a separate stage; 12b is not expanded to implement it.

### F13 native qualification: approved bounded next step

Root approved native TxF qualification first, with mandatory unsupported-volume
or API refusal and no ordinary-writer fallback. An initial updates-only stage
cannot complete F13. Every settled denial/effective precommit race must retain
zero committed delta. Cancellation before commit admission rolls back; after
owner-admitted CommitTransaction, settle the actual result and never falsely
report denial or retry an unknown outcome. Blocked attacker attempts may retain
the original authorized identity. Transaction-private staging is distinct from
committed delta; ordinary temp artifacts/ACL changes are not permitted.

A structured prepared batch replacing constrained text reparsing remains a
necessary proposed seam, with no production edits before native proof/tests.
Caps, G4 absent-parent semantics and identity encoding remain pending. These
decisions from root's f13-design-plan adjudication are planning authorization,
not implementation or native transaction acceptance.

F11 first local checkpoint completed as
`f0c03c9bee7ac869a200e910699288f96f608f81`,
`feat(covenant): enforce closed tool admission at dispatch`. Exactly the six
reviewed production paths, frozen core feature/test file and ledger were staged;
names/whitespace checks passed and the index was empty. Source/feature/tests and
both lock hashes matched review; no auth/12b/superseded wrapper/user files were
included and no push occurred. Shared companion-test paths were then released
to their independent author. Full F11/product usability remains pending above.

### F14 caller cancellation: reviewed test checkpoint

Root accepted `f14-cancellation-review.md` PASS for caller-task cancellation
while the same Tokio runtime remains alive. The separately authored two-case
red run `d90b5b8a-c735-4cad-bd0d-1c741206943f` passed the cancelled waiter and
failed the consumed owner: one request consumed generation one, but no response
acknowledgement or complete persisted/fresh-probe generation followed. Both
cases confirmed task abort and continued runtime liveness before response release.

This test checkpoint contains exactly six frozen test paths and this ledger.
The two new files have 384 lines; shared changes only register/import modules
and extract the unchanged cleared-environment child command construction.
Test hashes match `f14-cancellation-tests-evidence.md`; all protected auth
production, manifest and lock inputs retain their reviewed hashes. The following
production checkpoint contains the separately reviewed ownership fix and its
actual focused/full green evidence. No permissive production scaffold, repeated
test, unrelated source, root-lock edit or remote action is included.

Acceptance is bounded to a live runtime and synthetic File-store/native HTTP
fixtures. Runtime/process death, atomic saves, competing metadata/login/logout
writers, permanent-failure cache recovery, redaction and full F14 remain pending.

F14 cancellation test checkpoint completed as
`cacc3e9bc46c414bd9160864be24f31f890bc121`,
`test(covenant): reproduce cancelled native auth refresh`: exactly six reviewed
test paths plus ledger, 496 changed lines. Staged names/blob content/whitespace
and the original root-lock hash passed; the index was empty afterward.

### F14 caller cancellation: accepted local ownership fix

Root accepted independent `f14-cancellation-review.md` PASS after reading the
source and evidence. Only the private refresh module changes: a caller still
waits for the lock and validates stored account/generation itself, then transfers
the guard, client and matching refresh token into one plain Tokio task owning
the complete HTTP response and authoritative merge/save. Caller cancellation
cannot drop that owned operation on a live runtime; cancelled waiters leave no
detached wait. Live callers retain existing result/cache reload behavior, with
a fixed content-free join failure. Backend/default-mode behavior is unchanged.

Focused run `e3826714-ed44-4e2f-a3ac-718d520ecac3` passed both frozen cases;
full fork-auth run `c2ff3abe-ed15-4f1b-b0cc-e9789a7cac2c` passed all eleven,
zero skipped. Both cancellation cases prove caller abort, continuing runtime,
complete persisted generation one and a fresh public probe, with one request,
zero reuse and one acknowledgement. Final scoped format/check passed; no tests
were repeated afterward. All protected hashes match the frozen handoff.

Evidence is `f14-cancellation-implementation-evidence.md`. Reviewed refresh SHA
is `8bebd1dd28b8f1cff0d525d18c5418e859ae53f80f23aba7b766125cfd20c3d9`.
This checkpoint contains that one source path plus ledger. Runtime shutdown,
process death, unseen responses, cancelled-manager cache repair, atomic writes,
competing writers, cached failures, redaction and full F14 remain pending.

F14 cancellation production checkpoint completed as
`7467030184b19c4b8c51bbd82df620ddfa23e4c0`,
`feat(covenant): preserve consumed refresh through caller cancellation`.
Exactly the reviewed refresh source and ledger were staged; names, normalized
blob content, whitespace and original root-lock hash passed. The index was
empty afterward, and no remote action occurred.

### F12 12b environment: reviewed contract test checkpoint

Root accepted `f12b-review.md` PASS after reading the two production modules,
frozen tests and actual evidence. This first checkpoint records the 381-line
frozen test corpus and approved standalone windows-sys 0.61.2 manifest/lock
addition, plus ledger. All 101 prior external package records remain identical;
only the new package/root dependency edge was added. Root manifest/lock,
schema and prior launch/source tests remain unchanged.

Final author run `e94ddf55-c2dc-44a3-98b7-d610e34cbdfe` had two passes and
nine intended behavior failures, with 24 prior tests filtered. The temporary
permissive scaffold was used only to establish that executable red; it is not
committed. The following production checkpoint registers these frozen sibling
tests with the final reviewed implementation. This test-only checkpoint does
not claim that an unregistered source file by itself ran the corpus.

The tests compare full map/native content using the independent Windows ordinal
oracle, validate every raw pair before scrub, exercise the 27-name/AWS canaries,
pseudo drives and exact bounds, and preserve unrelated challenge values.
Evidence and hashes are in `f12b-tests-evidence.md` and the lock comparison.
No tests were repeated, no unrelated path was staged and no push occurred.

F12b contract test checkpoint completed as
`f46813960eeabd2ece4c405ff1d182d015ea1c58`,
`test(covenant): define frozen Windows environment contract`. Exactly the frozen
test file, approved isolated manifest/lock and ledger were committed; staged
names/blob content/whitespace and reviewed/root-lock hashes passed, with an
empty index afterward. No permissive scaffold entered the checkpoint.

### F12 12b environment: accepted local representation

Root accepted independent `f12b-review.md` PASS for the final bounded caller-
supplied environment. Raw owned pairs are bounded and validated losslessly,
including pseudo drives and duplicates, before scrub. Fallible Windows ordinal
comparison drives sorting, duplicate detection and the finite 27-name/AWS
policy; all unexpected native results refuse. One retained post-scrub entry set
builds complete immutable JSON/native representations; empty is exactly two
NUL units. The two private production files contain 176 and 56 lines.

Run `a3aff6e2-e525-4a24-be3e-90d581c2047f` passed all eleven frozen tests in
0.373s, with 24 prior F10/12a tests filtered. Library-only Clippy and scoped
final formatting/check passed; no post-format test repeat. Native comparator
failure propagation is source-reviewed, not claimed empirically forced.
All protected input, test, schema and lock hashes remain unchanged.

Evidence is `f12b-implementation-evidence.md`; final production hashes are
`0184fa3b9527ee75d9baf9fdd2611c03ef619505bde6a3c4c3d5c66cec330b25`
and `6bf5386630bb4e6d2a157458a48bb199f60a78753aff3fb4e86c26c7443809fb`.
This checkpoint contains final environment/policy modules, frozen lib
registration and ledger only. Actual backend environment completeness,
F21 provider-name closure, native env/argv/process/Job/image, real G4 and
product/Bazel integration remain pending; this is not full F12 acceptance.

F12b final production checkpoint completed as
`37734f4c5fa050e8ebb212c3f9ea8622fb6dfb18`,
`feat(covenant): freeze scrubbed Windows environment pairs`: exactly final
environment/policy/lib and ledger, 269 added lines. All staged path/blob and
whitespace checks and original root-lock hash passed; index was empty afterward.
No permissive scaffold, prior-test repeat, unrelated file or push was included.
This hash annotation is retained for the next routine ledger checkpoint.

### F12c1 envelope test checkpoint

Root accepted the independently reviewed bounded envelope tranche on
2026-09-06. The separate test author froze seven Windows behavior tests in
`covenant/runtime/src/exec_envelope_tests.rs` (362 lines), SHA-256
`740052dd8a58ca716638476583e993d1bf431e065374a313b90e452d41a57790`.
They compare complete wire/native facts and owned input, exercise lexical
path/ParentDir and Unicode/NUL refusals, coupled argument/path/sandbox limits,
and the actual public 1,048,576-byte encoded boundary plus one-byte refusal.

Final scaffold red run `738ed43c-2465-480a-aec8-172ef3709d7e` executed all seven:
two passed, five failed behaviorally, with 35 prior tests filtered. Every
negative row is evaluated before aggregate assertions. The permissive scaffold
was only a temporary red harness and is not included in any checkpoint.
Evidence is `f12c1-tests-evidence.md` and `f12c1-frozen-hashes.json` under the
session evidence directory; root-approved API decisions are in
`f12c-api-plan.md`.

This test-only checkpoint intentionally adds the sibling source before its
module registration in the immediately following reviewed production stage.
The unregistered test file alone is not claimed to be a runnable red target;
the recorded actual-source scaffold run is the behavioral evidence. Exact
frozen API/registration, final production and independent review follow next.
No prior test, dependency, schema, lock, native implementation or remote action
belongs to this checkpoint. Protocol reply/deadline, native process/image/Job,
F21 and actual sidecar/full F12 acceptance remain pending.

### F12c1 envelope production checkpoint

The frozen test stage is committed locally as
`876f4b8d5ed9ca1384f0e14807ba81fd28e4effa`, exactly tests plus ledger, 397 added
lines. Root then accepted `f12c1-review.md` PASS for the separate 238-line
implementation and authorized only final source, frozen lib registration and
ledger in this production checkpoint.

The constructor now enforces native lexical and lossless UTF-16 bounds before
copying, derives the sole argv[0] from the one program, and retains the exact
owned environment/native fields. Both serializations use a fixed-capacity
checked 1 MiB Write sink; the inner bytes pass unchanged F10 decoding and drop
before the outer allocation. Errors retain no submitted content. This proves
bounded additional allocation and wire/native data binding, not filesystem
identity or native launch authority.

Run `f41e0d65-4026-4bb6-9d9f-c50fdce940eb` passed all seven frozen tests in
0.368s, with 35 prior cases filtered; scoped library Clippy with warnings denied
and final formatting/check passed. Independent source review confirmed bound
ordering, buffer growth limits, F10 validation and shared native/wire fields.
All twenty protected hashes matched; no test was repeated for review or after
formatting. Evidence is `f12c1-implementation-evidence.md`, its result JSON and
`f12c1-review.md` in the session directory.

Final source SHA-256 is
`f95e3abd58e3d78cf3a2a6fe20a691085862bfd7ea0f0b3fd45102ce27f04b77`;
frozen lib SHA-256 is
`d38bf8d4bbdb6b7199ad984bfba41fcaccdabbb3f0126f92c245601e2d2514ea`.
Protocol reply/deadline/completion, native backend/image/Job/quoting, F21,
G4 and full F12 remain separate. No permissive scaffold, dependency/lock,
unrelated concurrent F11 source, test repeat or remote push is included.

F12c1 final source/lib checkpoint completed as
`fdad55f63575d278d4c57a1e5af0d22c92a0f33a`, 280 added lines including ledger.
Reviewed hashes, staged paths/blobs and protected root lock passed; index empty.

### F11 companion wire contract checkpoint

Root accepted independent `f11-companion-review.md` PASS within the raw local
call/earliest parallel admission scope. This checkpoint contains the frozen
169-line wire test file plus ledger. It covers exact raw Function/Custom names,
forms and namespaces, all ToolSearch execution/id/argument cases, and the
ordinary positive normalization/search control. Whole accepted ToolCall data
and fixed typed refusal are checked without printing the argument canary.

Final three wire cases were genuinely red in run
`46bf60dc-b904-4a11-b059-117e5618b482`; ordinary baseline control passed in
`96f3270c-da79-4fd0-aaba-b6d7c692e257`. Frozen SHA-256 is
`f1cbb961202f55bc3ce9f68f740314664b3fe63d8e20c36e5bae00d5bf8f137b`.
Evidence is `f11-companion-tests-evidence.md`. Registration stays with the
following reviewed production checkpoint; this unregistered test source alone
is not represented as an executed target. Readiness tests are split next for
review size. Native authority, hosted response processing, product construction
and full F11/F21 remain separate. No F14, lock, unrelated source or remote action.

F11 companion wire test checkpoint completed as
`2b3b1418ebe253a2f8296d20a1f33545eec1b384`, 192 added lines including ledger;
staged paths/blobs and root lock passed, with an empty index afterward.

### F11 companion readiness contract checkpoint

This root-authorized second test checkpoint contains the frozen 350-line
readiness sibling plus ledger, SHA-256
`2d7a5e0f33f7b3a37428c9532995b7a870b7200eecd869fc1b01b8ad5814aca6`.
Three actual constrained cases require denial before attempted-call metadata,
parallel-runtime lookup, readiness construction/poll and cancellation/extension
lifecycle. A held execution lock, enabled metadata and pre-cancelled token make
those observations discriminate a late inner denial; admitted raw and cancelled
controls prove the detectors and ordinary accepted lifecycle still operate.

Final red run `0d9e462c-dfb2-4ddd-9fb5-8f48f54cd53b` executed all three and
failed behaviorally. The complete private core target and frozen dependencies
were retained in the approved full-source verification checkout; exactly 149
local version fields were reconciled there, with 1,232 external records fixed.
The original root lock remains unchanged. Independent review accepted this
fixture sensitivity and final implementation; evidence is in
`f11-companion-tests-evidence.md` and `f11-companion-review.md`.
Registration follows with reviewed production; no claim that this unregistered
file alone ran the tests. No test repeat, F14 source, dependency or remote action.

F11 readiness test checkpoint completed as
`ed8965c0aad1ec442022d7c0429e381adaf48032`, 375 added lines including ledger;
staged paths/blobs and root lock passed, with an empty index afterward.

### F11 companion: accepted raw and earliest parallel admission

Root accepted independent `f11-companion-review.md` PASS and authorized the
exact policy/router/parallel sources plus ledger. A shared private two-row
predicate now serves structured payload and borrowed raw ResponseItem adapters.
Raw ToolSearch refuses before parsing. Constrained Function/Custom calls keep
the original absent namespace; ordinary builds keep upstream normalization.
Parallel denial returns a ready error before all eager metadata, readiness,
task and cancellation work; accepted behavior retains its existing eager path.
Current nonlocal ResponseItem variants are exhaustively classified, without
granting hosted authority or adding a second allowlist. Frozen test registration
is included here; no permissive scaffold is committed.

Run `711c1742-31e1-44d1-bb22-6895b13c7efc` passed all fourteen constrained
cases, including the original eight; run
`5ee2d606-fc7a-42bf-8965-5e3e9342ec0d` passed the ordinary control. Each
filtered 2,341 upstream cases. Scoped tools/core library Clippy passed; final
exact-file formatting/check changed zero bytes. No review/post-format test
repeat. Actual full-source verification graph and all eleven protected path
pairs, source overlays, both locks and 1,232 external pins were independently
verified; the original root Cargo.lock never changed.

Evidence is `f11-companion-implementation-evidence.md`; final policy/router/
parallel SHA-256 values respectively are
`aaee23f4bf1ed99b9a7096a935c93133dbbdbe75a3dd28ab41e6b16acfe1329c`,
`7e374c396c9e37a6a52137e8f0bb9c5f49ffe60432b135ec8a1ea3d30b1b185c`,
`dc42f64e77d5343ffa6f23eef391ab5f030eb169872c089fd49c22282d33a0aa`.
This accepts the bounded local-call admission gap only. Effective product tools,
startup constructors, unsolicited hosted-item local persistence, real-hook/
repository-read E2E, native F12/F13 authority and full F21/F31 remain pending.
No concurrent F14 source, dependency, unrelated document or remote push included.

F11 companion production checkpoint completed as
`2204b2e63049dab575b6dd31f1c24fef8bc3faf0`, 154 changed lines including ledger;
exact staged paths/blobs, reviewed hashes and root lock passed; index empty.

### F12c2 exact reply test checkpoint

Root accepted independent `f12c2-review.md` PASS for the bounded protocol
foundation. This checkpoint contains the frozen 382-line reply test sibling
plus ledger, SHA-256
`b821ccd3aa95fa704aac92eea276cd440cfd7b0396045817a29cc35c7922e702`.
Eight tests cover every exact split and bytewise arrival, every proper prefix,
every byte-position mismatch and malformed/equivalent response, large/separate
overflow, sticky refusal, all completion facts and priority, exact deadline
boundaries, and real elapsed expiry through public wrappers.

Actual scaffold run `5f74b41b-11a8-4b59-9c9e-92c4f98e61a8` executed all eight:
one passed and seven failed behaviorally, with 42 prior tests filtered. The
same private transition methods are called by real-Instant public wrappers;
no test-only clock or parallel implementation supplies the boundary checks.
Evidence is `f12c2-tests-evidence.md` and its frozen result. The temporary
permissive scaffold is not committed. Registration follows with the separately
reviewed final source, so this unregistered file alone is not an executed target.
No previous test, schema, lock, F14 work or remote action enters this checkpoint.

F12c2 test checkpoint completed as
`7c4178caadd16b3a023bf4cd1cd25d7e2a1ff13c`, 406 added lines including ledger;
exact staged paths/blobs and root lock passed, with an empty index afterward.

### F12c2 reply: accepted bounded protocol foundation

Root accepted independent `f12c2-review.md` PASS and authorized the final
152-line decision_reply module, frozen lib registration and ledger. The parser
retains only an immutable real deadline, prefix length and optional fixed error.
Chunk length is checked before comparison; no output body is copied, allocated
or retained. First refusal wins permanently. Consuming finish requires, in
order, no prior refusal, no cancellation, an unexpired real deadline, completed
stdin closure, stdout EOF, observed exit zero and the complete exact ALLOW token.
Public wrappers call the same private transitions with real Instant observations;
there is no public fake clock, timeout override or ProtocolAllow constructor.

Run `743090a3-5d67-4c48-bf8b-5b210871ce85` passed all eight frozen tests in
1.132s, including the real deadline wait; 42 prior tests were filtered. Scoped
library Clippy with warnings denied and final exact-file formatting/check passed.
Formatting changed zero bytes and no tests were repeated for review or afterward.
Independent review checked all frozen/protected sources, schema and root/runtime
locks, exact priority, prefix arithmetic and constant additional allocation.

Evidence is `f12c2-implementation-evidence.md`; final source SHA-256 is
`357689dcd9625782590394997f398ebe5deb7b610a7ba1b7c212382974f2dd51` and
frozen lib is `6f53979d0d6379a8368072843694842fa845f09b9e70ea1e32cfa244eafe011c`.
Completion facts remain caller reports. Actual native pipe/process/cancellation
observation, verified image/ancestor identity, Job cleanup and private native
permit remain separate. Begin must precede future native hashing/creation.
No full F12/F21/sidecar acceptance, dependency/lock change, concurrent F14 source,
permissive scaffold or remote push is included in this checkpoint.

F12c2 final source/lib checkpoint completed as
`49663b267cad43973b8214700563b216018d5aa4`, 202 added lines including ledger;
exact staged paths/blobs and protected root lock passed, with an empty index.

### F14 File persistence fixture checkpoint

Root accepted independent `f14-persistence-review.md` bounded PASS and authorized
the frozen fixture/test and final production checkpoints. This first checkpoint
contains only the 306-line persistence support sibling plus ledger, SHA-256
`f86f0a613d9e3cbd35dfef0ab4c254f022a448ceedd6f8c419ec577372becdab`.
It creates isolated synthetic File auth state, retained-reader and read-only
controls, complete expected documents, fresh public-manager probes and explicit
cleanup checks without real credentials or keyrings.

The separate author's actual-source run
`e49ef3b7-b958-4f06-a0d1-bfb13c341856` executed both public cases: one passed
and the retained-reader case failed on in-place mutation, with eleven prior
cases filtered. The public test sibling and registration follow next to keep
complex checkpoints below 500 changed lines. This unregistered helper checkpoint
alone is not represented as an executed red target. Evidence is
`f14-persistence-tests-evidence.md` and the independently reviewed final evidence.
No test repeat, F21 work, production/dependency change or remote push is included.

F14 persistence helper checkpoint completed as
`6706e41d0409692e7cd41871415dfd49c8081a27`, 329 added lines including ledger;
exact staged paths/blobs and protected root lock passed, with an empty index.

### F14 File persistence public contract checkpoint

This root-authorized checkpoint contains the frozen 187-line public test sibling
and four registration lines. SHA-256 values respectively are
`73abd5bb9d606a0509822a39fe36b849dad4f42fd4dde03c260a19679749e50b`
and `d6302e0ebb912d48c9cfc61dcae01889433f58d0ba191bf10ce7e50f12bcc6c8`.
The retained-reader case requires the old open handle to retain complete old
bytes while public refresh saves the complete winner and a fresh manager probes
it. The read-only case requires public-save denial, consumed-refresh failure
with exact prior bytes, and successful explicit new-login recovery. Both check
one request, zero reuse, one acknowledgement, siblings and cleanup.

Initial run `e49ef3b7-b958-4f06-a0d1-bfb13c341856` was one pass/one fail;
the same frozen cases later both passed with the separately reviewed production
bytes. No tests are repeated for checkpointing. The following source checkpoint
records final two/thirteen-run evidence and the exact Clippy/Bazel limits.
This accepts the tests for bounded File replacement, not crash durability,
hostile namespace/ACL handling or full F14. No F21 source, dependency, root lock
or remote action belongs to this checkpoint.

F14 persistence public-test checkpoint completed as
`95dcab7eede7f33db010ed0313c34476043e9664`, 215 added lines including ledger;
exact staged paths/blobs and protected root lock passed, with an empty index.

### F14 File persistence: accepted replacement and failure preservation

Root accepted independent `f14-persistence-review.md` bounded PASS. The final
77-line private Windows backend wraps only valid opted-in File storage, inside
the unchanged permanent transaction guard. Same-parent create-new ordinary
temporary files receive complete pretty JSON, flush and sync before one
std::fs::rename. Success immediately disables temporary-name cleanup with an
infallible flag change; failure explicitly closes the owned temporary and
returns a fixed safe error preserving its kind. No target truncation, delete,
read-only bypass, app retry or fallible post-commit operation is introduced.
Default/unset, Auto/Keyring/Secrets, Ephemeral and existing refresh ownership
remain unchanged. Pinned Rust1.95/native fixture evidence confirms the observed
retained-reader replacement and read-only denial; its actual rename flags omit
IGNORE_READONLY. The earlier compile discovery and MoveFileEx-only failed runs
remain preserved, not relabelled green.

Focused run `7dddb702-d096-4272-9964-b5735b39a045` passed both new cases;
affected run `58673895-aff6-408c-b702-1eeb0a2f3daa` passed all thirteen fork-auth
cases with zero skips. Exact-three-source final formatting/check changed zero
bytes. Source review independently verified tested/final hashes and frozen
tests; no review or post-format test repeat. Evidence is
`f14-persistence-implementation-evidence.md` and its independent review.

The approved Windows normal tempfile dependency retains the existing dev edge.
Only codex-login -> existing tempfile was added to the auth isolation lock;
all 809 external full records and package identities remain unchanged. Root
Cargo.lock and MODULE.bazel.lock are byte-identical. Required Bazel regeneration
failed on the preserved codex-build-info 0.0.0 baseline path/version mismatch.
Scoped Clippy was unavailable because Cargo's nonmember feature resolver
panicked before analysis. No successful Bazel/Clippy validation is claimed and
no upstream graph workaround was attempted. Product closure remains separate.

Final backend SHA-256 is
`9c5f0f7d8980f7c1464f76e0d3bcc8bbbe56b9dc2d208e56b60438a95fc72ea6`;
the two factory/registration paths and both approved Cargo files match the
reviewed final hash manifest. Partial-temp-write/sync faults, crash durability,
hostile directory/name/ACL races, old security descriptor preservation,
Auto/Secrets atomicity, competing metadata/login/revoke semantics and complete
secret redaction remain separate test-first obligations. No full F14/F21,
native sidecar, unrelated concurrent source or remote push is accepted here.


### F21a: accepted immutable managed feature foundation

Root accepted independent `f21-profile-review.md` bounded PASS. Frozen tests
were checkpointed separately as `aa6ce04632079bdf5f69e5f6e2cea2a4cc98c8d1`,
exactly 490 added lines, with verified staged blobs and an empty index afterward.
Test SHA-256 is
`04b6ce6054352fac6331185f5772f65246b7598be5c31e0b59a24dc0d6ea262d`.
The four frozen registration lines follow in this production checkpoint; the
unregistered test-only checkpoint alone is not represented as an executed target.
The separate author established five unique behavioral reds plus an ordinary
positive control. Fixture/compile/formatter discoveries were excluded from red.

The private profile has 56 immutable rows: ShellTool on and 55 approved features
off. All managed constructors and mutations use the same retained policy.
Default/no-requirement/test conversion share an infallible constructor; external
requirements are validated before warnings, then compatible pins are merged.
Compiled pins are reapplied after dependency normalization. A shared existing-key
resolver rejects unknown, contradictory-alias and opposite mandatory settings
with fixed content-free InvalidData text. Ordinary feature behavior and explicit
managed-conflict semantics remain intact; no public API or schema was added.

Actual implementation run `4c74697b-247e-452d-b65c-6c54d0567ad2` passed all five
constrained cases, 2,355 unrelated tests filtered, 0.647s. Feature-off run
`ce6b8acd-d9de-409c-b71d-db5c6f56fb61` passed its one ordinary control, 2,342
filtered, 0.102s. Scoped actual-core library Clippy with warnings denied passed
in 6m00s. Exact three-file final format/check passed; one iterator-chain wrap
plus CRLF-to-LF normalization affected managed_features.rs. Independent reversal
of both changes reproduced the exact tested hash. No review/checkpoint or
post-format test repeat occurred. An accidental unchanged-snapshot run remains
separately logged and excluded from implementation evidence.

Final profile/managed/config hashes respectively are
`fa611537febc525f788d7ab311ffb9ae99077eacef43bfd77a88f23b6d9d3d6b`,
`b11a428dc2c1b1068350f5f73e7357908ba9ce6aed4f198e749b3efcb0eea2f7`,
`5b165b8970ed48694b29c0a0687295d17a7508da0657145a9af8d94a9e58ead6`.
Corrected evidence is `f21-profile-implementation-evidence.md`, SHA-256
`933167b6ef9db81148e61af2fa2a125e40df2bdd9b28c429e06835c655779bfa`;
final source/log/run provenance is `f21-profile-implementation-final-hashes.json`.
Both actual-source copies and protected inputs were verified. All other 25
config paths, including the next unregistered test draft, remain byte-identical.
The original root and MODULE locks never changed. The disposable full-source
graph retains only the approved 149 local version-field reconciliations, with
all 1,232 complete external package records identical. No dependency repair,
manifest edit, unrelated source, abandoned wrapper or remote push is included.

Acceptance covers ManagedFeatures and the tested ConfigBuilder/rebuild/bootstrap
paths only. Non-feature settings, role/model metadata, session raw reloads,
MCP/hooks/contributor/worker construction, effective tool specs, hosted effects,
provider/catalog/CLI product integration and native authority remain separate.
F21b1 is approved design with an unregistered draft, not tested implementation.
No full F21, F14, sidecar or fork completion follows from this checkpoint.

### Session prerequisite: verified actionlint 1.7.12

The official pinned Windows amd64 release was installed under the existing
session tool area after checking its release checksum row and GitHub asset
digests before extraction/execution. Archive SHA-256 is
`6e7241b51e6817ea6a047693d8e6fed13b31819c9a0dd6c5a726e1592d22f6e9`;
executable SHA-256 is
`54ca21be3de4c7cfa26914aa8b61bd76bf573ef3caac5f80d110558cdf241718`.
Version execution and session activation confirmed 1.7.12. Only ignored
load-env.ps1 gained the versioned PATH entry; existing tool pins and global
settings were preserved. Provenance is `actionlint-installation-evidence.md`
and `actionlint-installation.json`. No covenant release workflow exists yet,
so this records tool availability, not workflow validation, dispatch or F03
completion. The existing sync workflow was untouched.


### F21b1: accepted effective configuration materialization

Root accepted independent `f21-config-review.md` bounded PASS with 144 passing
integrity checks. The frozen 458-line tests are checkpointed alone as
`a1aaf5d8e5877810a3ed11c0c83b26aaf2caf58d`, SHA-256
`17256efec9c21a5a8cb325f5a886b6dd8bd442a684ac5ad9d3aa0c08d42e4b2f`.
That unregistered test-only checkpoint is not claimed independently executed;
the four exact registration lines accompany this reviewed production change.
The separate author obtained three genuine policy failures and an ordinary
positive control, with every fixture/provenance/security control reached.

The private config profile refuses nonempty mandatory hook requirements with
fixed InvalidData text before mutation or derived diagnostics. It normalizes
MCP/orchestrator, web, notify, tool-suggestion and input/plan channels before
materialization while retaining raw config layers and permission/auth values.
MCP setters remain normalized empty; web setters remain Disabled with the
existing validator and source retained. Both raw suggestion resolvers return
empty before ordinary layer traversal. Ordinary compiled behavior is preserved.
This is materialization and projection coverage, not lifecycle exclusion.

Actual constrained run `b4069779-142d-4b4d-8ba1-e801205137d7` passed all three
cases, zero skips/failures, 2,360 unrelated tests filtered. Feature-off run
`9beabc13-294f-4517-a7b6-471729522f85` passed one control, 2,343 filtered.
Scoped actual-core library Clippy with `-D warnings` passed in 29.58s. Exact
two-source final format/check passed with zero byte changes across all 28
config paths. No review/checkpoint or post-format test repeat occurred.

Final helper and config/mod.rs SHA-256 respectively are
`5bfe96d9fb15b7a4c344016dd5861f3534e770d888130d10a856eb32be86f64d` and
`f7f65e6fa00571b9dd1bd85f69b2f9d7ceeac22b747b85a94f01a693d31b9ec1`.
Evidence is `f21-config-implementation-evidence.md`, SHA-256
`984f1f50b951e9478c353682ec92ce60be6a9b5449d1f2f80be25310b6640361`;
its final hash manifest and `f21-config-review-integrity.json` retain exact
source/log/run provenance. Both source copies and protected inputs match.
Root/MODULE locks remain unchanged; the disposable source graph retains only
149 approved local version-field changes and all 1,232 external full records.

Direct public field mutation, role I/O/model metadata, raw session reload,
discovery/contributor/worker constructors, effective tool specs, hosted effects,
provider/catalog/CLI integration and native authority remain separate stages.
The paused role draft is unregistered and excluded. No dependencies, schema,
upstream repair, abandoned wrapper, original user documents or remote action
are included. This checkpoint does not complete F21, F12 or the full fork.


### F21 role-I/O: accepted early loader exclusion

Root accepted independent `f21-role-review.md` FINAL bounded PASS. The frozen
429-line test (SHA-256 below), four registration lines and five production
lines form one coherent checkpoint. The existing private profile initializes
agents.enabled=false; the final Config owner selects an empty role map before
constructing the role-loader future. Ordinary import/call/await remain exact.
No role parser, public API, metadata method, permission/auth or schema changed.

The separate author obtained genuine RED `8647a759-2382-4d26-bb2f-4cb6a76e7291`
after all six fixture/provenance/security controls; ordinary baseline
`326b5591-0e38-4c14-8436-d12e866e69e0` passed. Final constrained run
`5ede7a89-c428-4ba3-9b1d-7019a4f0ef61` passed 1/1, 2,363 unrelated tests
filtered; ordinary `f4d20660-0874-4825-8f92-da3b90523b8c` passed 1/1, 2,344
filtered. Both include all six input rows and complete role/counter objects.
Eleven filesystem forwards and 33 saturating counters prove early owned-role
I/O exclusion, with actual declared/discovered reads as ordinary controls.

Scoped core-library Clippy with warnings denied passed in 46.22s. Exact
two-source format/check passed with zero byte changes across 28 config paths.
Independent review verified final logs/source; no review/checkpoint or
post-format test repeat. Test, helper and config/mod.rs SHA-256 respectively:
`0d656585ab8e502c0019ed92e506540cf621dfb7a88c8d20f461f45cd3715649`,
`e69a40f1f01c056aba1a6a77bc8f28c89c939613430598c6fab85455397fb876`,
`a3b1c2bae03afca7be3792f9d310753842e586b275e27e610face23fc2dd984f`.
Evidence `f21-role-implementation-evidence.md` has SHA-256
`a67e00800e4657e4344ecc8d2cb35d101d7f3cbaa60dcf40074dbf17242d1bf7`;
its final manifest and review retain complete run/hash/provenance details.
Root/MODULE locks and protected source copies remain exact; the disposable
graph retains 149 prior local version changes and all 1,232 external records.

Acceptance covers initial Config agents/roles and owned role-path I/O only.
Direct field mutation, model metadata, Session/contributor/worker lifecycle,
effective tool specs, provider/catalog/CLI and native authority remain separate.
Concurrent runtime author files, deferred metadata draft, user docs and abandoned
wrapper are excluded. No dependency/Bazel repair, native experiment or remote
push is included; full F21/F12/fork acceptance remains pending.


### F12d1: accepted bounded native command-line representation

Root accepted independent `f12d1-review.md` bounded PASS with 41 integrity checks.
The two frozen test files are checkpointed alone as
`10409fe8177d48202a1c49feb5f7030c927d5df2` (391 added lines); that unregistered
checkpoint is not claimed independently executed. Their exact registrations
and the approved Windows-only Shell32 dev stanza accompany production here.
The separate author obtained two genuine existing-constructor failures in
`4f554ba8-5fd5-4ba3-aeed-4bc7313422b0`; four future-getter cases were frozen
unregistered and first executed only with the real implementation.

The private std-only encoder retains one command buffer from the envelope's
single bound program/argv. It rejects embedded program quotes, applies minimal
argv0 and CRT-style tail quoting, and checks every extension against 32,767
UTF-16 units including exactly one final NUL. The getter only borrows owned
data. Existing G4 JSON, environment, path/raw limits and safe errors remain.

Actual new-case run `5a8f08e7-04d2-45df-861d-4b04af107832` passed 6/6 with
50 unrelated tests filtered; affected envelope run
`06784942-a048-404f-9e62-11180c41efd9` passed all seven, 49 filtered. Library
Clippy with warnings denied passed in 2.16s. Final exact two-file format/check
changed zero bytes; no review/checkpoint or post-format test repeat occurred.
Encoder and envelope SHA-256 respectively are
`c800f3345379596943d07ccf95a5c08ff8bb17f3d606f696b116afd81bfc24d9` and
`4f0489d8eb118ed2a8818fa949bc8157ddf4aac59f60f4429f09def7941ef75a`.
Evidence `f12d1-implementation-evidence.md` SHA-256 is
`d439e34c0eba73a1476683b81c97e4aff1706f0d7860c0fe1eda9f92b69da217`;
its frozen manifest/review retain exact test/source/log hashes and run counts.

All 103 runtime lock records/raw bytes, old tests/lib/schema and root/MODULE
locks remain unchanged. A08 standalone/Bazel handling and author feature-view
limits remain explicit; no product release/link/Bazel proof is inferred.
Actual Shell32 parsing is representation evidence, not child argv delivery.
Image/ancestor identity, application-name binding at native creation, Jobs,
pipes/handle cleanup, cancellation and policy authority remain later stages.
Concurrent metadata source/tests, user docs and abandoned wrapper are excluded.
No F13 experiment, dependency refresh or remote push is included. This does
not complete F12, F21 or the fork.

### F21: accepted Config metadata override

Root accepted independent `f21-metadata-review.md` FINAL bounded PASS (292
integrity checks). The compiled profile returns `Some(Disabled)` from the
existing Config override owner; its two existing priority consumers and exact
ordinary body are preserved. The 118 appended test lines preserve the prior
429-line role-I/O fixture byte for byte. Direct agents-field mutations, managed
Collab/V2 setter attempts, cloning and all four model metadata values are covered.

Separate author RED `fccc077f-65ea-4226-a8c2-a947c9872b32` reached the final
whole-observation assertion; ordinary baseline
`1b3dbef5-c854-4f6c-9c5d-94876641649b` passed all four precedence rows.
Implementation runs `b2e52f0b-0392-4524-a0cf-981b2c741e1b` and
`f61dc017-a5a8-44fa-b2ab-996d397b121c` each passed 1/1, with 2,364 and 2,345
unrelated tests filtered respectively. Core-library Clippy with warnings denied
passed in 14m02s; exact one-source format/check changed zero bytes across all
28 config paths. No review/checkpoint or post-format test repeat occurred.

Final config source and complete test SHA-256 respectively:
`33eb06a9fef43069bac3fce3af9c2588c37feb905e9a8402889c53d42064aa7c`,
`abf9e98c18feca8dbac8748a8fdffa878b2f69b1c01f62fdc9a2107acf883982`.
Evidence `f21-metadata-implementation-evidence.md` SHA-256 is
`e213b97f7a0f089ec7e1e96d24c7dce9df8cd1e8508a0598abb579d1b3ddb1b4`;
the final manifest and independent review retain complete logs and provenance.
Root/MODULE/snapshot locks and all 1,232 external package records remain exact;
the isolated graph retains only its prior 149 local version substitutions.

Acceptance covers Config metadata methods, not Session/cached lifecycle,
MCP/hook/worker effects, tool registry, provider/catalog/CLI or native authority.
The unregistered MCP draft, user documents and abandoned wrapper are excluded.
No dependency repair, native experiment or remote push is included; full F21
and fork acceptance remain pending.

### F03: accepted MSVC environment preparation

Root accepted independent `f03-msvc-rereview.md` PASS for the new 202-line
helper. Frozen 346-line tests are checkpointed separately as
`4a6446692b40f44d9bd98e29315ee7fe5cb12b31`; that test-only commit requires the
helper here and is not claimed independently runnable. Initial missing-helper
discovery ran zero tests, not behavioral RED. Review then found a bounded nested
JSON query escaping as RecursionError; the separate regression reproduced it,
and the existing owner now normalizes that exception to its fixed refusal.
All seven amended tests passed in 11.586s. Scoped Ruff and final format/check
passed with zero formatting bytes; review/checkpoint repeated no tests or native
invocation. Final helper and test SHA-256 respectively:
`c6c6cf9b2d0b3db36c0531b3ae2c8574311f225c4673919b0c19cebacbef6a3a`,
`da96dbf11cc9567d861ef7fc1a49b63a0a200ec6662ad1e6164764491307241a`.

One local invocation from codex-rs validated the complete success schema and
observed VS 17.12.35527.113, MSVC 14.42.34433 and SDK 10.0.22621.0. The selected
Hostx64/x64/link.exe hash was
`f627ee8b9983af24d4b5ab6efab25f4a3be7a02bb09253aa2f5abab3c1fb2c6b`.
Its full environment capture stays private. `f03-msvc-correction-evidence.md`,
the frozen manifest and `f03-msvc-native-result.json` retain exact provenance.
Only the approved environment keys are returned; paths, x64 selection, query
bounds and linker bytes are checked. Existing helpers/tests/recipe/locks and
concurrent MCP changes remain preserved. No dependency, workflow or remote edit.

Acceptance is MSVC preparation only. The adapter inherits parent environment
and stderr; ordinary path checks do not retain filesystem identity. Native
downloads, product graph/default-feature integration, actual F02 build and
hosted workflow/artifact validation remain pending. No artifact attestation or
complete F03/fork acceptance is inferred, and nothing was pushed.
