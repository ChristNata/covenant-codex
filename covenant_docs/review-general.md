# Review: codex-fork

**Date:** 2026-09-05
**Rigor:** hard
**Phase:** plan
**Reviewers:** general

## Verdict

PASS

Cycle-2 and cycle-3 land as one coherent contract. Signing-gate
language is gone; remaining `sign` mentions are the deliberate
no-sign policy. F00's `AUTHORITY-INVENTORY.toml`, F11 SC6/SC8,
F12 SC7/SC8, F31 SC8, F33's hash-then-inventory-then-publish
sequence, and F32's built-exe `sha256` still fit. Graph is
acyclic with F31's extra F00 edge. Shared files still serialize.
Counts stay inside MAX_PRODUCES=12 / MAX_SUCCESS_CRITERIA=8.
Section C names every CRN amendment this fork needs. Three
targeted leftovers: F11 SC8 is on the wrong side of F21, F32
SC5 names a CRN test nobody is contracted to write, and F11 /
F12 / F31 are legally sized but fat for one pass.

## Checks

| Question | Verdict |
|---|---|
| Consistency after two passes | PASS — no dangling signer/signature Produce or criterion; F00 inventory → F31 SC8; F11 no longer owns `sink_instrumentation.rs`; F33 has no sign-before-hash step; F32 pins the built-exe digest |
| Dependency graph / parallelism | PASS — F31 `[F00, F11, F12, F13, F14, F22, F40]` is acyclic (F00 already an ancestor via F11/F12/F14); `main.rs` F21→F22; `covenant-release.yml` F03→F33; `COVENANT_PATCHES.md` F40→F31(read)→F33; G4/C3 sidecar and distinct-root waits stay cross-plan |
| Criteria observable | MIXED — F11 SC6, F12 SC7/SC8, F14 SC7, F31 SC8 mutations, F33 asset/digest checks bite; F11 SC8 cannot bite at F11; F32 SC4/SC5 are phase-entry gates, not assertions on `pinned_versions.rs` |
| Doc-plan agreement | PASS — DECIDE_V1/schema `if`/`then` match F10 SC3; RELEASE.md matches no-sign F33/F32; Scope / Success Criteria / COVENANT_PATCHES carry no disposable-`CODEX_HOME` contract and no signing gate |
| Section C completeness | PASS with one targeted owner gap — G4 complete-env + identities, C3+C6 distinct roots, C2/P6 `$CODEX_AUTH_HOME`, C7 `--covenant-inventory`, S2 bump, F1.4 stays accepted are all named; F32's real-adapter test is not a required CRN SC |
| Size | PASS counts; advisory fat — F11/F12/F31 sit at 8 SCs; F40 is 9 Produces (under 12) |

## Findings

### F11 SC8 proves two-row viability before F21 strips the advertised schema

**Severity:** targeted
**Reviewers:** general
**Type:** correctness
**Location:** `docs/master-plans/cross/codex-fork/master.md:219-220,651-653,751-758,1093-1111`
**Issue:** F11 SC8 requires a fixture-provider repository-read
turn (`turn.completed` with known file content) whose captured
tool plan contains no denied native read/view identity and no
excluded surface, against the F02 exe plus this phase's
admission table. F11 and F21 are declared parallel after
F00+F02. At F11 time `spec_plan.rs` still advertises upstream
read/hosted/MCP identities; F11 only denies them at dispatch.
A cooperating model that picks a still-advertised read tool
fails SC8, and the written failure action is "re-plan the
two-row table" — the wrong re-plan. A stubbed two-tool schema
makes the same SC green without proving viability. The
criterion is a real assertion in the wrong phase.
**Suggested fix:** Move the viability turn to F22 (already
Depends On F11+F21) and require the captured schema to be the
F21-constrained advertised set. Keep F11 SC6 as the
admission-only writer/`read`/`none` deny matrix. Do not
re-plan the two-row table on an F11-only red.
**Routing:** planner

### F32 SC5 names a CRN separation test with no CRN owner

**Severity:** targeted
**Reviewers:** general
**Type:** alignment
**Location:** `docs/master-plans/cross/codex-fork/master.md:237-246,1385-1392,1422-1426`;
live C3 `docs/master-plans/cross/child-runtime-native/master.md:1448-1502,1541-1545`
**Issue:** F32 SC5 (and the C3+C6 bullet) refuse adoption
until a real-adapter separation test proves auth I/O stays
under the stable auth root while probe-classified mutable
paths follow the selected home model. F32's only Produce is
`pinned_versions.rs`. This plan's Out-list forbids editing
CRN. Live C3 SC8 only checks that the four fork env keys are
set, both to S5 `config_dir`. Section C tells CRN to add the
probe and distinct-root rule, but it does not require C3/C6
to grow a named criterion/Produce for that separation test.
F32 SC5 is then a prose gate nobody in either plan is
contracted to implement.
**Suggested fix:** In the C3+C6 amendment bullet, require a
CRN C3 (or C6) success criterion for the real-adapter
separation test: `CODEX_AUTH_HOME ≠ CODEX_HOME` when the probe
permits, auth I/O under the stable auth root, mutable paths
on the selected home model. Keep F32 SC5 as the adoption
refuse that waits on that CRN criterion being green.
**Routing:** planner (this workspace's Section C text only)

### F11, F12, and F31 are at the SC cap and each now carries a second job

**Severity:** targeted
**Reviewers:** general
**Type:** correctness
**Location:** `docs/master-plans/cross/codex-fork/master.md:726-758,883-934,1279-1302`
**Issue:** Counts are legal (every phase ≤8 SCs, ≤12
Produces; F40 is 9 Produces). Three phases are still a
poor one-pass bet after the added SCs. F11 stacks a closed
admission matrix with an end-to-end turn (SC8). F12 stacks
the exec gate, sidecar discriminating tests, the full
Windows env-block/scrub/set-equality matrix (SC7), and
background/detached settlement that fail-and-replans (SC8).
F31 stacks four-hunk reaudit CI with per-family inventory
mutations including startup/direct-MCP and hosted-provider
paths (SC8). F12 SC8 is now honest about bouncing to
planning, which means F12 may not independently go green.
This is reviewer size input, not a lint fault.
**Suggested fix:** Leave counts as-is if the orchestrator
accepts bounce-to-replan on F12 SC8. If one-pass is required,
split F12 env+settlement into a follower that Depends On the
gate module, and keep F31 instrumentation vs
`covenant-reaudit.yml` as two phases sharing no file. Do not
split unless a bounce actually happens.
**Routing:** planner (advisory)

### Why still labels the signing decision "OQ4"

**Severity:** trivial
**Reviewers:** general
**Type:** alignment
**Location:** `docs/master-plans/cross/codex-fork/master.md:175-182,1652-1678`
**Issue:** Why records "OQ4 resolved (user decision): no
Authenticode code signing." Open Questions item 4 is now
marker-as-capability. A reader chasing OQ4 lands on the
wrong decision. No behavior or gate is affected.
**Suggested fix:** Call the signing call a resolved decision
in Why without the OQ4 number, or add a one-line "former OQ4"
note under Open Questions.
**Routing:** planner

## Disagreements

(none)
