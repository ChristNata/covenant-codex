# Final alignment review: codex-fork

Single alignment-lens pass after inline fixes to the adversarial cycle's four
blockers. Overwrite `docs/master-plans/cross/codex-fork/review-alignment.md` in
the `review.md` artifact format. Rigor: hard (single lens by user direction).
Phase: plan. Reviewers: alignment. Verdict PASS / PASS-with-issues / FAIL.

## What changed (verify each landed and is internally consistent)

1. **Discriminating G4 tests.** F13 Key Behaviors and F13 SC1 now require three
   independent pairs — each differing only by `volume_serial`, only by
   `file_index`, and only by `pre_image_digest` — so a partial G4 that evaluates
   only one field fails. F12 SC1 env pair uses a key chosen at test time. The
   Dependencies G4 row lists the same expanded discriminating set plus a `Read`
   pair on `$CODEX_AUTH_HOME/auth.json`.
2. **F32 cross-plan gate.** F32 SC5 refuses adoption until CRN C3+C6 land with
   `CODEX_AUTH_HOME ≠ CODEX_HOME` and a real-adapter separation test passes.
   (Confirm this is present and coherent; it predates this pass.)
3. **Inventory commit-binding.** F31 Key Behaviors + SC8 fail the job if
   `AUTHORITY-INVENTORY.toml`'s recorded commit ≠ `UPSTREAM.toml` commit;
   inventory completeness is documented as an accepted human-audit residual in
   `repo-context/docs/RESIDUALS.md` H4.
4. **Build-provenance attestation (not code-signing).** F33 Key Behaviors + SC6
   generate a GitHub `actions/attest-build-provenance` attestation binding the
   exe digest to the workflow/commit/tag; F32 Key Behaviors + SC6 verify it
   (`gh attestation verify`) before pinning; `repo-context/docs/RELEASE.md`
   Provenance section documents it. No signer/certificate anywhere.

## Read

- `docs/master-plans/cross/codex-fork/master.md`
- `docs/master-plans/cross/codex-fork/review.md` (prior synthesis / findings)
- `docs/master-plans/cross/codex-fork/repo-context/docs/RELEASE.md`,
  `RESIDUALS.md`
- `docs/master-plans/cross/child-runtime-native/master.md` C3, C6, G4 (Section C
  fidelity)

## Alignment checks (cite line numbers)

1. Each of the four fixes above is present, internally consistent, and does not
   contradict another phase or the repo-context docs.
2. No SC-cap violation (per-phase ≤ 8 success criteria); F31/F32/F33 counts.
3. Preserve-list intact (F11 admission-only, F12/F13 boundaries, exact ALLOW,
   ALLOW_WITH_CONTEXT=DENY, non-retriable CovenantDenied, no agent-loop rewrite,
   Windows-only, F31 mandatory, F33 sole release path, new identities default
   DENY, no code-signing).
4. Section C still names the CRN dependencies correctly (G4 complete-env +
   three identity fields + Read-deny on CODEX_AUTH_HOME; C3+C6 distinct roots;
   C7 `--covenant-inventory`; C2/P6 auth path; S2 official pin v0.153.4).
5. Grammar: plan-lint shape holds (bracketed Depends On, dotted Produces,
   observable criteria, acyclic, shared files serialized).

Report findings with severity/type/location/issue/fix/routing. Note any residual
the plan now *accepts* (e.g., inventory completeness, mutable-metadata beyond
attestation) as acknowledged residuals rather than defects, since this is a
single-user local tool. Do not edit files.
