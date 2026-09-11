# Review: codex-fork

**Date:** 2026-09-05
**Rigor:** hard
**Phase:** plan
**Reviewers:** alignment, adversarial, general (synthesis)

## Verdict

PASS (final, after two re-plans + inline fixes)

**Resolution trail:** cycle-1 synthesis was FAIL (below). Cycle-2 applied
S1–S5/T1–T5. Cycle-3 resolved the signer question by user decision (no code
signing; SHA-256-pinned promotion). A re-review then re-raised the four
adversarial blockers as public-supply-chain concerns; the orchestrator judged
three over-engineered for a single-user local tool and took the proportionate
subset inline: expanded F12/F13 discriminating tests (each identity field
independently), F31 commit-bound inventory + accepted human-audit residual
(RESIDUALS H4), F32 hard cross-plan gate on CRN C3+C6, and — instead of
Authenticode — a **free GitHub build-provenance attestation** verified before
pinning (closes the mutable-Release-metadata gap without a certificate). A final
single alignment lens returned PASS; its three targeted/trivial findings
(F00 commit field, attest-action SHA pin, integrity restatements) were fixed
inline. Plan lint clean. The original cycle-1 FAIL synthesis is retained below
for history.

---

## Cycle-1 synthesis (historical): FAIL

Alignment PASS and general PASS; adversarial FAIL with four blocker-tagged
findings. Per the artifact rule, an adversarial blocker yields FAIL. The
amendment landed every locked decision and accepted rec (alignment Q1 map);
the failures are missing verification gates and under-specified criteria, not a
broken architecture. F11 admission-only, F12/F13 final boundaries, exact-ALLOW,
non-retriable `CovenantDenied`, no agent-loop rewrite, Windows-only, F31
mandatory, F33 sole release path all hold. Two of the four blockers were
independently confirmed by the orchestrator against the live tree
(`child-runtime-native` C3 sets `CODEX_HOME` and `CODEX_AUTH_HOME` to the same
S5 `config_dir`; live G4 still specifies `env_allowlist` and omits the Windows
patch-identity fields).

Routing: all code/plan findings return to the planner for one re-plan pass
except OQ4 (Authenticode signer), which is a standing human decision, not a plan
defect — the plan is correctly fail-closed on it.

## Findings

### S1 — Stale-G4 sidecar can pass fork gates while ignoring the amended fields

**Severity:** structural (adversarial-tagged blocker)
**Reviewers:** adversarial (alignment, general concur on the G4 gap)
**Type:** fakery
**Location:** `master.md:179-185,736-741,773-777`; live G4
`child-runtime-native/master.md:1924-1984`
**Issue:** The G4 amendment is a prose precondition. `sidecar-fixture.toml`
pins only URL + exe digest, not the `decide_v1` schema/semantics the exe
implements. F12/F13 test one ALLOW + one DENY object, never a discriminating
pair differing only by an env pair, `volume_serial`, `file_index`, or
`pre_image_digest`. A sidecar built from today's `env_allowlist` G4 satisfies
F12 SC1 / F13 SC1 and lets F33 publish while ignoring the very fields that make
the gates effective.
**Suggested fix:** Pin an attested G4 schema/semantics id alongside the fixture
digest; add real-sidecar discriminating tests that flip one authoritative field
and prove the decision changes; make F33/F32 refuse promotion/adoption until
those tests pass against the exact sidecar version.
**Routing:** planner (codex-fork tests + gates); the CRN G4 amendment stays a
flagged cross-plan precondition.

### S2 — F14 can pass while the integrated adapter co-locates login and mutable state

**Severity:** structural (adversarial-tagged blocker)
**Reviewers:** adversarial + general
**Type:** correctness
**Location:** `master.md:183-185,198-221,877-950`; live C3
`child-runtime-native/master.md:1365,1371,1414-1418` (both homes = S5
`config_dir`)
**Issue:** Live C3 sets `CODEX_HOME` and `CODEX_AUTH_HOME` to the same S5
`config_dir`; C6 probes only that model. F14 verifies the fork with two
caller-supplied dirs, but F32 fork adoption is not gated on the C3/C6
distinct-root amendment. Every F14 criterion can be green while the real child
gets one root for login and mutable state — the split is a second name for the
same directory.
**Suggested fix:** Strengthen the Section C C3+C6 bullet to require
`CODEX_AUTH_HOME ≠ CODEX_HOME` when the probe allows; gate F32 adoption on
landed C3+C6 plus a real-adapter separation test that proves auth I/O stays
under the stable auth root while probe-classified mutable paths follow the
selected home model.
**Routing:** planner (strengthen flag + F32 gate); CRN impl stays flagged.

