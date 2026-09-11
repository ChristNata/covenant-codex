---
Feature: codex-fork
Scope: cross
Stack: rust
Rigor: hard
Status: draft
Created: 2026-09-05
Target: covenant-capital/codex
Dependencies: docs/master-plans/cross/child-runtime-native
---

# Master Plan: codex-fork

A minimal public fork of OpenAI Codex CLI that removes four
residuals. The fork lives in its own repository. This plan is
authored here so the harness can schedule it. `state.json` in
this workspace is bookkeeping for the harness only.

## What

Ship `covenant-capital/codex`, forked from `openai/codex` at the
latest stable `rust-v*` tag chosen at F01 (exact tag + commit
recorded in `covenant/UPSTREAM.toml`; the deep source audit was
done at `rust-v0.153.0` / `41e22fee981a63b3698df7ed36bad393cda24715`
and F00 re-anchors it at the chosen tag). The promoted artifact is
constrained `codex exec` / `codex exec --json` with three
fail-closed gates, a versioned Exec/Patch decide wire schema,
auth-home indirection, compiled-out ordinary hooks and MCP client,
  a stripped package, a C7-shaped inventory certificate, fork-repo
  context files, rebase CI that fails on drift, and a post-audit
  GitHub Release. The only harness-side code write is the S2
  `CODEX_PIN` row (`source = fork`, URL, digest). A schema copy in
  this workspace is what G4 consumes and is byte-identical to
  `fork:covenant/schema/decide-v1.json`. Landing is that GitHub
  Release (F33). The fork does not code-sign the binary. The repo is
  public; the harness pull path is token-free.

### Adoption sequence

`CODEX_PIN.source` default is `fork` once F33 publishes a
Release and F32 writes the pin. `official` is the documented
fallback: initial testing before that Release exists, and when a
rebase is mid-flight or broken. This is a harness-side pin
setting, not a mode inside the binary.

## Why

Stock Codex cannot fail-closed on writer effects: `PreToolUse`
fails open (issue #41979), MCP/app-server/TUI are extra
authorities, and `CODEX_HOME` couples login bytes to mutable
state. DECISIONS.md row 12 accepts a small localized fork when
rebases stay mechanically re-auditable. Row 15 pins refusals to
carry a remediation note. Row 2 is posture: permission-managed
workers, not a sandbox. Row 7: agent-facing MCP lives under fanin
— this fork compiles the MCP client out. Row 8 is harness-side
(`gh` wrappers); this fork publishes via its own GitHub Actions
Release job. Row 22: this plan's criteria are the fork repo's own
CI jobs and tests, plus the eight context files.

User posture: well-scoped permission-managed children. The fork
removes residuals; it does not invent a sandbox. The agent loop
stays upstream.

Anchors, all read before drafting:

- `docs/master-plans/cross/v2-program/refs/finalist-research/codex-covenant-dispatch-security-source-audit.md`
- `refs/finalist-research/openai-codex-cli-covenant-deep-dive-report.md`
- `refs/finalists/CODEX_CLI.md`, `refs/finalists/DECISIONS.md` §12
  and §15
- `docs/extras/CODEX_CLI_HOOKUP.md`, `docs/extras/CODEX_GUIDE.md`
- `docs/master-plans/cross/child-runtime-native/master.md` S2 / S4
  / C3 / C7 / G4 / P6 and `issue-native-homes-and-sealing.md`
- `explore/explore-03-v2-authority-surfaces.md` (incomplete dump;
  live contract taken from `decision.rs`)
- `docs/extras/RELEASE_PIPELINE.md`
- Live: `backend/covenant-utils/src/pinned_versions.rs`,
  `backend/covenant-cli/src/hooks/decision.rs`,
  `backend/covenant-cli/src/cli.rs` (`BashGuardClient`)
- This workspace `repo-context/` (eight files; Wave F4)

Corrected drift (prompt vs tree, cycle-1/2 vs rulings; this
amendment; task still stands):

- The deep source audit was at `rust-v0.153.0` /
  `41e22fee981a63b3698df7ed36bad393cda24715`. F01 does **not**
  freeze that tag. It roots at the latest stable `rust-v*` chosen
  at fork time and records exact tag + commit. F00 re-anchors the
  four seams and sink families at that tag; a newly introduced
  sink or tool identity fails F00 so later Produces are re-planned.
  The stock official pin in `child-runtime-native` S2 is a
  different artifact (flagged to bump `0.150.1` -> `v0.153.4`).
  This plan never overwrites an official pin until F32 fills
  `CODEX_PIN` with `source = fork`.
- `explore-03` on disk is an incomplete explorer dump. Hook
  decide contract used here is `decision.rs`: stdin JSON with
  `hook_event_name`, `DECIDE_DEADLINE = 1000ms`,
  `serialize_decision` emits raw `Decision` JSON, process exit 0,
  fail-closed arming. Exact ALLOW is `{"decision":"ALLOW"}`.
  `ALLOW_WITH_CONTEXT` is not ALLOW.
- `--client codex` is not in `BashGuardClient` today
  (`{ClaudeCode, Opencode, Cursor}`). S4 of
  `child-runtime-native` adds it. F12/F13 integration tests
  spawn the real sidecar with that flag.
- `hook_event_name` `Patch` and `Exec` are G4 of
  `child-runtime-native`. F10 owns `decide_v1`. G4 is being
  amended per the Cross-plan G4 bullet: parse `kind: exec |
  patch` of F10's exact schema; evaluate the complete
  post-overlay `env`; evaluate `volume_serial` / `file_index`
  / `pre_image_digest`; fork `Read`-deny on
  `$CODEX_AUTH_HOME/auth.json`. F12/F13 integration criteria
  run against the real sidecar built from that amended G4.
  Intra-plan `Depends On` cannot name G4 (unknown ID); it is a
  cross-plan hard precondition in Dependencies.
- `pinned_versions.rs` has no Codex row and no `source`
  discriminant today. S2 adds the schema. F32 writes
  `CODEX_PIN` (not a sibling `CODEX_FORK_PIN`):
  `source = fork`, URL, digest from the F33 Release.
- C7 has no Required Pattern block. F22 inventory output is the
  C7 certificate contract. C7, when `CODEX_PIN.source = fork`,
  invokes `codex --covenant-inventory` (dependency line only;
  this plan does not edit C7).
- Auth-load call site frozen as `codex-rs/core/src/auth.rs`. F00
  must confirm it. A different call site fails F00 so Produces
  are re-planned.
- Policy identity is the F12 launch contract
  (`COVENANT_DECIDER_PATH`, `COVENANT_DECIDER_SHA256`,
  `COVENANT_CHILD_MARKER`), set by `child-runtime-native` C3
  `codex_adapter.rs` `env()`. No PATH, no `COVENANT_CLI`.
- `child-runtime-native` Out-list forbids forks inside *that*
  plan. This plan is the separate authorized fork track
  (DECISIONS.md row 12). It does not edit that plan's files
  except the F32 `CODEX_PIN` row.
- Cycle-2 alignment: F12 `Depends On` includes F00; F32 pins
  come from F33; F03 stays non-publishing.
- This fork repo's criteria are its own CI jobs and tests. The
  only `covenant-cli` mentions are the runtime `hook decide`
  invocation, the launch-contract env the binary reads, and the
  F12/F13 sidecar fixture (a G4-green Windows asset pin).
- Last-sweep (FINAL-ROUND §F, DECISIONS rows 12 and 22, this
  amendment): F14's auth-home split depends on
  `child-runtime-native` **C3** setting `CODEX_AUTH_HOME` and the
  launch-contract env when `CODEX_PIN.source = fork`. `CODEX_HOME`
  lifetime is harness-decided via a C3/C6 redirectability probe
  (per-attempt disposable if fully boxed with no downside; else
  stable `CODEX_HOME` + redirected sub-dirs). F14 is compatible
  with both; it does not hard-code a disposable-home diagram.
  Adoption is fork-first once F33 publishes; `official` is the
  fallback. F12's decide object carries the complete
  post-overlay **scrubbed** process environment (set-equal to
  what the child sees). F13 identities include volume serial,
  file index, and pre-image content digest. F11 admits only
  identities named in this plan's F11 table (`exec_command` +
  custom `apply_patch`; no auto-admit of sink-clean `read`/`none`).
  F11 denies F00-named writer and `read`/`none` identities
  before the handler and does not use F31's
  `sink_instrumentation.rs`. F31 owns that file and consumes
  F00's authority inventory for sink-class agreement. Digest
  verification hashes an opened handle, not a path. Promotion
  requires a green F31 re-audit, the SHA-256 pin,
  `provenance.json`, and a GitHub build-provenance attestation
  the harness verifies before pinning; the fork does not
  code-sign the binary. Refresh uses a single-writer lock.
  `COVENANT_CHILD_MARKER` stays opaque presence-checked this
  cycle.
- Review-plan cycle 1 (this pass): live CRN C3 still sets
  `CODEX_HOME` and `CODEX_AUTH_HOME` to the same S5
  `config_dir`; live G4 still uses `env_allowlist` and omits
  Windows patch-identity fields. Both stay flagged
  preconditions (this plan does not edit CRN). F11 no longer
  classifies via F31's `sink_instrumentation.rs`. The G4
  citation is the Cross-plan G4 bullet, not
  `replan-d-g4-schema.md`. Sidecar fixture pins an attested
  G4 schema/semantics id; F32/F33 refuse without
  discriminating real-sidecar tests and (F32) landed C3+C6
  distinct-root separation.
- OQ4 resolved (user decision): no Authenticode code
  signing. This is a self-forked, single-user, local-only
  tool consumed only by the user's own harness; a code
  signature exists to prove provenance to third-party
  machines, of which there are none. Promotion integrity
  is the SHA-256 pin, a green F31 re-audit, and
  `provenance.json`. The signature is dropped; the
  verification is not.

Repo-context files in this workspace are the canonical copies
F40 lands byte-identically. This amendment synced them to the
  F10/F12/F13/F14/S2/F32 contract (case-insensitive complete
  exec `env` and closed secret-key set, patch
  `resolved_identities` conditionals, four-field `CODEX_PIN`,
  SHA-256 + re-audit + provenance (no code signing),
  `CODEX_AUTH_HOME` under either home model, compiled-out
  hooks, F21/F22 patch-index rows).
