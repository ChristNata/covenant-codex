# Covenant fork implementation ledger

Planning checkpoint: 2026-09-06. Source contract: `covenant_docs/master.md`,
its schema, review history, tasks, and repository-context pack. This ledger
records adaptations to the existing checkout; it does not certify an
implementation. All 15 phases below remain uncompleted at this checkpoint.

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

### F01 implementation checkpoint (corrections reviewed; root adjudication pending)

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