### S3 — Env-scrub is case-sensitive and its secret set is open

**Severity:** structural (adversarial-tagged blocker + general targeted)
**Reviewers:** adversarial + general
**Type:** edge-case
**Location:** `master.md:700-718,796-804`; `repo-context/docs/DECIDE_V1.md:47-67`
**Issue:** Windows env keys are case-insensitive. The scrub names uppercase
literals only, so `cOdEx_AuTh_HoMe` / `Gh_ToKeN` / mixed-case
`COVENANT_CHILD_MARKER` survive and `GetEnvironmentVariableW` in the descendant
still resolves them; a case-sensitive JSON-map equality also diverges from the
effective Windows env block. "Unrelated provider/login secrets" is not a closed
list, so a test cannot fail on an unnamed key (`OPENAI_API_KEY` etc.).
**Suggested fix:** Define one immutable Windows env block as the source for both
decision and spawn; canonicalize keys case-insensitively; reject case-colliding
entries; freeze a closed secret-key set (the four control keys + the
provider/login keys F00 records at the tag, or a documented prefix set); add
mixed-case/collision tests inspecting the real descendant env.
**Routing:** planner.

### S4 — Latest-tag re-anchor has no executable completeness artifact

**Severity:** structural
**Reviewers:** adversarial
**Type:** fakery
**Location:** `master.md:334-408,1097-1162`
**Issue:** F00 "re-confirms" four seams + five sink families but produces only a
path-map TOML — no scan or authority manifest proving completeness at a newly
selected tag. A newer tag can add a non-model startup constructor, a hosted
authority, or a network/MCP sink behind a new wrapper family; F00's
known-vocabulary check and F31's single known-fixture mutation both stay green
while the new family is never instrumented. Material because F11 does not
dominate hook/MCP startup or direct callers.
**Suggested fix:** F00 produces a commit-bound authority inventory + executable
audit procedure enumerating all model and non-model constructors plus process,
filesystem, network, hosted, MCP, hook, and generated-code sinks with source
anchors and reviewer disposition; F31 consumes that inventory; seed one negative
mutation per family (including startup/direct-MCP and hosted-provider paths).
**Routing:** planner.

### S5 — Two-tool build has no end-to-end task-viability gate

**Severity:** targeted (viability of the F11 closed-table decision)
**Reviewers:** adversarial
**Type:** correctness
**Location:** `master.md:570-668,1023-1041`
**Issue:** No criterion runs the constrained build through a complete
repository-read turn using only `exec_command` + custom `apply_patch`. Denying
every native read/view identity may be viable via shell reads, but the plan
never proves the tool plan + agent loop make that fallback usable; a build that
reaches `exec_command` in a unit test but cannot complete a read task can pass
every listed criterion.
**Suggested fix:** Before freezing the two-row F11 table, add a deterministic
fixture-provider acceptance test against the F02 exe that reads a known file
through the admitted one-shot exec path, completes the turn, and proves no
denied native read identity or excluded surface was needed.
**Routing:** planner.

### T1 — F12 SC8 cannot gate in one implementer pass