Do not copy a stale file and override later.

- Harness `repo-context/README-COVENANT.md` is the eighth file;
  the F40 Produce is `fork:README-COVENANT.md`. Criterion also
  requires a pointer from `fork:README.md` (upstream README is
  not replaced; a document-map block is added).

`Produces` paths inside the fork are written as `fork:<path>` so
each is one exact file. Those files are applied in
`covenant-capital/codex`, not this worktree. The schema copy
and F32 pin are the only this-repo writes.

## Dependencies

| Dependency | Status | Parallelism |
|---|---|---|
| `cross/child-runtime-native` S2 pin schema | planning in parallel | F32 serializes on `pinned_versions.rs`. This plan does not install, adapt, or certify the stock binary. |
| `cross/child-runtime-native` S4 `--client codex` | planning in parallel | F12/F13 integration tests spawn the sidecar-fixture `covenant-cli` as `hook decide --client codex`. Unit tests may stub. |
| `cross/child-runtime-native` G4 `Patch`/`Exec` amended (phase ID **G4**) | **must be amended** — see Cross-plan dependencies below | F10 owns `decide_v1`. The amended G4 must parse `kind: exec \| patch` of that exact schema and evaluate every field: the complete post-overlay exec `env` (not an `env_allowlist` snapshot), and patch `resolved_identities` including `volume_serial`, `file_index`, and `pre_image_digest`. The `Read`-kind auth-deny path must target `$CODEX_AUTH_HOME/auth.json` for the fork arm. **If G4 is not amended, F10/F12/F13 hardening is inert because the decider ignores the extra fields.** Version bumps require both plans. F12/F13 integration criteria pin a G4-green Windows `covenant-cli` Release asset (URL + SHA-256 + attested G4 schema/semantics id in `fork:covenant/sidecar-fixture.toml`) and spawn that exe as `hook decide --client codex`. Discriminating real-sidecar pairs (a complete-env pair whose differing key is chosen at test time; and each of `volume_serial`, `file_index`, `pre_image_digest` independently; plus a `Read` pair on `$CODEX_AUTH_HOME/auth.json`) must change the decision, so a partial G4 cannot pass. They cannot go green until that asset is the sidecar under test. Intra-plan `Depends On` cannot name G4. This plan does not edit G4. |
| `cross/child-runtime-native` **C3** | planning in parallel; **must be amended** with C6 | F14's auth-home split and F12's launch contract depend on C3. When `CODEX_PIN.source = fork`, C3 Produce `backend/covenant-cli/src/dispatch_run/codex_adapter.rs` `env()` sets `CODEX_AUTH_HOME` to the stable login home and the launch-contract env `COVENANT_DECIDER_PATH`, `COVENANT_DECIDER_SHA256`, `COVENANT_CHILD_MARKER`. `CODEX_HOME` lifetime is **not** hardcoded here: C3/C6 run a redirectability probe and either use a per-attempt disposable `CODEX_HOME` or keep stable `CODEX_HOME` + redirected sub-dirs. When the probe permits, `CODEX_AUTH_HOME ≠ CODEX_HOME` (live C3 today sets both to S5 `config_dir`; that same-root coupling is the gap). When `source = official`, the four fork keys are absent. Named as a dependency; this plan does not edit C3. |
| `cross/child-runtime-native` C2 overlay | planning in parallel; **must be amended** | Direct-file `deny_read` follows `$CODEX_AUTH_HOME/auth.json` on the fork arm, not `$CODEX_HOME/auth.json`. Named as a dependency; this plan does not edit C2. |
| `cross/child-runtime-native` C7 inventory | planning in parallel; **must be amended** | F22 output is the C7 certificate contract. When `CODEX_PIN.source = fork`, C7 **must invoke** `codex --covenant-inventory` and treat that JSON as the certificate body. Dependency line only; this plan does not edit C7. |
| `cross/child-runtime-native` P6 secret canary | planning in parallel; **must be amended** | Compensating control for obfuscated shell reads of the auth store. The canary follows `$CODEX_AUTH_HOME/auth.json` on the fork arm. F1.4 (shell-mediated auth reads) stays accepted on the fork arm. F14 proves fork-controlled sinks. This plan does not edit P6. |
| Wave-4 `hook decide` | landed | Do not reuse Codex `PreToolUse`. Do not extract `evaluate`. |

Inside this plan: F01 then F00, F02, and F40 (Produces
disjoint except F40 owns `COVENANT_PATCHES.md`). F10 after
F01. F11, F12, F14, F03, F21 may run in parallel after F00+F02
where Produces are disjoint. F12 `Depends On` F00. F13 waits on
F12 (shared gate module). F22 waits on F11 and F21. F31 waits
on the four patches plus F22 and F40. F33 waits on F03 and F31.
F32 waits on F33. F03 never publishes. Publication of a
promotable Release is F33.

### Cross-plan dependencies to amend in `child-runtime-native`

This plan does not edit that workspace. These amendments must
land there or the named fork contract is inert / incomplete.

- **G4** — take the complete post-overlay exec `env` (drop
  `env_allowlist`); add and evaluate `volume_serial` /
  `file_index` / `pre_image_digest` in `resolved_identities`;
  `Read`-deny path -> `$CODEX_AUTH_HOME/auth.json` for the fork
  arm. MUST land or F10/F12/F13 hardening is inert because the
  decider ignores the extra fields. This plan does not edit G4.
- **C3 + C6** — add the `CODEX_HOME` disposability probe and
  branch C3 on its result (per-attempt disposable if fully
  redirectable with no cons; else keep stable `CODEX_HOME` +
  redirected sub-dirs). When the probe permits,
  `CODEX_AUTH_HOME ≠ CODEX_HOME` (today live C3 sets both to
  S5 `config_dir`). F32 adoption refuses until this amendment
  lands and a real-adapter separation test proves auth I/O
  stays under the stable auth root while probe-classified
  mutable paths follow the selected home model. This plan
  does not edit C3/C6. (user decision 1)
- **C2** — `deny_read` targets `$CODEX_AUTH_HOME/auth.json` on
  the fork arm, not `$CODEX_HOME/auth.json`. This plan does
  not edit C2.
- **S2** — bump the official Codex pin `0.150.1` -> `v0.153.4`;
  ensure the `source: official|fork` discriminant exists.
  (user earlier decision)
- **C7** — when `CODEX_PIN.source = fork`, C7 **must invoke**
  `codex --covenant-inventory` and treat that JSON as the
  certificate body (not a fixture `codex exec --json`
  capture). Certificate fields still match F22. This plan
  does not edit C7.
- **P6** — canary auth path follows `$CODEX_AUTH_HOME/auth.json`
  on the fork arm. F1.4 stays accepted on the fork arm
  (shell-mediated auth reads remain residual). This plan
  does not edit P6.
- **Follow-up plan** — marker-as-capability (nonce +
  WorkerContract) across CRN C3/G4/`decision.rs` and the fork
  decide envelope. Not this cycle. (user decision 2)

## Scope

### In

- Public fork repo `covenant-capital/codex` from the F01-chosen
  latest stable `rust-v*` tag (F00 re-anchors the audit)
- F00 map of every upstream file later phases edit, including
  ordinary-hook and MCP-client seams
- Windows x64 `codex.exe` build; the promotion pin is the
  SHA-256 of the F33 asset
- CI that builds and tests the exe; promotable Release only
  via F33 after F31
- Eight fork-repo context files (Wave F4) plus a README pointer
- Versioned Exec/Patch decide wire schema v1, plus a copy in
  this workspace for G4
- Gate 1: deny-by-default compiled tool admission at
  `ToolRegistry::dispatch_any_with_terminal_outcome`, with an
  identity-to-effect table; no fallthrough to `pre_tool_use`
- Gate 2: `covenant-cli hook decide --client codex` kind `Exec`
  at `UnifiedExecProcessManager::open_session_with_prepared_exec_env`,
  carrying the complete post-overlay process environment
