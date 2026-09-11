# Task: amend the codex-fork plan and repo-context docs

You are the **planner**. Revise the existing `codex-fork` master plan and its
`repo-context/` documents in place. Do **not** redesign the fork architecture:
F11 tool admission, F12 final exec gate, F13 final patch gate remain the three
approved enforcement points. This is a targeted amendment, not a rewrite.

## Files you own and edit

- `docs/master-plans/cross/codex-fork/master.md` (revise in place)
- `docs/master-plans/cross/codex-fork/repo-context/docs/DECIDE_V1.md`
- `docs/master-plans/cross/codex-fork/repo-context/docs/RELEASE.md`
- `docs/master-plans/cross/codex-fork/repo-context/docs/RESIDUALS.md`
- `docs/master-plans/cross/codex-fork/repo-context/COVENANT_PATCHES.md`
- `docs/master-plans/cross/codex-fork/repo-context/CLAUDE.md`
- `docs/master-plans/cross/codex-fork/repo-context/AGENTS.md`
- `docs/master-plans/cross/codex-fork/repo-context/CONTRIBUTING.md`
- `docs/master-plans/cross/codex-fork/repo-context/README-COVENANT.md`
- `docs/master-plans/cross/codex-fork/schema/decide-v1.json` (keep byte-identical
  to F10's `fork:covenant/schema/decide-v1.json`; update to match the corrected
  DECIDE_V1 contract below)

## Do NOT edit

- Any file under `docs/master-plans/cross/child-runtime-native/`. You only
  *name* the required CRN amendments as flagged dependencies inside
  codex-fork `master.md` (see section C). CRN is a separate plan.
- `backend/**` — the only harness-side write remains F32's `CODEX_PIN` row,
  already in the plan; do not add others.

## Keep grammar plan-lint clean

Every phase keeps `**Rigor:**`, bracketed `**Depends On:**`, exact-file
`**Produces:**` (dotted filenames), and observable criteria. Graph stays
acyclic. Shared files stay dependency-reachable, not concurrent. Counts stay
under MAX_PRODUCES=12 and MAX_SUCCESS_CRITERIA=8 per phase.

---

## Locked user decisions to fold in

1. **Baseline = latest stable `rust-v*`** (not frozen 0.153.0). F01 roots at the
   latest stable upstream tag chosen at fork time and records its exact tag +
   commit. Add an **audit re-anchor gate** to F00/F01: the source security audit
   (four seams + sink families) must be re-confirmed at the chosen tag; a newly
   introduced sink or tool identity fails the phase and forces a re-plan of
   Produces. Rationale: the deep audit was done at 0.153.0; forking a newer tag
   without re-anchoring bolts gates onto un-audited code.
2. **Adoption = fork-first.** `CODEX_PIN.source` default is `fork` once F33
   publishes a signed Release; `official` is the documented fallback (initial
   testing + when a rebase is mid-flight/broken). Update the "Adoption sequence"
   section and the Why. It is a harness-side pin setting, not a mode inside the
   binary.
3. **Repo public.** `covenant-capital/codex` is public; the harness pull path
   stays token-free. No change needed beyond confirming this in docs.
4. **Daily upstream-tracking automation is documentation-only.** Add the
   following routine to `CONTRIBUTING.md` (rebase section) and cross-reference
   from `README-COVENANT.md`, explicitly marked **informational — not
   implemented by this plan; set up manually by the maintainer**:
   > A GitHub Action runs once per day and checks upstream for a new stable
   > release. If nothing changed it exits. If a new release exists it updates the
   > fork's clean `main` to match that upstream release exactly, then creates a
   > temporary upgrade branch from the customized `covenant` branch and opens a
   > PR. That PR triggers Codex Cloud, which merges updated `main` into the
   > upgrade branch, resolves conflicts, preserves the custom changes, performs
   > cleanup for the new version, runs tests/lint/build/validation, and pushes
   > fixes back to the PR branch. Once checks pass, the maintainer reviews or
   > auto-merges the PR into `covenant`.
   No workflow file is produced; OQ1 is reworded to "manual today; this daily
   routine is the intended maintainer-owned automation."

