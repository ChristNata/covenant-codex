# Re-review (general lens): codex-fork after cycle-2 + cycle-3

Holistic correctness/quality after two edit passes (cycle-2 = review.md
findings; cycle-3 = signing removal). Overwrite
`docs/master-plans/cross/codex-fork/review-general.md` in the `review.md`
format. Rigor: hard. Phase: plan. Reviewers: general.

## Read

- `docs/master-plans/cross/codex-fork/master.md`, `review.md`,
  `repo-context/**`, `schema/decide-v1.json`
- `docs/master-plans/cross/child-runtime-native/master.md` S2, C3, C6, C7, G4, P6

## Questions

1. **Consistency after two passes.** Any dangling reference to a dropped field
   (signer/signature) or a Produces/criterion the edits invalidated? Do F00
   (now with AUTHORITY-INVENTORY.toml), F11 (SC6/SC8), F12 (SC7/SC8), F31 (SC8
   consuming the inventory), F33 (no-sign sequence), and F32 still cohere?
2. **Dependency graph / parallelism.** F31 Depends On now includes F00 — still
   acyclic? Any phase that can't go green because a needed dependency isn't
   listed? Shared files still serialized?
3. **Criteria observable.** The new/changed criteria (F11 SC6 & SC8, F12 SC7 &
   SC8, F14 SC7, F31 SC8, F33 no-sign criteria, F32 SC4/SC5) are real
   assertions, not prose.
4. **Doc-plan agreement.** DECIDE_V1/schema conditionals match F10; RELEASE.md
   matches the no-sign F33/F32; COVENANT_PATCHES + Scope + Success Criteria carry
   no disposable-CODEX_HOME or signing-gate leftovers.
5. **Section C completeness** for contract compatibility with child-runtime-native.
6. **Size.** Per-phase counts within MAX_PRODUCES=12 / MAX_SUCCESS_CRITERIA=8
   after the added SCs; flag any phase now genuinely too big for one pass
   (advisory, with your judgment).

Report findings with severity/type/location/issue/fix/routing. Do not edit files.
