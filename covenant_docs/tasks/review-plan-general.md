# Review (general lens): codex-fork plan amendment

Holistic correctness and quality review of the revised `codex-fork` plan and
repo-context docs. Write `docs/master-plans/cross/codex-fork/review-general.md`
in the `review.md` artifact format. Rigor: hard. Phase: plan. Reviewers:
general.

## What to read

- `docs/master-plans/cross/codex-fork/master.md`
- `docs/master-plans/cross/codex-fork/repo-context/**` and
  `schema/decide-v1.json`
- `docs/master-plans/cross/codex-fork/tasks/replan-codex-fork-amendments.md`
- `docs/master-plans/cross/child-runtime-native/master.md` phases S2, C3, C6,
  C7, G4, P6

## General questions

1. **Internal consistency after the edits.** Do F00 (seam scout), F21
   (feature-gate + dropped deep-impl Produces), F31 (packaging-restore + sink
   instrumentation), and the C7/F22 inventory contract still cohere? Any dangling
   reference to a Produces file that F21 no longer produces, or a criterion that
   assumes an edit that no longer happens?
2. **Dependency graph and parallelism.** Are Depends On edges still correct after
   the amendments (F00 re-anchor gate, F11 closed table, F12 SC7/SC8, F21
   reduction)? Any new cycle, or a phase that can't go green because a dependency
   it now needs isn't listed?
3. **Criteria are observable and mapped.** Each new/changed criterion (F00
   re-anchor, F12 SC7 env-scrub, F12 SC8 background-descendant, F11 closed table,
   F14 harness-determined home) is a real assertion, not prose. Flag any that a
   test can't check.
4. **Doc-plan agreement.** Do the corrected repo-context docs match the plan they
   describe (DECIDE_V1↔F10, RELEASE↔S2/F32/F33, RESIDUALS↔F13/F14/F21,
   COVENANT_PATCHES↔F11–F14+F21+F22)? Any residual drift left uncorrected?
5. **Completeness of the cross-plan flag (section C).** Is anything the fork
   depends on from `child-runtime-native` missing from the flagged list, such
   that the two plans could ship contract-incompatible?
6. **Grammar / size.** plan-lint-clean grammar; per-phase counts within
   MAX_PRODUCES=12 / MAX_SUCCESS_CRITERIA=8; `likely-too-big` warnings are
   advisory input, not automatic rejection — judge whether any flagged phase is
   genuinely too big to implement in one pass.

Report findings with severity/type/location/issue/suggested-fix/routing. Do not
edit files.