- Gate 3: same, kind `Patch`, at `ApplyPatchRuntime::run`, with
  resolved ancestor/link/junction/`\\?\` identities plus volume
  serial, file index, and pre-image content digest
- Launch contract env read once at startup
- `CODEX_AUTH_HOME` as the login-store root under either
  harness home model; Codex owns credential I/O; direct-file
  `deny_read` on the auth directory; F12 argv-token deny
- Packaging that omits app-server, TUI, MCP client, multi-agent,
  plugins, Code Mode, hosted web, ordinary command/MCP hooks,
  plus a pinned model catalog
- `--covenant-inventory` emitting the C7 certificate contract
- Re-audit CI against upstream on every rebase, including a
  golden ungated-sink fixture that must fail
- Harness `CODEX_PIN` row `source = fork` consuming the F33
  Release

### Out

- Any harness adapter, doctor, seed, FE, or routing work except
  the F32 `CODEX_PIN` row
- Editing `child-runtime-native` G4/C7/C3/S2/P6 files (this plan
  only *defines* the schema, launch-contract env names, and pin
  those phases consume)
- The stock `openai/codex` official pin (CRN S2, flagged to
  bump `0.150.1` -> `v0.153.4`) and its certificate until F32
  replaces `CODEX_PIN` with the fork arm
- Grok Build, Zero, Worker, CLIProxyAPI, Covenant-owned OAuth
- Copying, reading, or projecting credential bytes in Covenant
- Codex TUI, app-server, MCP, Code Mode, plugins, hosted web,
  `write_stdin`, subagents — as shipped features
- Reusing stock `PreToolUse` / `codex-hooks` as enforcement
- Rewriting the reasoning, turn, or agent loop
- Interactive terminals (would add a fourth gate)
- Linux/macOS builds
- Harness `release.yml` / NSIS / sidecar changes
- Promoting Codex writer roles inside `child-runtime-native`
- PATH lookup or `COVENANT_CLI` for the policy runner
- Runtime allowlist extension files or env
- Privilege-boundary redesign of the auth store (no separate
  user for Codex; obfuscated shell reads stay residual)
- Harness lifecycle commands in fork CI, criteria, or landing
- Replacing upstream `README.md` wholesale

## Phases

### Wave F0 — Fork bootstrap

Repository, integration-file map, Windows digest recipe, test-only
release CI. No security gates yet. No promotable Release.

#### Phase F01 — Repository and upstream pin

**Rigor:** medium

**Depends On:** []

**Scope:** Create `covenant-capital/codex` from the latest
stable `rust-v*` tag chosen at fork time and land the upstream
map. Do not patch gates yet. Do not write the patch index (F40
owns it).

**Produces:**

- `fork:covenant/UPSTREAM.toml`

**Key Behaviors:** The fork's first Covenant commit is the
chosen latest-stable upstream `rust-v*` tag plus this file.
`UPSTREAM.toml` records that exact tag, its 40-hex commit,
upstream URL, license SPDX found in the tag tree, and the
rust-toolchain path (do not create a second toolchain file
later). The deep source audit was performed at `rust-v0.153.0`
/ `41e22fee981a63b3698df7ed36bad393cda24715`; F00 must
re-anchor that audit at the chosen tag before later phases
edit. Retain upstream LICENSE/NOTICE bytes unmodified.
Covenant attribution lives in this file and in Wave F4. The
repo is public (`covenant-capital/codex`); the harness pull
path stays token-free.

**Skills Needed:** `rust-general`

**Phase Success Criteria:**

1. `covenant/UPSTREAM.toml` records `tag` as the chosen latest
   stable `rust-v*` value and `commit` as that tag's exact
   40-hex object name.
2. `git merge-base HEAD <recorded-tag>` equals the recorded
   commit.
3. Upstream `LICENSE` and `NOTICE` bytes are unmodified.

#### Phase F00 — Integration-file scout

**Rigor:** medium

**Depends On:** [F01]

**Scope:** Freeze the exact upstream files later phases edit.
Do not patch behavior.

**Produces:**

- `fork:covenant/INTEGRATION-FILES.toml`
- `fork:covenant/AUTHORITY-INVENTORY.toml`

**Key Behaviors:** Grep the chosen-tag tree and write one toml
that maps each seam to an existing exact path. The frozen
defaults below are the later-phase Produces unless a phase
explicitly dropped a path (F21 prefers compile/profile/flags
over editing hook/MCP impl files; those four rows stay scouted
for a possible F21 re-plan). F00 confirms each non-absent path
exists at the tag and is the named seam. A miss on a
later-phase Produce fails this phase so Produces are re-planned
— do not edit a file that is not a later Produce.

**Audit re-anchor:** re-confirm the source security audit (four
seams F11/F12/F13/F14 plus sink families process, filesystem,
network, hosted, MCP) at the F01-recorded tag. A newly
introduced sink family or tool identity, or a moved four-seam
path, fails this phase and forces a re-plan of later Produces.
Forking a newer tag without this re-anchor would bolt gates
onto unaudited code.

**Authority inventory:** `AUTHORITY-INVENTORY.toml` is the
commit-bound completeness artifact for this tag. It records the
F01 40-hex `commit` (the same value as `UPSTREAM.toml` `commit`)
in a named field so F31 can compare them. It enumerates
every model and non-model constructor and every process,
filesystem, network, hosted, MCP, hook, and generated-code
sink, each with a source anchor and a reviewer disposition.
It also records the closed secret-key set: the four control
keys (`COVENANT_DECIDER_PATH`, `COVENANT_DECIDER_SHA256`,
`COVENANT_CHILD_MARKER`, `CODEX_AUTH_HOME`) plus every
provider/login key present at the tag (or a documented prefix
set). It includes the executable audit procedure a reviewer
or F31 CI walks against this commit. F31 consumes this
inventory; it does not rediscover the set. A constructor or
sink family missing from the inventory fails this phase so
later Produces are re-planned.

| Seam | Frozen path |
|---|---|
| CLI parser/entry | `codex-rs/cli/src/main.rs` |
| config-resolution | `codex-rs/core/src/config/mod.rs` |
| managed-requirements | `codex-rs/core/src/config/mod.rs` |
| model-catalog | `codex-rs/core/src/config/mod.rs` (`multi_agent_version_for_model`) |
| build-metadata | absent at tag; F22 introduces `covenant_inventory.rs` |
| auth-load / refresh / write | `codex-rs/core/src/auth.rs` |
| Gate 1 | `codex-rs/core/src/tools/registry.rs` |
| Gate 2 | `codex-rs/core/src/unified_exec/process_manager.rs` |
| Gate 3 | `codex-rs/core/src/tools/runtimes/apply_patch.rs` |
| hosted / collab / one-shot exec registration | `codex-rs/core/src/tools/spec_plan.rs` |
| feature registry | `codex-rs/features/src/lib.rs` |
| ordinary command hooks (scout; F21 edits only if SC5 proves residual reachability) | `codex-rs/hooks/src/events/pre_tool_use.rs` |
| hook runtime (scout; same F21 rule) | `codex-rs/core/src/hook_runtime.rs` |
| MCP stdio launcher (scout; same F21 rule) | `codex-rs/rmcp-client/src/stdio_server_launcher.rs` |
| MCP connection manager (scout; same F21 rule) | `codex-rs/codex-mcp/src/connection_manager.rs` |

Also list every registered tool identity from `spec_plan.rs`
with a scout-only provisional effect class
(`none` / `read` / `process` / `filesystem` / `network` /
`hosted` / `MCP`). That list is a map of names, not admission
authority. F11 does **not** auto-admit a `read`/`none`
identity from this list. F11 denies every F00-named writer
identity (MCP, hosted, web, `write_stdin`, and the other
process / filesystem / network / hosted / MCP names other
than the two admitted rows) plus any F00 `read`/`none`
identity, with no dependence on F31.
`fork:covenant/tests/sink_instrumentation.rs` is F31-only.

**Skills Needed:** `rust-general`

**Phase Success Criteria:**

1. `INTEGRATION-FILES.toml` names each seam in the table; every
   non-absent later-phase Produce path exists in the tag tree.
2. The auth-load / refresh / write path is
   `codex-rs/core/src/auth.rs` (or this phase fails).
3. The tool-identity list includes `exec_command` and custom
   `apply_patch` and does not omit a `spec_plan.rs` registration.
4. Hook and MCP seams are recorded present or absent at the
   frozen paths. Absence is not a phase failure.
5. The four gate seams and the sink families from the
   `rust-v0.153.0` audit are re-confirmed at the F01 tag. A
   newly introduced sink family or tool identity, or a moved
   four-seam path, fails this phase so later Produces are
   re-planned.
6. `AUTHORITY-INVENTORY.toml` enumerates every model and
   non-model constructor and every process, filesystem,
   network, hosted, MCP, hook, and generated-code sink at the
   F01 commit, each with a source anchor and reviewer
   disposition, plus the closed secret-key set and the audit
   procedure, and records the F01 40-hex `commit` (equal to
   `UPSTREAM.toml` `commit`) in a named field. A missing
   constructor, sink family, provider/login key, or commit
   field fails this phase so later Produces are re-planned.

#### Phase F02 — Windows toolchain and digest recipe

**Rigor:** medium

**Depends On:** [F01]

**Scope:** Pin the Windows toolchain and the exact cargo
invocation used to produce `codex.exe`. Do not publish. Do not
run a local two-build ritual.

**Produces:**

- `fork:covenant/windows-repro.toml`

**Key Behaviors:** Record rustc from the F01 toolchain file
(channel plus date or hash), cargo version, target
`x86_64-pc-windows-msvc`, package/bin name, SHA-256 method
(`Get-FileHash` equivalent), `node = false`, runner image
`windows-2022` (not `windows-latest`), and
`msvc = "windows-2022 VS Build Tools"`. Prefer byte-for-byte
reproducibility; if MSVC timestamps prevent it, set
`repro = "digest-recorded"` so F03 can identify the CI
artifact. That digest is never a promotion pin. The
promotable digest is the SHA-256 F33 captures of the built
exe. Windows-only. No npm shim. Byte identity of the F03
CI `codex.exe` is F03's job, not two local runs.

**Skills Needed:** `rust-general`

**Phase Success Criteria:**

1. `windows-repro.toml` names target `x86_64-pc-windows-msvc`,
   one cargo invocation that produces `codex.exe`, rustc from
   the F01 toolchain file, `node = false`, and runner
   `windows-2022`.
2. The toml sets either `repro = "byte"` or
   `repro = "digest-recorded"`.

#### Phase F03 — Release CI without a promotable Release

**Rigor:** medium

**Depends On:** [F02]

**Scope:** GitHub Actions on `windows-2022` that installs the
F02 toolchain, runs the F02 cargo invocation, hashes the exe,
and uploads workflow artifacts. Do not create a GitHub Release.
Do not edit harness workflows. Tag-push publication is F33.

**Produces:**

- `fork:.github/workflows/covenant-release.yml`

**Key Behaviors:** `workflow_dispatch` only for this phase's
success. Checkout the fork, install the F01 toolchain file,
run the F02 cargo invocation, hash the exe, upload
`codex-x86_64-pc-windows-msvc.exe` and `codex.exe.sha256` as
workflow artifacts. The digest file is one 64-hex line. Fail
if the exe is missing or the hash is not 64 hex. Pin every
Actions step to a full commit SHA (no floating major tag).
Pin Windows SDK/MSVC to that image's VS Build Tools. No `v*`
tag job publishes a Release here.

**Skills Needed:** `rust-general`

**Phase Success Criteria:**

1. A `workflow_dispatch` run on `windows-2022` uploads both the
   exe and a 64-hex `codex.exe.sha256`.
2. The workflow YAML pins every `uses:` to a 40-hex SHA and
   does not create a GitHub Release.
3. No `v*` tag job in this file publishes a promotable Release.

### Wave F1 — Schema and four patches

Each patch phase is one residual. Hard rigor. Each carries an
upstream-rebase test: the named call site still exists and the
Covenant insertion is still before the first side effect.

#### Phase F10 — Exec/Patch decide wire schema v1

**Rigor:** hard

**Depends On:** [F01]

**Scope:** One versioned JSON schema for kinds `exec` and
`patch`. No runtime gate yet. This file is the single source
G4 copies.

**Produces:**

- `fork:covenant/schema/decide-v1.json`
- `docs/master-plans/cross/codex-fork/schema/decide-v1.json`

**Key Behaviors:** Both files are byte-identical. That
byte-identity is the F10 gate: the workspace copy is the
canonical G4 input and must match
`fork:covenant/schema/decide-v1.json` exactly. Kind `exec`
carries program, argv array, cwd, the **complete effective
process environment** (post-overlay and post-scrub, as the
spawned process will see it — not an allowlist subset; no
`env_allowlist` field), and sandbox/network/TTY flags. No
lossy shell re-stringify of argv. Secret-shaped values stay in
the decide object; fork logs redact them.
Kind `patch` carries resolved absolute paths, per-file
`add|update|delete|move`, structured hunks, permissions,
resolved existing-ancestor identities, symlink/junction
identities, `\\?\` normalization, Windows file identity
(`volume_serial` + `file_index`), and the content digest of
each file pre-image (`pre_image_digest` required for
`update|delete|move` and `kind == file`; optional for `add`
and directory kind; `target` required for
`symlink|junction`; ancestor `win32_normalized` required).
`child-runtime-native` G4 (amended) must evaluate this exact
schema, including every env pair. This workspace copy is the
harness-plan input; do not edit G4 here. Both copies stay
byte-identical.

**Required Pattern:**

```text
DecideV1 {
  version: 1,
  kind: "exec" | "patch",
  exec?: { program, argv: [string], cwd,
           env: {k:v complete post-overlay post-scrub},
           sandbox, network, tty },
  patch?: {
    paths: [abs],
    operations: [{path, op: add|update|delete|move,
                  destination?, hunks, pre_image_digest?}],
    permissions: {sandbox, write_roots: [abs], network, tty},
     resolved_identities: [{
       path, kind: file|directory|symlink|junction,
       target?, ancestor_identities: [{path, kind, target?,
         win32_normalized, volume_serial, file_index}],
       win32_normalized,
       volume_serial, file_index,
       pre_image_digest?
     }]
  }
}