---

## Section A — codex-fork `master.md` edits

A1. **F01 / F00 baseline + audit re-anchor** — per locked decision 1.

A2. **Adoption fork-first** — per locked decision 2 (Adoption sequence + Why +
    Scope where they mention official-first).

A3. **F14 env-scrub (rec #2, ACCEPTED).** The environment of a model-spawned
    exec subprocess must exclude the Covenant control variables and secrets:
    `COVENANT_DECIDER_PATH`, `COVENANT_DECIDER_SHA256`, `COVENANT_CHILD_MARKER`,
    `CODEX_AUTH_HOME`, and unrelated provider/login secrets. Scrub them before
    freezing the F12 decide object, so the frozen `decide_v1.exec.env` stays
    set-equal to the actual (scrubbed) child environment. Add a discriminating
    F12 success criterion: a spawned subprocess that inspects its own env must
    not observe any of those keys, and the frozen decide object must match.
    This reinforces F12 set-equality; it is not a new gate.

A4. **F14 `CODEX_HOME` lifetime = harness-determined / probe-driven (user
    decision 1).** The fork honors the `CODEX_AUTH_HOME` login/disposable split
    regardless of which home model the harness selects. Do NOT hard-code a
    disposable-`CODEX_HOME` diagram into F14. State that the `CODEX_HOME`
    lifetime is decided by the harness adapter based on a redirectability probe
    (flagged in section C): if `CODEX_HOME` can be fully boxed, isolated, and
    disposed per-attempt with no downside versus the current stable model, the
    harness runs per-attempt isolation; otherwise it keeps the stable
    `CODEX_HOME` + redirected sub-dirs model. F14's contract is compatible with
    both.

A5. **F12 background-descendant success test (rec #4, ACCEPTED as
    acceptance-test-first).** Add an F12 criterion: an explicitly ALLOWED exec
    that spawns a detached/background child then exits must leave zero surviving
    descendant before the exec tool settles and before Codex takes its next
    model/tool turn. Acceptable results: (1) policy rejects the
    background/detached pattern, or (2) Codex process supervision reaps the
    descendant before settlement. Do not rely on the outer Job Object killing
    everything at session end. Treat as an acceptance test first; only add a
    source patch (expand F12 Produces) if the test proves current Codex
    supervision cannot satisfy the invariant.

A6. **F21 minimal upstream diff (rec #5, ACCEPTED).** Reframe F21 to achieve its
    capability-removal target with the **smallest** upstream diff: prefer a
    Covenant compile/build profile or feature flags + immutable managed clamps +
    F11 compiled default-deny over rewriting/removing large sections of the
    upstream MCP/hook implementations. Only modify deeper MCP/hook
    implementation paths when tests demonstrate they remain reachable or can
    perform startup side effects despite being disabled. Keep the full
    capability exclusion list (TUI, app-server, MCP client/server-management,
    `write_stdin`/interactive terminals, multi-agent/subagents, Code Mode,
    plugins/apps, image/browser/computer-use, hosted web/provider-hosted tools,
    dynamic/extension authorities, ordinary command/MCP hooks as an
    enforcement/execution surface). Keep all hostile-override tests. Where this
    lets you drop a Produces file (e.g. a deep MCP impl edit no longer needed),
    remove it from F21 Produces and say why.

A7. **F11 freeze exact read-tool set (rec #7, ACCEPTED).** F00 inventories and
    classifies identities, but F11 must NOT auto-admit every identity an
    implementer classifies `read`/`none`. The admitted const table is
    `exec_command` + custom `apply_patch` + only read tools **explicitly named
    and approved in the F11 table in this plan**. A newly discovered upstream
    read tool is DENY until reviewed and added by an explicit plan amendment.
    Update F11 Key Behaviors and criteria: sink-clean is necessary but not
    sufficient; explicit plan approval is required.

A8. **Marker stays present-only this cycle; capability upgrade is a documented
    follow-up (rec #3 → user decision 2 = follow-up).** Keep the current
    opaque, presence-checked `COVENANT_CHILD_MARKER` semantics. Do NOT add
    nonce/WorkerContract binding to F10/F12/F13 this cycle. Add an Open Question
    / follow-up note describing the future upgrade: make the marker a
    high-entropy per-dispatch capability the decider resolves to an immutable
    launcher-owned WorkerContract (run_id, child_id, role, worktree, workspace,
    allowed Produces, protected artifacts, policy scope); every Exec/Patch
    decision bound to it; unknown/expired/reused/mismatched marker => DENY. Note
    it is cross-plan (CRN C3 mint+store, G4 lookup, fork envelope carries the
    marker) and that rec #2's env-scrub (A3) is what makes deferring it safe
    (the marker cannot leak into spawned subprocesses in the interim).

A9. **G4 dependency tightened.** Update the Dependencies table entry for
    `child-runtime-native` G4: the amended G4 must consume the complete
    post-overlay exec `env` (not an `env_allowlist` snapshot) and must include
    AND evaluate patch `resolved_identities` with `volume_serial`, `file_index`,
    and `pre_image_digest`, and the `Read`-kind auth-deny path must target
    `$CODEX_AUTH_HOME/auth.json` for the fork arm. State plainly: if G4 is not
    amended, F10/F12/F13's hardening is inert because the decider ignores the
    extra fields. (You are naming the dependency, not editing G4.)

A10. **Branch/release topology (rec #8, ACCEPTED).** Add to Constraints /
     Invariants: upstream `openai/codex` -> fork `main` = clean adopted upstream
     stable tag with no Covenant behavioral patches; `covenant` branch = `main`
     + F11/F12/F13/F14 + F21/F22. Never merge `covenant` into `main`. Releases
     (F03 CI, F33 promotion) build only from the audited `covenant` branch/tag.
     Adopting a new upstream tag: update clean `main`; merge/rebase `main` into
     `covenant`; resolve conflicts; rerun F31; rebuild the Windows artifact;
     test; publish only when green.

A11. **OQ1 reword** — per locked decision 4.

## Section B — repo-context doc corrections (fix canonical, then F40 copies)

Update F40 so it copies the **corrected** files byte-identically; drop any
"copy stale then let master.md override later" framing.

B1. **DECIDE_V1.md** — bring into sync with F10/F13:
    - Replace `ExecCall.env_allowlist` with the **complete post-overlay
      process environment** (`env`, the exact map the spawned process sees).
    - Add `PatchCall.resolved_identities`: resolved existing ancestor
      identities; file/directory/symlink/junction kind; link/junction targets
      where applicable; `\\?\` normalization / `win32_normalized`; Windows file
      identity (`volume_serial`, `file_index`); pre-image content digest
      (`pre_image_digest`) — the identity information F13 needs to detect
      ancestor replacement, junction swaps, and reparse-point races.
    - Fix the exec/patch example JSON to match.
    - The document must describe the actual schema F10 produces and G4
      evaluates.

B2. **RELEASE.md** — sync harness-consumption with S2/F32:
    - `CODEX_PIN` has exactly four fields: `source`, `version`, `url`,
      `sha256`. Remove `source_commit`, `inventory_digest`, `signature_id`
      from the pin table; state they live in the Release `provenance.json` /
      inventory evidence, not in the pin.
    - Resolve the reproducibility/signing contradiction into one coherent
      policy: (1) reproducibility is desirable and tested where possible;
      (2) SHA-256 identifies the promoted CI artifact; (3) provenance records
      source/toolchain/runner; (4) Authenticode signing is the preferred
      release identity once available; (5) do NOT require identical independent
      builds as the fallback unless Windows Codex byte-reproducibility has first
      been proven. Keep "unsigned is never promotable" once a signer exists per
      OQ4.

B3. **RESIDUALS.md** — update to the latest design:
    - Ordinary Codex hooks are **compiled out / non-authoritative** in the
      promoted artifact; describe the residual accordingly rather than framing
      hook fail-open as the active Covenant boundary.
    - H3: F13 now **resolves** ancestor/link/junction identities before
      authorization; the remaining filesystem residual is the TOCTOU/reparse
      race **after** that resolution, not "identity is delegated to the
      harness."
    - Keep shell-mediated auth-store access as an accepted residual while native
      shell remains admitted.

B4. **COVENANT_PATCHES.md** — resolve the completeness claim (rec #6,
    preferred = expand): add index-table rows for **F21** (constrained
    packaging/profile) and **F22** (inventory/certificate) so the index is the
    single unambiguous answer to "what does Covenant modify relative to this
    upstream tag." Keep the existing packaging-invariants prose consistent.

B5. **CLAUDE.md / AGENTS.md** — update the Adoption sequence to fork-first; add
    the branch/release topology (A10); keep the marker described as opaque and
    add a one-line pointer to the follow-up capability upgrade.

B6. **CONTRIBUTING.md / README-COVENANT.md** — add the branch/release topology
    and the info-only daily `gh`+Codex-Cloud routine (locked decision 4).

B7. **schema/decide-v1.json** — update to match B1 and keep byte-identical to
    F10's `fork:covenant/schema/decide-v1.json` Produces (state the
    byte-identity requirement in F10).

## Section C — child-runtime-native amendments to FLAG (do not edit CRN)

Add a clearly labeled "Cross-plan dependencies to amend in
`child-runtime-native`" subsection to codex-fork `master.md` (near Dependencies
or Open Questions). List exactly:

- **G4** — take the complete post-overlay exec `env` (drop `env_allowlist`);
  add and evaluate `volume_serial`/`file_index`/`pre_image_digest` in
  `resolved_identities`; `Read`-deny path -> `$CODEX_AUTH_HOME/auth.json` for
  the fork arm. MUST land or fork hardening is inert.
- **C3 + C6** — add the `CODEX_HOME` disposability probe and branch C3 on its
  result (per-attempt disposable if fully redirectable with no cons; else keep
  stable `CODEX_HOME` + redirected sub-dirs). (user decision 1)
- **S2** — bump the official Codex pin `0.150.1` -> `v0.153.4`; ensure the
  `source: official|fork` discriminant exists. (user earlier decision)
- **C7 / P6** — verify inventory-certificate fields still match F22 after the
  DECIDE/RELEASE doc corrections; P6 canary auth path follows the
  `CODEX_AUTH_HOME` resolution.
- **Follow-up plan** — marker-as-capability (nonce + WorkerContract) across
  CRN C3/G4/`decision.rs` and the fork decide envelope. (user decision 2)

## Preserve unchanged (unless a new audit requires otherwise)

F11 = identity admission only. F12 = final resolved process-start boundary.
F13 = final resolved patch boundary. Stock `PreToolUse` is never enforcement.
Only exact `{"decision":"ALLOW"}` authorizes; `ALLOW_WITH_CONTEXT` = DENY.
Policy unavailable/malformed/timeout/crash/nonzero/identity-mismatch = DENY.
`CovenantDenied` is non-retriable, never a sandbox retry. No agent/reasoning/
turn-loop rewrite. Windows x64 only. New upstream tool/executor identities
default DENY. F31 semantic sink/inventory re-audit stays mandatory before every
promotion. F33 is the only promotable release path.

## Deliverable

Revise `master.md` and the repo-context docs in place. In your returned result,
summarize the concrete changes made per section (A/B/C) and explicitly list the
flagged `child-runtime-native` amendments so both plans stay contract-compatible.
Note any place where an edit surfaced a new contradiction you could not resolve
within scope.
