# Re-review (alignment lens): codex-fork after cycle-2 + cycle-3

The plan was amended twice since the last review: cycle-2 applied the review.md
findings S1–S5 and T1–T5; cycle-3 resolved OQ4 by removing code signing.
Overwrite `docs/master-plans/cross/codex-fork/review-alignment.md` in the
`review.md` artifact format. Rigor: hard. Phase: plan. Reviewers: alignment.

## Read

- `docs/master-plans/cross/codex-fork/master.md`
- `docs/master-plans/cross/codex-fork/review.md` (the prior synthesis with S1–S5,
  T1–T5 — verify each is now resolved)
- `docs/master-plans/cross/codex-fork/repo-context/**` and `schema/decide-v1.json`
- `docs/master-plans/cross/codex-fork/tasks/replan-codex-fork-cycle2.md` and
  `.../replan-codex-fork-cycle3-nosigning.md` (the required edits)
- `docs/master-plans/cross/child-runtime-native/master.md` phases S2, C3, C6,
  C7, G4, P6 (for Section C fidelity)

## Verify (cite line numbers, one verdict per item)

1. Each prior finding S1–S5 and T1–T5 is actually resolved as review.md's
   suggested fix required. Name any still open or only partially applied.
2. OQ4/signing removal is complete and consistent: OQ4 is a resolved decision
   (no signing, SHA-256-pinned); F33 sequence has no sign step and no
   sign-before-hash ordering; F33 criteria dropped the unsigned/signer-unset
   gate; F32 `sha256` is the built-exe hash (no "post-sign"); Constraints,
   Scope Out, Success Criteria, and RELEASE.md carry no signing-gate language;
   remaining signing mentions are only "we deliberately do not sign".
3. No locked decision or preserve-list item was lost across the two passes
   (F11 admission-only, F12/F13 boundaries, exact ALLOW, ALLOW_WITH_CONTEXT=DENY,
   non-retriable CovenantDenied, no agent-loop rewrite, Windows-only, F31
   mandatory, F33 sole release path, new identities default DENY).
4. Section C still names every CRN dependency correctly (G4 complete-env +
   volume_serial/file_index/pre_image_digest + Read-deny on CODEX_AUTH_HOME;
   C3+C6 distinct roots; C7 --covenant-inventory when source=fork; C2/P6 auth
   path; S2 official pin v0.153.4).
5. Grammar/size: plan-lint-clean shape; per-phase MAX_PRODUCES=12 /
   MAX_SUCCESS_CRITERIA=8 (note F11 and F12 counts after the added SCs).

Alignment failure yields FAIL. Report only; do not edit files.