G4 Event envelope (F12/F13 stdin):
  { hook_event_name: "Exec"|"Patch",
    cwd, command?, path?,
    decide_v1: DecideV1 }
  not: ExecRequest/ApplyPatchRequest fields beside hook_event_name
```

**Skills Needed:** `rust-general`

**Phase Success Criteria:**

1. Both Produces exist, are byte-identical, and set
   `"version": 1`.
2. The exec branch requires `program`, `argv` as an array, `cwd`,
   `env` as the complete post-overlay map, `sandbox`, `network`,
   `tty`. The schema has no `env_allowlist` field.
3. The patch branch requires resolved absolute `paths`, per-file
   `operations`, structured `hunks`, `permissions`, and
   `resolved_identities` with ancestor/link/junction kind,
   `win32_normalized` (identity and every ancestor),
   `volume_serial`, and `file_index`. `pre_image_digest` is
   required on `update|delete|move` operations and on
   `kind == file` identities; it is not required for `add` or
   directory kind. `target` is required for `symlink|junction`.
   Both schema copies encode those `if`/`then` conditionals
   and stay byte-identical.

#### Phase F11 — Tool identity admission

**Rigor:** hard

**Depends On:** [F00, F02]

**Scope:** Deny-by-default compiled allowlist at registry
dispatch. Identity plus effect class. No exec/patch semantics.
No runtime extension path. No fallthrough to upstream
`pre_tool_use`.

**Produces:**

- `fork:codex-rs/core/src/covenant_admission.rs`
- `fork:codex-rs/core/src/tools/registry.rs`

**Key Behaviors:** In
`ToolRegistry::dispatch_any_with_terminal_outcome`, before
`pre_tool_use` and before `handle_any_tool`, admit only exact
canonical tool identities from a Rust const table in
`covenant_admission.rs`. Unknown, namespaced, or unparsable
identity is a terminal non-retriable `CovenantDenied`, not a
sandbox retry. After admission, do **not** call upstream
`pre_tool_use`. User/project config, env, and any planted
writable manifest cannot widen the list. `COVENANT_TOOL_ALLOWLIST`
does not exist.

Identity-to-effect table (frozen in this plan). Effect classes
are `none` / `read` / `process` / `filesystem` / `network` /
`hosted` / `MCP`. This phase freezes the two-row table and
denies every other F00-named identity before the handler.
It does not own or consult
`covenant/tests/sink_instrumentation.rs` (F31-only). Sink-clean
is **necessary but not sufficient**: F00 may classify an
identity `read`/`none`; F11 does **not** auto-admit it. F31
later agrees sink class with this table.

| Identity | Form | Effect | Dominating gate |
|---|---|---|---|
| `exec_command` | function | process | F12 |
| `apply_patch` | custom | filesystem | F13 |

The admitted const table is exactly these two rows. No `read`
or `none` identity is plan-approved this cycle. A newly
discovered upstream read tool is DENY until reviewed and added
by an explicit plan amendment that names it in this table.
Any future admitted `read` identity must carry a pinned
`deny_read` on the auth directory (`CODEX_AUTH_HOME` when set,
else `$CODEX_HOME/auth.json` parent). No identity of class
`process`, `filesystem`, `network`, `hosted`, or `MCP` other
than the two rows above may be admitted. Never admit
`write_stdin`, MCP namespaces, extension/dynamic identities,
agent tools, `request_permissions`, or hosted specs.

**Required Pattern:**

```text
dispatch_any_with_terminal_outcome:
  if !covenant_admission::is_admitted(tool_name) {
    return terminal CovenantDenied { code: unknown_tool }
  }
  // never call upstream pre_tool_use
  handle_any_tool(...)

ADMITTED const table (closed; plan amendment to add):
                      exec_command -> process,
                      apply_patch (custom) -> filesystem
no auto-admit of F00 read/none classifications
future read identities: deny_read(auth_directory)
F00-named writers + F00 read/none: deny before handler
sink-class agreement: F31 only; this phase does not
                      use sink_instrumentation.rs
sink-clean is necessary, not sufficient
```

**Skills Needed:** `rust-general`

**Phase Success Criteria:**

1. Dispatch of an identity absent from the const table returns
   `CovenantDenied` and does not invoke the handler.
2. Dispatch of `exec_command` and custom `apply_patch` reaches
   the handler (admission only; no decide call yet) and does
   not invoke `pre_tool_use`.
3. A planted writable allowlist file, `COVENANT_TOOL_ALLOWLIST`,
   or user `config.toml` tool list does not admit an extra
   identity.
4. An identity not named in this phase's table, including an
   F00-classified `read` or `none` identity, is denied before
   its handler runs.
5. A rebase test fails if
   `dispatch_any_with_terminal_outcome` is missing or the
   admission call is no longer before `handle_any_tool`.
6. Dispatch of a named F00 writer identity (MCP, hosted, web,
   `write_stdin`, and every other F00-listed process /
   filesystem / network / hosted / MCP identity other than
   `exec_command` and custom `apply_patch`) and dispatch of
   any F00 `read`/`none` identity are denied before the
   handler. This criterion does not use
   `sink_instrumentation.rs`.
7. A newly discovered upstream read tool not named in this
   table is denied even if F00 classified it `read` or `none`.
8. Against the F02 exe with this phase's admission table, a
   deterministic fixture-provider acceptance test completes a
   repository-read turn (`turn.completed` carrying the known
   file content) using only `exec_command` and custom
   `apply_patch`. The captured tool plan contains no denied
   native read/view identity and no excluded surface. Failure
   fails this phase so the two-row table is re-planned; do
   not freeze it.

#### Phase F12 — Exec gate

**Rigor:** hard

**Depends On:** [F00, F02, F10]

**Scope:** Dedicated `covenant_gate` client plus the
process-start insertion. Do not gate patches here.

**Produces:**

- `fork:codex-rs/core/src/covenant_gate.rs`
- `fork:codex-rs/core/src/unified_exec/process_manager.rs`
- `fork:covenant/sidecar-fixture.toml`

**Key Behaviors:** New module talks to the launch-contract
absolute path with arguments `hook`, `decide`, `--client`,
`codex` only. Do not call `codex-hooks`. At process start,
read `COVENANT_DECIDER_PATH`, `COVENANT_DECIDER_SHA256`, and
`COVENANT_CHILD_MARKER` **once** into an immutable
`LaunchContract`. Never re-read process env. Before every
decide spawn, open the policy executable, hash **that opened
handle** (not a path re-hash), hold a no-replace/delete lock
through process creation, and verify the created image is the
same object as the hashed handle and equals the frozen
SHA-256, with the marker present. Mismatch or missing
contract: deny, zero effect. Model-visible tools cannot change
process env (the frozen contract is what decide uses). No PATH
lookup. `COVENANT_CLI` is ignored if set.

At entry of
`UnifiedExecProcessManager::open_session_with_prepared_exec_env`,
after the final `ExecRequest` exists and before `backend.start`
or `codex_sandboxing::spawn_process`, overlay the child
environment into **one immutable Windows environment block**.
That block is the single source for both the frozen
`decide_v1.exec.env` and the spawned process. Canonicalize
keys case-insensitively; reject case-colliding or invalid
entries before freeze. **Scrub** the closed secret-key set:
the four control keys (`COVENANT_DECIDER_PATH`,
`COVENANT_DECIDER_SHA256`, `COVENANT_CHILD_MARKER`,
`CODEX_AUTH_HOME`) plus the provider/login keys F00 recorded
at the tag (or that documented prefix set). Freeze a
`DecideV1` exec object whose `env` is that complete
post-overlay post-scrub block (set-equal to the environment
the spawned process will see; no omitted inherited or
internally added pair). Spawn decide with the G4 `Event`
envelope (`hook_event_name = "Exec"`, `cwd`, `command` as
display-only, `decide_v1` = the frozen object); closed stdin
after write; 1000 ms deadline (`DECIDE_DEADLINE`);
policy-runner process env scrubbed separately; no shell
interpolation. This scrub reinforces F12 set-equality; it is
not a new gate.
Anything except exact stdout `{"decision":"ALLOW"}` (no extra
keys) means do not execute.

Background/detached descendants: an explicitly ALLOWED exec
that spawns a detached or background child then exits must
leave zero surviving descendants at exec-tool settlement
(process tree after the tool result, not before the next
model turn). Acceptable: (1) policy rejects the
background/detached pattern, or (2) Codex process supervision
reaps the descendant before settlement. Do not rely on the
outer Job Object killing everything at session end. Do not
add a turn-loop harness. F00 has not pre-authorized a source
Produce (the scout has not shown supervision cannot reap). A
red settlement test fails this phase and returns to planning;
do not expand F12 Produces in-phase.

Deny any argv token that names the auth directory after
canonicalization, case-insensitive compare, and `\\?\`
normalization. Agent-originated calls missing `ToolCtx` deny.
Denial is `CovenantDenied`, never `SandboxDenied`.

Unit tests may stub the policy binary. Integration tests pin a
G4-green Windows `covenant-cli` Release asset in
`covenant/sidecar-fixture.toml` (URL + SHA-256 + attested G4
schema/semantics identifier naming complete post-overlay `env`,
`volume_serial` / `file_index` / `pre_image_digest` evaluation,
and fork `Read`-deny on `$CODEX_AUTH_HOME/auth.json`) and spawn
that exe as `hook decide --client codex`. Display-only
`command` ALLOW vs `decide_v1` DENY (and the reverse) must
follow `decide_v1`. Real-sidecar discriminating tests: a pair
of `decide_v1` exec objects differing only by one env pair
must produce different decisions against that exact sidecar
version.

**Required Pattern:**

```text
startup: LaunchContract = env once
         {COVENANT_DECIDER_PATH, COVENANT_DECIDER_SHA256,
          COVENANT_CHILD_MARKER}; never re-read