**Severity:** targeted
**Reviewers:** general
**Type:** correctness
**Location:** `master.md:720-729,805-814`
**Issue:** SC8 hides a second phase ("expand F12 Produces with the smallest
source patch") an implementer cannot legally do, and "before Codex takes its
next model/tool turn" needs a turn-loop harness the Out-list forbids. A red SC8
becomes scope drift or a weakened assertion.
**Suggested fix:** Rewrite SC8 to F00's fail-and-replan pattern; observable
proof = zero surviving descendants at exec-tool settlement (process tree after
the tool result, not the next model turn); pre-authorize a named Produce only if
the scout already shows supervision cannot reap.
**Routing:** planner.

### T2 — F11 SC6 has a circular dependency on F31's sink file

**Severity:** targeted
**Reviewers:** general
**Type:** correctness
**Location:** `master.md:601-605,662-665`; F31 Produce `:1112`
**Issue:** F11 SC6 names F31's `covenant/tests/sink_instrumentation.rs` as the
classifier, but F31 Depends On F11 (and both list the file as a Produce). SC6
cannot be proven in F11 without a circular dependency / shared-file ambiguity.
**Suggested fix:** Resolve ownership of `sink_instrumentation.rs` to exactly one
phase; gate F11 on dispatch of named upstream writer identities from the F00
list (MCP, hosted, web, `write_stdin`, etc.) plus any F00 `read`/`none`
identity, with no dependence on F31's harness; keep sink-class agreement on F31.
**Routing:** planner.

### T3 — Patch schema accepts an existing-file op with no pre-image digest

**Severity:** targeted
**Reviewers:** adversarial + general + alignment
**Type:** correctness
**Location:** `schema/decide-v1.json:92-120,163-208`;
`repo-context/docs/DECIDE_V1.md:84-95`; F10 SC3 `master.md:564-568`
**Issue:** `pre_image_digest` (and symlink/junction `target`, ancestor
`win32_normalized`) are optional in the JSON schema, so a schema-valid
update/delete/move can omit the content identity F13/G4 need; F10 SC3 also lists
`pre_image_digest` as if always present (false for `add` / directory kind).
**Suggested fix:** Add schema `if`/`then` requires matching DECIDE_V1 (require
`pre_image_digest` for update/delete/move source and `kind == file`; require
`target` for symlink/junction); keep it optional on `add` / non-file kinds;
align F10 SC3; keep the workspace copy byte-identical to
`fork:covenant/schema/decide-v1.json`.
**Routing:** planner.

### T4 — Section C under-specifies CRN contracts the fork depends on

**Severity:** targeted
**Reviewers:** general + alignment
**Type:** alignment
**Location:** `master.md:104-119,185,198-221`; live C7
`child-runtime-native/master.md:1286-1345`; C2 / residual matrix
**Issue:** (a) Why + the C7 dependency line claim C7 invokes
`codex --covenant-inventory` when `source = fork`, but Section C only says
"verify fields match F22" and live C7 captures `codex exec --json` via a
fixture, never `--covenant-inventory`. (b) G4 `Read`-deny is flagged onto
`$CODEX_AUTH_HOME/auth.json` but C2's direct-file `deny_read` and P6's canary
still target `$CODEX_HOME/auth.json`. (c) CRN's residual matrix claims F14
removes F1.4 while this plan keeps shell-mediated auth reads accepted.
**Suggested fix:** Strengthen the Section C bullets (do not edit CRN): C7 must
invoke `codex --covenant-inventory` when `source = fork`; C2 `deny_read` and P6
canary follow `$CODEX_AUTH_HOME/auth.json`; C3+C6 keep `CODEX_AUTH_HOME ≠
CODEX_HOME`; F1.4 stays accepted on the fork arm. Retarget the Why G4 citation
from the stale `replan-d-g4-schema.md` to the Cross-plan G4 bullet.
**Routing:** planner.

### T5 — Leftover disposable-home / signer-hedge drift

**Severity:** trivial→targeted
**Reviewers:** general
**Type:** alignment
**Location:** `master.md:248-249`; `repo-context/COVENANT_PATCHES.md:71-74`;
`repo-context/docs/RELEASE.md:6,43`
**Issue:** Scope In + COVENANT_PATCHES F14 still frame a disposable `CODEX_HOME`
as the fork contract (F14 Key Behaviors correctly say harness-decided);
RELEASE.md still hedges "unsigned is never promotable once a signer exists"
while OQ4/F33 SC5 forbid an unsigned Release outright; F14 has no SC proving
auth I/O still resolves after `CODEX_HOME` deletion.
**Suggested fix:** Restate Scope In / the F14 patch-index purpose as
`CODEX_AUTH_HOME` under either home model; drop the RELEASE hedge; add an
optional F14 SC (write auth, delete `CODEX_HOME`, auth probe still succeeds from
`CODEX_AUTH_HOME`).
**Routing:** planner.

## Human decision (not a plan defect)

### OQ4 — Authenticode signer unresolved

**Severity:** blocker (shipment), by design
**Reviewers:** adversarial
**Issue:** No signer identity is named. F33 correctly signs before hash /
inventory / provenance / publish and refuses to publish unsigned; F32 depends on
F33. There is no accidental unsigned bypass — the plan is correctly fail-closed.
Shipment is simply blocked until a human names the signer.
**Routing:** human. Does not block the re-plan pass; it blocks F33 execution
only.

## Disagreements

(none) — all three lenses agree the G4 desync and the C3 same-root coupling are
the material risks; adversarial rates them blocker, alignment/general rate the
same underlying gaps targeted. Synthesis keeps the higher severity.
