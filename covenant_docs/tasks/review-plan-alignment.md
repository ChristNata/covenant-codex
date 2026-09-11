# Review (alignment lens): codex-fork plan amendment

Review the **revised** `codex-fork` plan and repo-context docs for spec/contract
fidelity. Write `docs/master-plans/cross/codex-fork/review-alignment.md` using
the `review.md` artifact format (Verdict, Findings with severity/type/location/
issue/suggested-fix/routing, Disagreements). Rigor: hard. Phase: plan.
Reviewers: alignment.

## What to read

- `docs/master-plans/cross/codex-fork/master.md` (the revised plan)
- `docs/master-plans/cross/codex-fork/repo-context/docs/DECIDE_V1.md`,
  `RELEASE.md`, `RESIDUALS.md`, `COVENANT_PATCHES.md`, `CLAUDE.md`, `AGENTS.md`,
  `CONTRIBUTING.md`, `README-COVENANT.md`
- `docs/master-plans/cross/codex-fork/schema/decide-v1.json`
- The requirement spec this amendment had to satisfy:
  `docs/master-plans/cross/codex-fork/tasks/replan-codex-fork-amendments.md`
- Cross-plan contract source of truth:
  `docs/master-plans/cross/child-runtime-native/master.md` phases S2, C3, C6,
  C7, G4, P6 (read those sections)

## Alignment questions (answer each; cite line numbers)

1. Does the revised plan faithfully implement every locked decision and accepted
   rec in the requirement spec (sections A1–A11, B1–B7, C)? Name any item
   dropped, weakened, or misapplied.
2. Do the repo-context docs now describe the ACTUAL F10/S2 contracts?
   Specifically: DECIDE_V1 uses complete post-overlay `env` (no `env_allowlist`)
   and full `resolved_identities` (kind/target/`win32_normalized`/
   `volume_serial`/`file_index`/`pre_image_digest`); RELEASE `CODEX_PIN` is
   exactly `source`/`version`/`url`/`sha256`; RESIDUALS reflects hooks
   compiled-out and H3 post-resolution TOCTOU; COVENANT_PATCHES index includes
   F21 and F22.
3. Is `schema/decide-v1.json` consistent with the corrected DECIDE_V1.md and
   with what G4 must evaluate?
4. Are the flagged `child-runtime-native` amendments (section C) stated
   correctly and completely, such that both plans stay contract-compatible? In
   particular, does the G4 dependency demand complete `env` +
   `volume_serial`/`file_index`/`pre_image_digest` evaluation + fork `Read`-deny
   on `$CODEX_AUTH_HOME/auth.json`, matching the live G4 gap?
5. Is the preserve-list honored (F11 admission-only, F12/F13 boundaries, exact
   ALLOW, `ALLOW_WITH_CONTEXT`=DENY, non-retriable `CovenantDenied`, no
   agent-loop rewrite, Windows-only, F31 mandatory, F33 sole release path)?
6. Grammar: every phase has Rigor, bracketed Depends On, exact-file dotted
   Produces, observable criteria; graph acyclic; shared files
   dependency-reachable not concurrent; per-phase counts within
   MAX_PRODUCES=12 / MAX_SUCCESS_CRITERIA=8.

Alignment failure yields FAIL. Do not fix files; report only.