before decide spawn: open exe; hash opened handle;
                     hold no-replace lock through CreateProcess;
                     created image == hashed object == frozen SHA-256
missing/mismatch/swap-after-hash: CovenantDenied, zero effect

stdin:  G4 Event envelope
        { hook_event_name: "Exec", cwd, command?,
          decide_v1: DecideV1 { version:1, kind:"exec",
            exec:{ program, argv, cwd, env: complete, ... } } }
stdout: exactly {"decision":"ALLOW"}  -> spawn
else:   CovenantDenied, no backend.start, no spawn_process
deadline: 1000ms, then kill + deny
argv token names auth dir (canon, case-insensitive): deny
one immutable Windows env block -> decide_v1.exec.env AND spawn
  keys case-insensitive; reject collisions/invalid
  closed secret-key set: four control keys + F00 provider/login
  keys (or documented prefix set); mixed-case/collisions absent
background/detached: zero surviving descendant at tool-result
  settlement (policy reject or Codex reap; not Job Object;
  red -> re-plan; no in-phase Produce expansion)

sidecar-fixture.toml pins:
  url, sha256,
  g4_schema_id,   // F10 schema $id + decide-v1.json SHA-256
  g4_semantics_id // complete-env + volume_serial/file_index/
                  // pre_image_digest + auth_home_read_deny
```

**Skills Needed:** `rust-general`

**Phase Success Criteria:**

1. Integration against the pinned G4-green sidecar in
   `covenant/sidecar-fixture.toml` (URL + SHA-256 + attested
   G4 schema/semantics id): a frozen `decide_v1` exec object
   that G4 would ALLOW allows the process start to be
   attempted; a G4 DENY object does not; display-only `command`
   cannot override `decide_v1`. A pair of `decide_v1` objects
   that differ only by a single env pair whose key is chosen at
   test time (not fixed in the sidecar) produce different
   decisions against that exact sidecar version.
2. Table-driven at the exec call site for missing binary, spawn
   failure, timeout, crash, malformed JSON, empty stdout, extra
   JSON keys, non-zero exit, disconnect, cancellation, `DENY`,
   `ALLOW_WITH_CONTEXT`: zero command descendants and zero live
   policy-runner PID, handles, or tasks after settlement.
3. Cwd/PATH shadow of `covenant-cli`, `COVENANT_CLI` override,
   and file-swap of the policy executable after handle-hash and
   before process creation are all denials with zero command
   descendants.
4. Missing or mismatched launch contract (unset path, bad
   handle digest, missing marker) makes every gate deny with
   zero command descendants.
5. An argv token that names the auth directory
   (canonicalized, case-insensitive, `\\?\`) is denied with
   zero descendants.
6. A rebase test fails if
   `open_session_with_prepared_exec_env` is missing or the gate
   is no longer before both start branches.
7. One immutable Windows environment block is the single
   source for both `decide_v1.exec.env` and the spawned
   process. Keys are canonicalized case-insensitively; a
   case-colliding or invalid entry is denied before spawn.
   The closed secret-key set is `COVENANT_DECIDER_PATH`,
   `COVENANT_DECIDER_SHA256`, `COVENANT_CHILD_MARKER`,
   `CODEX_AUTH_HOME`, plus the provider/login keys F00
   recorded at the tag (or that documented prefix set). Each
   named key is absent from the real descendant env, including
   mixed-case and collision spellings (`cOdEx_AuTh_HoMe`,
   `Gh_ToKeN`, mixed-case `COVENANT_CHILD_MARKER`). `exec.env`
   is set-equal to that block. An inherited or internally
   added pair present in the spawned process but absent from
   the frozen map (or the reverse) is denied before spawn.
8. After an explicitly ALLOWED exec that spawned a detached or
   background child returns its tool result, the process tree
   contains zero surviving descendants of that exec (inspect
   after the tool result, not before the next model turn).
   Acceptable: policy rejects the background/detached pattern,
   or Codex process supervision reaps before settlement. The
   outer Job Object is not the proof. Do not add a turn-loop
   harness. No F12 Produce is pre-authorized for this
   invariant. A red result fails this phase and returns to
   planning; do not expand F12 Produces in-phase.

#### Phase F13 — Patch gate

**Rigor:** hard

**Depends On:** [F12]

**Scope:** Wire the F12 client at `ApplyPatchRuntime::run`. Do
not change process-start.

**Produces:**

- `fork:codex-rs/core/src/tools/runtimes/apply_patch.rs`

**Key Behaviors:** At the start of `ApplyPatchRuntime::run`,
before `apply_patch_with_options`, resolve every target through
existing ancestors, symlink/junction identities, and `\\?\`
normalization. Capture Windows file identity (`volume_serial`
plus `file_index`) and the content digest of each pre-image.
Freeze a `DecideV1` patch object that carries those resolved
identities (F10 `resolved_identities`) plus per-file
operation, structured hunks, and permissions. Call the same
gate with the G4 `Event` envelope (`hook_event_name = "Patch"`,
`path` / `cwd` display-only, `decide_v1` = the frozen object).
Same exact-ALLOW rule, deadline, launch-contract identity
(handle-hash, not path-hash), and `CovenantDenied` type as
F12. Denied means zero committed delta (no partial write).
Do not re-parse model text. Hold the captured identities
through mutation; same-path same-kind replacement (volume
serial or file index changed, or pre-image digest changed) is
denial. Unit tests may stub; integration tests use the F12
sidecar fixture. Real-sidecar discriminating tests: three
independent pairs of `decide_v1` patch objects — each differing
only by `volume_serial`, only by `file_index`, and only by
`pre_image_digest` — must each produce different decisions
against that exact sidecar version, so a partial G4 that
evaluates only one of the three cannot pass.

**Skills Needed:** `rust-general`

**Phase Success Criteria:**

1. Integration against the F12 sidecar fixture (same URL +
   SHA-256 + attested G4 schema/semantics id): a frozen
   `decide_v1` patch object that G4 would ALLOW reaches
   `apply_patch_with_options`; a G4 DENY object does not. Three
   independent pairs — each differing only by `volume_serial`,
   only by `file_index`, and only by `pre_image_digest` — each
   produce different decisions against that exact sidecar
   version; a partial G4 that evaluates only one of the three
   fails this criterion.
2. Table-driven at the patch call site for missing binary,
   spawn failure, timeout, crash, malformed JSON, empty stdout,
   extra JSON keys, non-zero exit, disconnect, cancellation,
   `DENY`, `ALLOW_WITH_CONTEXT`: zero committed byte delta and
   zero live policy-runner PID, handles, or tasks after
   settlement (add/update/delete/move).
3. Junction swap of an existing ancestor between freeze and
   write is denied or leaves zero committed delta outside the
   authorized identity.
4. Replacement of an existing ancestor directory between freeze
   and write is denied or leaves zero committed delta outside
   the authorized identity.
5. A reparse-point race on the target or an ancestor between
   freeze and write is denied or leaves zero committed delta
   outside the authorized identity.
6. A rebase test fails if `ApplyPatchRuntime::run` is missing or
   the gate is no longer before `apply_patch_with_options`.
7. Same-path same-kind replacement (path and kind unchanged;
   `volume_serial` or `file_index` differ, or pre-image digest
   differs) between freeze and write is denied with zero
   committed delta.

#### Phase F14 — Auth-path indirection

**Rigor:** hard

**Depends On:** [F00, F02]

**Scope:** Split login store from `CODEX_HOME` via
`CODEX_AUTH_HOME`. Codex owns credential reads, writes, and
refresh. Covenant never reads credential bytes. No gate
changes. No privilege boundary redesign. Do not hard-code a
disposable-`CODEX_HOME` diagram.

**Produces:**

- `fork:codex-rs/core/src/covenant_auth_home.rs`
- `fork:codex-rs/core/src/auth.rs`

**Key Behaviors:** Honor `CODEX_AUTH_HOME` (name may match an
upstream equivalent if F00 finds one) as the directory for
`auth.json` / login cache / refresh writes, regardless of
which `CODEX_HOME` lifetime the harness selects. `CODEX_HOME`
lifetime is decided by the harness adapter from a
redirectability probe (flagged CRN C3+C6): if `CODEX_HOME` can
be fully boxed, isolated, and disposed per-attempt with no
downside versus the current stable model, the harness runs
per-attempt isolation; otherwise it keeps stable `CODEX_HOME`
+ redirected sub-dirs. F14's contract is compatible with both.
If `CODEX_AUTH_HOME` is unset, preserve upstream
(`$CODEX_HOME/auth.json`). The new module is the env parser;
`auth.rs` is the F00-frozen load/refresh/write call site and
is switched to it. Covenant never reads or copies the file.
`CODEX_AUTH_HOME` is for Codex's own auth I/O; F12 scrubs it
from model-spawned exec subprocess environments.
Refresh takes a cross-process single-writer lock on the auth
file (or an atomic generation/CAS equivalent). Bare
reread-plus-replace is not the protocol. Concurrent runs must
not corrupt the stable store and must not commit a stale
generation over a newer authorized one.

Fork-controlled sinks must not retain a sentinel secret: logs,
errors, mutable `CODEX_HOME`, retained output. Covenant-retained
sinks are proven by `child-runtime-native` P6, not this phase.
Obfuscated shell reads of the auth path are the accepted
residual; P6 is the compensating control.

If F00 failed the auth path, do not implement — re-plan
Produces.

**Skills Needed:** `rust-general`

**Phase Success Criteria:**

1. With `CODEX_AUTH_HOME` and `CODEX_HOME` pointing at different
   empty dirs, `codex login` (or the F00-recorded auth probe)
   reads and writes `auth.json` only under `CODEX_AUTH_HOME`.
2. Refresh writes new credential bytes only under
   `CODEX_AUTH_HOME` and is atomic; refresh failure leaves the
   prior file intact.
3. Two refreshers forced past the same generation, and five
   concurrent refreshes against the same `CODEX_AUTH_HOME`
   with rotating single-use tokens, leave one well-formed
   `auth.json` whose generation is the newest authorized
   generation; a subsequent auth probe succeeds with that
   winner (no truncated or mixed writes, no stale commit
   under the single-writer lock, including failure and
   cancellation).
4. A sentinel secret used as the login token is absent from
   fork logs, errors, mutable `CODEX_HOME`, and retained
   output.
5. The same probe with only `CODEX_HOME` set still uses
   `$CODEX_HOME/auth.json` (upstream default).
6. A rebase test fails if the F00-recorded auth-load symbol is
   missing or the `covenant_auth_home` call is no longer before
   the first `auth.json` read or write.
7. After writing auth under `CODEX_AUTH_HOME`, deleting
   `CODEX_HOME` leaves a subsequent auth probe succeeding from
   `CODEX_AUTH_HOME` alone.

### Wave F2 — Packaging and inventory

Strip the surfaces that would blow the three-point budget.
Compile out ordinary hooks and the MCP client. Emit the C7
certificate inputs.

#### Phase F21 — Strip non-exec surfaces

**Rigor:** hard

**Depends On:** [F00, F02]

**Scope:** Achieve the capability-removal target with the
**smallest** upstream diff: a Covenant compile/build profile
or feature flags, immutable managed clamps, and F11 compiled
default-deny. Do not rewrite or remove large sections of the
upstream MCP/hook implementations unless tests prove they
remain reachable or perform startup side effects despite being
disabled. Clamp after every config layer. Do not add inventory
output yet.

**Produces:**

- `fork:codex-rs/Cargo.toml`
- `fork:codex-rs/features/src/lib.rs`
- `fork:codex-rs/cli/src/main.rs`
- `fork:codex-rs/core/src/config/mod.rs`
- `fork:codex-rs/core/src/tools/spec_plan.rs`
- `fork:covenant/model-catalog.json`

**Key Behaviors:** Default build is constrained `codex exec`
via a Covenant compile/build profile or feature flags.
Omit or feature-gate app-server and TUI entry points in
`cli/src/main.rs` so they are not in the published exe.
Disable ordinary command/MCP hook execution and the MCP
client the same way. Do not edit
`hooks/src/events/pre_tool_use.rs`,
`core/src/hook_runtime.rs`,
`rmcp-client/src/stdio_server_launcher.rs`, or
`codex-mcp/src/connection_manager.rs` unless SC5 proves they
remain reachable or perform startup side effects despite being
disabled; then re-plan this Produces list to add the failing
path. F00 still scouts those seams. Managed feature
requirements force `unified_exec = false` (one-shot
`exec_command`, no `write_stdin`), empty MCP
allowlists, `allowed_web_search_modes = ["disabled"]`, Code
Mode / collab / multi_agent_v2 / plugins / apps / image /
browser / computer-use / hooks false. Pin the model/provider
catalog (`covenant/model-catalog.json`) and load it fail-closed
so `multi_agent_version_for_model` cannot take a model-provided
override (DECISIONS.md §15). Startup fails closed if managed
requirements or the catalog cannot load.

Full exclusion list (unchanged): TUI, app-server, MCP
client/server-management, `write_stdin`/interactive terminals,
multi-agent/subagents, Code Mode, plugins/apps,
image/browser/computer-use, hosted web/provider-hosted tools,
dynamic/extension authorities, ordinary command/MCP hooks as
an enforcement/execution surface.

Clamp after every config layer: user/project `config.toml`,
env, profile, `-c`, CLI `--enable` / equivalent feature flags,
model-provided feature overrides, hosted tool specs, direct
Code Mode/plugin/app commands. A fixture provider (test-only)
records the outbound request and asserts no excluded tool spec
leaves the process. Keep the full hostile-override test
matrix. This is packaging plus immutable defaults plus F11
default-deny, not a fourth runtime gate.

**Skills Needed:** `rust-general`

**Phase Success Criteria:**

1. The F02 cargo invocation's exe has no app-server/TUI/MCP
   management command that starts a daemon or RPC listener
   (`codex app-server`, `codex mcp-server`, TUI default
   entry all fail or are absent).
2. With default managed features, `write_stdin` is not
   registered (spec_plan one-shot path); `--enable unified_exec`
   (or the F00-recorded equivalent) does not register it.
3. Hostile override matrix (user `config.toml`, env, profile,
   `-c`, CLI `--enable` / feature flags, model-provided feature
   override) does not register multi-agent, MCP, Code Mode,
   plugins, hosted web, apps, image/browser/computer-use, or
   `write_stdin`, and does not change the pinned catalog.
4. A fixture-provider capture of an exec request contains no
   hosted or excluded tool specification.
5. A fixture hook config that would spawn a marker process, and
   a fixture MCP server that would receive a request, produce
   no process and no request from the F02 exe.

#### Phase F22 — Inventory command (C7 contract)

**Rigor:** hard

**Depends On:** [F11, F21]

**Scope:** `--covenant-inventory` prints the C7 certificate
contract. Runtime command is the certificate input. No
build-emitted blob. No new gates.

**Produces:**

- `fork:codex-rs/cli/src/covenant_inventory.rs`
- `fork:codex-rs/cli/src/main.rs`

**Key Behaviors:** `codex --covenant-inventory` (wired from
`cli/src/main.rs`) prints one JSON document whose field names
and digest inputs match C7 Key Behaviors plus S2's
`PinCertificate` Required Pattern (C7 has no Required Pattern
block). Refuse to run `codex exec` if the effective registry
contains an identity not in the F11 const table.

C7 digest inputs: binary digest of this exe, profile digest
(fork emits residual/empty; C7 fills the C2 overlay + AGENTS
+ skills when `source = fork`), inventory id, optional schema
residual. Evidence payload: outbound tool schema, loaded
instruction/skill sources, effective feature values.

**Required Pattern:**

```text
codex --covenant-inventory -> PinCertificate JSON:
  binary_digest, profile_digest, inventory_id,
  schema_residual: Option, evidence:
    { tool_schema, instruction_sources, skill_sources, features }
