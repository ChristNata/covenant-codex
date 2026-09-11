# Task: codex-fork re-plan pass (review-plan cycle 1 findings)

You are the **planner**. Apply the findings in
`docs/master-plans/cross/codex-fork/review.md` to the `codex-fork` plan and
repo-context docs. Do NOT redesign the architecture: F11 admission-only, F12
final exec gate, F13 final patch gate remain the three enforcement points, and
the preserve-list in the prior task still holds. Read `review.md` in full first;
it has exact locations and suggested fixes. This task is the checklist.

## Files you own (same as before)

`master.md` and every `repo-context/**` doc + `schema/decide-v1.json`. Do NOT
edit anything under `child-runtime-native/`. Keep plan-lint-clean grammar;
per-phase MAX_PRODUCES=12 / MAX_SUCCESS_CRITERIA=8 (note F12 is already at 8 —
if a fix adds an F12 criterion, consolidate rather than exceed 8).

## Structural fixes (change a gate or criterion)

S1 — **Bind promotion to the amended G4.** Add to F12/F13 real-sidecar
discriminating tests: a pair of `decide_v1` objects differing only by ONE
authoritative field (an exec env pair; a patch `volume_serial`, `file_index`, or
`pre_image_digest`) must produce different decisions. Pin an attested G4
schema/semantics identifier (not just the exe digest) in
`covenant/sidecar-fixture.toml`. Make F33 (promotion) and F32 (adoption) refuse
until those discriminating tests pass against the exact sidecar version the
harness will use. The CRN G4 amendment stays a flagged precondition — do not
edit G4.

S2 — **Gate F14's benefit on a real split.** Strengthen the Section C C3+C6
bullet to require `CODEX_AUTH_HOME ≠ CODEX_HOME` when the probe permits (today
live C3 sets both to S5 `config_dir`). Add an F32 adoption precondition: landed
C3+C6 amendments plus a real-adapter separation test proving auth I/O stays
under the stable auth root while probe-classified mutable paths follow the
selected home model. Do not edit C3/C6.

S3 — **Make the env-scrub Windows-correct and closed.** In F12/A3: define one
immutable Windows environment block as the single source for both the frozen
`decide_v1.exec.env` and the spawned process; canonicalize env keys
case-insensitively; reject case-colliding/invalid entries; freeze a CLOSED
secret-key set = the four control keys (`COVENANT_DECIDER_PATH`,
`COVENANT_DECIDER_SHA256`, `COVENANT_CHILD_MARKER`, `CODEX_AUTH_HOME`) plus the
provider/login keys F00 records at the tag (or a documented prefix set). Rewrite
SC7 to assert each named key is absent from the real descendant env (mixed-case
and collision cases included) and that `exec.env` is set-equal to that block.
Mirror the case-insensitive key semantics in `DECIDE_V1.md`.

S4 — **Give F00 an executable completeness artifact.** F00 additionally produces
a commit-bound authority inventory + audit procedure enumerating all model and
non-model constructors and process/filesystem/network/hosted/MCP/hook/
generated-code sinks with source anchors and reviewer disposition. F31 consumes
that inventory (not a rediscovered set) and seeds one negative mutation per sink
family, including startup/direct-MCP and hosted-provider paths. Keep counts
within limits — if this needs a new Produce on F00, add it.

S5 — **Prove two-tool viability before F11 freezes.** Add a deterministic
fixture-provider acceptance test against the F02 exe: complete a repository-read
turn (`turn.completed` with the file content) using only `exec_command` + custom
`apply_patch`, proving no denied native read identity or excluded surface was
needed. Place it so it gates the F11 table freeze.

## Targeted fixes

T1 — Rewrite F12 SC8 to F00's fail-and-replan pattern. Observable proof = zero
surviving descendants at exec-tool settlement (process tree after the tool
result, NOT "before the next model turn"). Pre-authorize a named Produce only if
the scout already shows Codex supervision cannot reap; otherwise a red SC8
returns to planning. Do not require a turn-loop harness.

T2 — Resolve `covenant/tests/sink_instrumentation.rs` ownership to exactly one
phase (remove the F11/F31 duplicate-Produce / circular reference). Gate F11 SC6
on dispatch of named upstream writer identities from the F00 list (MCP, hosted,
web, `write_stdin`, etc.) plus any F00 `read`/`none` identity — deny before the
handler, with no dependence on F31's file. Keep sink-class agreement on F31.

T3 — Add JSON-schema `if`/`then` conditionals matching `DECIDE_V1.md`: require
`pre_image_digest` for `update|delete|move` source ops and `kind == file`
identities; require `target` for `symlink|junction`; require ancestor
`win32_normalized`. Keep them optional for `add` / directory kind. Align F10 SC3
(pre_image_digest is not always present). Keep the workspace copy byte-identical
to `fork:covenant/schema/decide-v1.json`.

T4 — Strengthen Section C (do not edit CRN): C7 must invoke
`codex --covenant-inventory` when `source = fork`; C2 `deny_read` and P6 canary
follow `$CODEX_AUTH_HOME/auth.json`; C3+C6 keep `CODEX_AUTH_HOME ≠ CODEX_HOME`;
F1.4 stays accepted on the fork arm. Retarget the Why G4 citation away from the
stale `replan-d-g4-schema.md` to the Cross-plan G4 bullet (the correct
demand: complete post-overlay `env`, evaluate
`volume_serial`/`file_index`/`pre_image_digest`, fork `Read`-deny on
`$CODEX_AUTH_HOME/auth.json`).

T5 — Remove leftover drift: restate Scope In and the COVENANT_PATCHES F14 row as
`CODEX_AUTH_HOME` under either harness home model (drop the disposable-`CODEX_HOME`
framing); drop the RELEASE.md "unsigned is never promotable once a signer exists"
hedge (OQ4/F33 SC5 forbid an unsigned Release outright); add an optional F14 SC —
write auth, delete `CODEX_HOME`, auth probe still succeeds from `CODEX_AUTH_HOME`.

## Do NOT touch

OQ4 (Authenticode signer) — it is a standing human decision, correctly
fail-closed. Leave it as an Open Question; do not invent a signer or a bypass.

## Deliverable

Revise the files in place. In your returned result, list each finding (S1–S5,
T1–T5) with the concrete edit made and the line(s) touched, and note any finding
you could not fully resolve within scope (with why). Do not self-certify; the
orchestrator re-runs review-plan.