unknown registered identity -> non-zero inventory status,
  exec refuses to start
```

**Skills Needed:** `rust-general`

**Phase Success Criteria:**

1. `--covenant-inventory` prints JSON with `binary_digest`,
   `profile_digest`, `inventory_id`, `schema_residual`, and
   `evidence.{tool_schema,instruction_sources,skill_sources,features}`.
2. `evidence.features` equals the F21 effective feature values;
   `evidence.tool_schema` identities equal the F11 const table
   actually registered.
3. Injecting a fake registered identity makes inventory fail
   and `codex exec` refuse to start.

### Wave F3 — Re-audit, promotion, and harness pin

#### Phase F31 — Re-audit CI

**Rigor:** hard

**Depends On:** [F00, F11, F12, F13, F14, F22, F40]

**Scope:** CI job that diffs the four patch sites and the
inventory against upstream, plus a golden sink-instrumentation
harness. Fail on drift. Do not mutate `COVENANT_PATCHES.md`
(F33 commits hunk SHAs after this job is green on an immutable
commit). No harness code files. No promotable Release.

**Produces:**

- `fork:.github/workflows/covenant-reaudit.yml`
- `fork:covenant/tests/sink_instrumentation.rs`

**Key Behaviors:** On each rebase PR and on a scheduled/manual
run: fetch the UPSTREAM.toml commit; diff the four Covenant
insertions (F11 registry admission, F12 process-start, F13
patch runtime, F14 auth-load) against upstream; fail if a
gate function moved without the Covenant call, or if a gate
symbol named in `COVENANT_PATCHES.md` is absent from the tree.
Diff `--covenant-inventory` tools/features against the F11
const table. Re-run F21 packaging checks (no app-server / TUI
/ MCP management command; no `write_stdin`; hostile overrides
still clamped).

Semantic completeness is proven by golden sink instrumentation
in `covenant/tests/sink_instrumentation.rs` (this phase owns
that file; F11 does not). The harness consumes F00's
`AUTHORITY-INVENTORY.toml` rather than rediscovering
constructors or sink families, and fails if that inventory's
recorded commit does not equal the `UPSTREAM.toml` commit so a
rebase cannot reuse a stale inventory. Inventory *completeness*
at a new tag is a human audit (F00), not a machine oracle; the
accepted residual is an upstream authority family omitted from
the inventory, compensated by F11 compiled default-deny, this
commit-bound re-audit, and human rebase review (RESIDUALS H4 /
H7). It instruments process,
filesystem, network, hosted, MCP, hook, generated-code, and
no-op sinks over every inventoried model and non-model
constructor (including aliases, wrappers, generated code,
callers outside `spec_plan.rs`, startup/direct-MCP, and
hosted-provider paths) and seeds one negative mutation per
inventoried sink family. The reaudit job builds a fixture
tree that inserts an intentionally ungated process-start (or
filesystem-write) beside the F12/F13 sites; that fixture
**must fail CI**. The same harness fails on renamed gates,
registrations outside `spec_plan.rs`, same-name effect-class
changes detected by sinks rather than table labels, new
direct process/filesystem/network/MCP/hosted/dynamic callers,
and moved/aliased feature flags. Does not auto-merge. Does
not write hunk SHAs.

**Skills Needed:** `rust-general`

**Phase Success Criteria:**

1. Deleting any of the four Covenant insertions (F11, F12, F13,
   F14), or removing a gate symbol named in
   `COVENANT_PATCHES.md`, fails the reaudit job.
2. A synthetic extra registered tool identity fails the
   reaudit job.
3. An intentionally ungated process-start or filesystem-write
   path in the golden fixture tree fails the reaudit job.
4. An admitted identity whose instrumented sinks in
   `covenant/tests/sink_instrumentation.rs` change class
   without a matching admission-table change fails the
   reaudit job.
5. A rebase that restores `codex app-server`, TUI default
   entry, MCP management, or `write_stdin` on the F02 exe
   fails the reaudit job.
6. A no-op rerun against the current (patched) tree is green.
7. A new process, filesystem, network, MCP, or hosted caller
   introduced via alias or wrapper outside `spec_plan.rs`
   fails the reaudit job (`sink_instrumentation.rs`).
8. The reaudit job consumes F00's `AUTHORITY-INVENTORY.toml`
   (not a rediscovered set) and seeds one negative mutation
   per inventoried sink family, including startup/direct-MCP
   and hosted-provider paths; each mutation fails the job. The
   job also fails if the inventory's recorded commit does not
   equal the `UPSTREAM.toml` commit.

#### Phase F33 — Post-audit promotion

**Rigor:** hard

**Depends On:** [F03, F31, F22]

**Scope:** After F31 is green, commit hunk SHAs, re-run F31 on
that immutable commit, build with the F02 toolchain, and
publish the GitHub Release. F03's `workflow_dispatch` path
stays non-publishing.

**Produces:**

- `fork:.github/workflows/covenant-release.yml`
- `fork:COVENANT_PATCHES.md`

**Key Behaviors:** Enable tag-push (or an explicit promote job)
on the F03 workflow. Sequence: write F11–F14 hunk SHAs into
`COVENANT_PATCHES.md`; commit; re-run the F31 reaudit job on
that immutable commit; on green, build with the F02 pinned
toolchain on `windows-2022`; hash the built exe; run
`codex --covenant-inventory` against that exe; assemble
`provenance.json`; generate a GitHub build-provenance
attestation (`actions/attest-build-provenance`, pinned to a
40-hex SHA like every other `uses:` per F03 SC2) binding the
exe digest to this immutable workflow run, source commit, and
tag; publish the Release. The fork does not code-sign; the
attestation (not a certificate) is the trust anchor that binds
the promoted bytes to the F31-green build, closing the mutable-
Release-metadata gap. Publish a GitHub Release whose assets
are:
`codex-x86_64-pc-windows-msvc.exe`, `codex.exe.sha256`,
`covenant-inventory.json`, `provenance.json` (source commit,
toolchain, runner, exe digest, inventory digest, re-audit
run id — no signer field), and `COVENANT_PATCHES.md`. No
Actions-artifact URL is a Release URL. F32 consumes exactly
this Release's digest. Promotion refuses unless F12 SC1 and
F13 SC1 discriminating tests are green against the exact
sidecar version pinned in `covenant/sidecar-fixture.toml`
(URL + SHA-256 + attested G4 schema/semantics id).

**Skills Needed:** `rust-general`

**Phase Success Criteria:**

1. A tag-push (or promote job) of an F31-green commit creates a
   GitHub Release under `covenant-capital/codex` with the five
   assets named above, all taken from the built exe.
2. `codex.exe.sha256` is one 64-hex line matching the built
   exe; `covenant-inventory.json` `binary_digest` equals that
   same digest; `provenance.json` records source commit,
   toolchain, runner, exe digest, inventory digest, and
   re-audit run id, with no signer field.
3. F31 reaudit is green on the tagged commit (the tree that
   contains the hunk SHAs).
4. `workflow_dispatch` on the same YAML still uploads artifacts
   only and does not create a Release.
5. Promotion refuses unless F12 SC1 and F13 SC1 discriminating
   tests are green against the exact sidecar version (URL,
   SHA-256, and attested G4 schema/semantics id) in
   `covenant/sidecar-fixture.toml`.
6. The promote job generates a GitHub build-provenance
   attestation (`actions/attest-build-provenance`, pinned to a
   40-hex SHA per F03 SC2) binding the published exe digest to
   the workflow run, source commit, and tag; the Release does
   not publish if attestation generation fails.

#### Phase F32 — Harness CODEX_PIN row

**Rigor:** hard

**Depends On:** [F33]

**Scope:** The only harness-side code write. Fill S2's
`CODEX_PIN` fork arm from the F33 Release. Do not change
adapters, doctor, seeds, or FE. Do not add `CODEX_FORK_PIN`.
Do not widen the pin past S2's four fields.

**Produces:**

- `backend/covenant-utils/src/pinned_versions.rs`

**Key Behaviors:** One `CODEX_PIN` the S2 schema can store:
`source = fork`, version matching the fork tag, URL of the F33
GitHub Release asset `codex-x86_64-pc-windows-msvc.exe` (no
Actions artifact URL), SHA-256 equal to that Release's
`codex.exe.sha256` (the built exe's hash). Source commit and
inventory digest live on the Release `provenance.json`, not as
extra constants unless S2 is revised. If S2 has no `source` field,
add the discriminant with values `official | fork` and set
this row to `fork`. If S2 has not landed, stop and re-plan
this Produces list rather than inventing a parallel constant.
`covenant-utils` still has zero `covenant-*` deps. Serialize
with S2 on this file. Before writing the pin, verify the F33
GitHub build-provenance attestation for the Release exe
(`gh attestation verify` bound to the tag, source commit, and
workflow); a missing or failed attestation, or a digest not
bound to the F31-green build, refuses the pin. Adoption also
refuses unless F12 SC1 and F13 SC1 discriminating tests are
green against the exact sidecar version in
`sidecar-fixture.toml`, CRN C3+C6 amendments have landed with
`CODEX_AUTH_HOME ≠ CODEX_HOME` when the probe permits, and a
real-adapter separation test proves auth I/O stays under the
stable auth root while probe-classified mutable paths follow
the selected home model.

**Required Pattern:**

```text
CODEX_PIN {
  source: fork,
  version: "<fork tag without v>",
  url: "<GitHub Release asset URL for
        codex-x86_64-pc-windows-msvc.exe>",
  sha256: "<64 hex from that Release's codex.exe.sha256>"
}
```

**Skills Needed:** `rust-general`

**Phase Success Criteria:**

1. `CODEX_PIN.source` equals `fork` and its URL names
   `covenant-capital/codex` plus asset
   `codex-x86_64-pc-windows-msvc.exe` (the F33 Release URL, not
   an Actions artifact).
2. `CODEX_PIN.sha256` is 64 hex and matches the F33 Release
   `codex.exe.sha256` for that version.
3. No other harness path besides `pinned_versions.rs` changes
   in this phase.
4. Adoption refuses unless F12 SC1 and F13 SC1 discriminating
   tests are green against the exact sidecar version (URL,
   SHA-256, attested G4 schema/semantics id) in
   `covenant/sidecar-fixture.toml`.
5. Adoption refuses unless CRN C3+C6 amendments have landed
   with `CODEX_AUTH_HOME ≠ CODEX_HOME` when the probe permits,
   and a real-adapter separation test proves auth I/O stays
   under the stable auth root while probe-classified mutable
   paths follow the selected home model.
6. The pin is written only after `gh attestation verify`
   succeeds for the Release exe, bound to the tag, source
   commit, and workflow run; a missing/failed attestation or a
   digest not bound to the F31-green build refuses the pin.

### Wave F4 — Repository context

Land the eight harness `repo-context/` files at their fork-repo
paths so agents inside `covenant-capital/codex` can work without
this harness repo.

#### Phase F40 — Fork context pack

**Rigor:** medium

**Depends On:** [F01]

**Scope:** Copy the eight harness context files into the fork
and add a document-map pointer on upstream `README.md`. Do not
patch gates. Those harness copies are already the corrected
canonical contract; F40 lands them byte-identically. Do not
copy a stale file and override later.

**Produces:**

- `fork:CLAUDE.md`
- `fork:AGENTS.md`
- `fork:COVENANT_PATCHES.md`
- `fork:docs/DECIDE_V1.md`
- `fork:docs/RELEASE.md`
- `fork:docs/RESIDUALS.md`
- `fork:CONTRIBUTING.md`
- `fork:README-COVENANT.md`
- `fork:README.md`

**Key Behaviors:** Byte-copy from
`docs/master-plans/cross/codex-fork/repo-context/` as follows:

| Harness copy | Fork path |
|---|---|
| `repo-context/CLAUDE.md` | `CLAUDE.md` |
| `repo-context/AGENTS.md` | `AGENTS.md` |
| `repo-context/COVENANT_PATCHES.md` | `COVENANT_PATCHES.md` |
| `repo-context/docs/DECIDE_V1.md` | `docs/DECIDE_V1.md` |
| `repo-context/docs/RELEASE.md` | `docs/RELEASE.md` |
| `repo-context/docs/RESIDUALS.md` | `docs/RESIDUALS.md` |
| `repo-context/CONTRIBUTING.md` | `CONTRIBUTING.md` |
| `repo-context/README-COVENANT.md` | `README-COVENANT.md` |

`CLAUDE.md` / `AGENTS.md`: what the fork is, the four patches,
the never-touch list, fork-first Adoption sequence,
branch/release topology, opaque marker plus follow-up pointer,
how to rebase. `README-COVENANT.md` and `CLAUDE.md` carry the
same Adoption sequence section.
`COVENANT_PATCHES.md`: patch index with F11–F14 plus F21
(constrained packaging/profile) and F22 (inventory/certificate)
so the index is the complete answer to what Covenant modifies.
`docs/DECIDE_V1.md`: wire schema with complete exec `env` and
patch `resolved_identities`, examples, failure semantics.
`docs/RELEASE.md`: toolchain pins, provenance, four-field
`CODEX_PIN`, how the harness pin consumes a Release.
`docs/RESIDUALS.md`: accepted residuals and harness
compensating controls.
`CONTRIBUTING.md`: upstream-first; branch topology; info-only
daily upstream-tracking routine. `README.md` keeps upstream
content and gains a document-map block linking the eight files.
F31 treats absence of a gate symbol named in
`COVENANT_PATCHES.md` as failure.

**Skills Needed:** `md-authoring`

**Phase Success Criteria:**

1. Each of the eight harness `repo-context/` files is
   byte-identical to its fork path. F33 may later rewrite
   `COVENANT_PATCHES.md` hunk SHAs; the other seven stay
   identical through the F33 Release.
2. Each of the eight files exists and is referenced from
   `fork:README.md`.
3. `COVENANT_PATCHES.md` names F11–F14 and the three audit
   call sites (`registry.rs` dispatch,
   `process_manager.rs` open, `apply_patch.rs` run) plus
   `auth.rs`, and index rows for F21 (constrained
   packaging/profile) and F22 (inventory/certificate).

## Success Criteria

1. Fork repo `covenant-capital/codex` is rooted at the
   F01-recorded latest stable `rust-v*` tag and commit; F00
   re-anchors the four seams and sink families at that tag
   and produces the commit-bound authority inventory (F01,
   F00).
2. F00's integration map matches later-phase Produces (F00).
3. CI builds Windows `codex.exe` plus a 64-hex digest without
   publishing a promotable Release (F03).
4. Decide wire schema v1 exists in the fork and in this
   workspace, including complete exec `env` and patch
   `resolved_identities` with volume serial, file index, and
   pre-image digest (F10).
5. Unknown tool identity is a terminal `CovenantDenied`; the
   admitted const table is exactly `exec_command` + custom
   `apply_patch` as named in this plan; sink-clean `read`/`none`
   is not auto-admitted; F00-named writers are denied before
   the handler; a fixture-provider repository-read turn
   completes using only those two tools; `pre_tool_use` is
   never invoked (F11).
6. Process start occurs only after exact decide ALLOW on the
   frozen `decide_v1` exec object (one immutable Windows env
   block, case-insensitive keys, closed secret-key set absent
   including mixed-case/collisions, set-equal) via the pinned
   G4 sidecar (digest + attested schema/semantics id) and a
   handle-hashed launch contract; discriminating env-pair
   objects change the sidecar decision; every other outcome
   is zero command descendants and zero live policy-runner;
   an ALLOWED background/detached exec leaves zero surviving
   descendant at tool-result settlement (F12).
7. `apply_patch` writes only after exact decide ALLOW on the
   frozen `decide_v1` patch object with resolved identities
   including file id and pre-image digest; deny, junction
   swap, ancestor replacement, same-path same-kind
   replacement, and reparse race leave zero committed delta
   (F13).
8. `CODEX_AUTH_HOME` separates login from `CODEX_HOME` under
   either harness home model; Codex owns read/write/refresh;
   the single-writer lock leaves the newest authorized
   generation; auth probe still succeeds after `CODEX_HOME`
   deletion (F14).
9. Published exe has no app-server/TUI/MCP/multi-agent/Code
   Mode/hosted-web/hooks surface; a fixture hook and fixture
   MCP server produce no process and no request; inventory
   matches the C7 certificate contract (F21, F22).
10. Re-audit CI fails on patch-site deletion, inventory drift,
      F21 packaging restore, an ungated golden-fixture sink, an
      aliased sink outside `spec_plan.rs`, or a negative
      mutation of an F00-inventoried sink family (F31).
11. F33 publishes a GitHub Release whose SHA-256 the
    harness pins and whose exe carries a build-provenance
    attestation, only after F12/F13 discriminating sidecar
    tests are green against the pinned sidecar version;
    F32 `CODEX_PIN.source = fork` points at that digest,
    verifies the attestation before pinning, and refuses
    until those tests plus landed C3+C6 distinct-root
    separation are green (F33, F32).
12. The eight context files are byte-identical to the harness
    copies and referenced from `fork:README.md` (F40).

## Constraints / Invariants

- Windows-only. No `#[cfg(unix)]`, no extra targets.
- Do not rewrite the agent/turn/reasoning loop.
- Do not reuse stock `PreToolUse` as enforcement. Ordinary
  command/MCP hooks and the MCP client are compiled out of the
  promoted build.
- Exact ALLOW only; `ALLOW_WITH_CONTEXT` is deny.
- `CovenantDenied` is non-retriable and is never mapped to
  sandbox-retry.
- Policy runner is not launched through the model process path.
- Launch contract env (`COVENANT_DECIDER_PATH`,
  `COVENANT_DECIDER_SHA256`, `COVENANT_CHILD_MARKER`) is read
  once at startup, verified before every decide call by hashing
  the opened executable handle (not a path), and never
  re-read. No PATH lookup. No `COVENANT_CLI`. Model-visible
  tools cannot change it. Missing or mismatched contract denies
  every decide call with zero effect.
- Allowlist is a compiled const table. Effect class is the
  class observed by `covenant/tests/sink_instrumentation.rs`,
  not a self-declared label. No runtime extension path.
- Decide `exec.env` is one immutable Windows environment
  block, the single source for both the frozen object and the
  spawned process, set-equal. Keys are canonicalized
  case-insensitively; colliding or invalid entries deny.
  Closed secret-key set: `COVENANT_DECIDER_PATH`,
  `COVENANT_DECIDER_SHA256`, `COVENANT_CHILD_MARKER`,
  `CODEX_AUTH_HOME`, plus the provider/login keys F00 records
  at the tag (or that documented prefix set). Mixed-case and
  collision spellings of those keys are absent from the
  descendant env.
- `COVENANT_CHILD_MARKER` is opaque and presence-checked this
  cycle. Nonce / WorkerContract binding is a documented
  follow-up, not F10/F12/F13 work. Env-scrub is what makes
  deferring it safe.
- The F11 admitted table is closed. A new upstream identity,
  including a sink-clean read tool, is DENY until an explicit
  plan amendment names it in that table.
- `CODEX_HOME` lifetime is harness-decided (C3/C6 probe). F14
  honors `CODEX_AUTH_HOME` under either model.
- Branch/release topology: upstream `openai/codex` -> fork
  `main` = clean adopted upstream stable tag with no Covenant
  behavioral patches; `covenant` branch = `main` +
  F11/F12/F13/F14 + F21/F22. Never merge `covenant` into
  `main`. Releases (F03 CI, F33 promotion) build only from the
  audited `covenant` branch/tag. Adopting a new upstream tag:
  update clean `main`; merge/rebase `main` into `covenant`;
  resolve conflicts; rerun F31; rebuild the Windows artifact;
  test; publish only when green.
- Patch identities include `volume_serial`, `file_index`, and
  pre-image content digest, and detect same-path replacement.
- Promotion requires a green F31 re-audit, the SHA-256 pin,
  `provenance.json`, and a GitHub build-provenance attestation
  (verified before the pin); the fork does not code-sign the
  binary (single-user local tool).
- User/project config cannot replace the allowlist, the decide
  binary, or re-enable stripped features.
- Direct file tools `deny_read` the auth directory. F12 denies
  argv tokens that name that path (canonicalized,
  case-insensitive). Obfuscated shell reads of the auth store
  are the accepted residual; `child-runtime-native` P6 secret
  canary is the compensating control. No privilege-boundary
  redesign.
- Codex owns credential reads, writes, and refresh. Covenant
  never reads credential bytes. F14 must not log or copy
  `auth.json` bytes.
- `covenant-utils` remains free of `covenant-*` dependencies.
- Patch surface stays the four F1 hunks plus packaging/inventory
  machinery and compiled-out hook/MCP seams. New executors
  default-deny.
- `Produces` under `fork:` are written in
  `covenant-capital/codex`. This-repo writes are F10's schema
  copy and F32's `CODEX_PIN` row.
- Owner of rebases is the `covenant-capital/codex` maintainer.
  Cadence default is explicit user intent. Each `rust-v*` rebase
  re-applies F11–F14 hunks plus F21 defaults and F22 inventory.
  Expected breaks: renamed call sites, new tool identities,
  feature-flag churn. F31 is the mechanical check. F1
  zero-effect tests and F21 SC1–5 re-run on rebase.
- Fork criteria are this repo's CI jobs and tests. Landing is
  the F33 GitHub Release.
- F10 `decide_v1` is the shared schema; G4 consumes it; version
  bumps require both plans.
- Refresh of `auth.json` uses a cross-process single-writer
  lock. Last-writer-wins with a re-read is not the protocol.

## Open Questions

1. **Upstream cadence.** Manual today; the daily GitHub Action
   + Codex Cloud routine documented in `CONTRIBUTING.md` is
   the intended maintainer-owned automation. It is
   informational — not implemented by this plan. Default until
   that automation is set up: pin each audited `rust-v*` tag;
   rebase only on explicit user intent; F31 is the gate.
2. **F14 upstream.** If OpenAI ships auth-home indirection, drop
   the F14 patch and keep the env name they chose. Default: keep
   `CODEX_AUTH_HOME` until that happens.
3. **Public-fork license/attribution.** Confirm SPDX and NOTICE
   requirements against the tag tree at F01. Default: keep
   upstream LICENSE bytes; Covenant attribution only in
    `covenant/UPSTREAM.toml` and Wave F4 files; repo is public;
    harness pull path is token-free.
4. **Follow-up: marker-as-capability.** Not this cycle. Keep
   the opaque, presence-checked `COVENANT_CHILD_MARKER`. Future
   upgrade: make the marker a high-entropy per-dispatch
   capability the decider resolves to an immutable
   launcher-owned WorkerContract (`run_id`, `child_id`, role,
   worktree, workspace, allowed Produces, protected artifacts,
   policy scope); every Exec/Patch decision bound to it;
   unknown/expired/reused/mismatched marker => DENY. Cross-plan:
   CRN C3 mints and stores, G4 looks up, fork envelope carries
   the marker. Env-scrub (F12) is what makes deferring this
   safe — the marker cannot leak into spawned subprocesses in
   the interim.
