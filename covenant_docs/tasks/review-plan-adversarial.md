# Review (adversarial lens): codex-fork plan amendment

Try to BREAK the revised `codex-fork` plan. Write
`docs/master-plans/cross/codex-fork/review-adversarial.md` in the `review.md`
artifact format. Rigor: hard. Phase: plan. Reviewers: adversarial. Tag any
finding that blocks shippability `blocker`.

## What to read

- `docs/master-plans/cross/codex-fork/master.md`
- all `docs/master-plans/cross/codex-fork/repo-context/**` docs and
  `schema/decide-v1.json`
- `docs/master-plans/cross/codex-fork/tasks/replan-codex-fork-amendments.md`
- `docs/master-plans/cross/child-runtime-native/master.md` phases S2, C3, C6,
  C7, G4, P6

## Attack surface — find where the amendment introduces a hole

1. **F11 closed table (exec_command + apply_patch only, zero read tools).** Does
   constrained `codex exec` actually FUNCTION with no admitted native read/view
   tool? If Codex's exec loop needs a file-read tool, does denying it break basic
   operation or silently push all reads through `exec_command` in a way that
   defeats the intent? Is F00's inventory required to prove viability before F11
   freezes? Find the failure mode.
2. **Env-scrub vs set-equality (A3/F12 SC7).** Scrubbing `COVENANT_*`/
   `CODEX_AUTH_HOME`/secrets from the child env, then asserting frozen
   `decide_v1.exec.env` is set-equal to the child env — is there any ordering,
   inheritance, or internally-added-var path where the scrub and the freeze
   disagree, or where a scrubbed var still reaches a descendant? Does scrubbing
   `CODEX_AUTH_HOME` from an `exec`'d subprocess break a legitimate nested Codex
   call?
3. **F21 minimal-diff (A6).** With the four deep hook/MCP impl files dropped from
   Produces and only feature-gated, can a hook or MCP path still perform a
   startup side effect or be reachable, such that F21 SC5 / F31 packaging-restore
   would pass while a real path survives? Is "feature-gate + F11 default-deny"
   genuinely sufficient, or does it leave a reachable constructor?
4. **G4 dependency (A9) unamended-state.** The plan says fork hardening is inert
   if G4 is not amended. Is there any criterion in F10/F12/F13 that would go
   GREEN against the CURRENT (unamended) G4, giving a false sense the hardening
   works? Should the fork fail closed when the sidecar schema version/shape does
   not match?
5. **Sign order / promotion (F33) and repro policy (RELEASE.md B2).** After the
   B2 rewrite, is there still any path that pins pre-sign bytes, promotes an
   unsigned build, or requires an impossible independent-rebuild match? Is OQ4
   still a hard blocker with no accidental bypass?
6. **Marker deferral (A8).** With the marker still presence-only this cycle, and
   env-scrub as the only compensating control, can a marker still leak (logs,
   error text, retained output, a non-exec spawn path not covered by A3) and be
   replayed? Is deferral genuinely safe or does it leave an exploitable window?
7. **Baseline latest-tag + audit re-anchor (A1).** Can F00/F01 pass while the
   audit is stale — e.g., a new sink family upstream that the re-anchor scan does
   not enumerate? Is the re-anchor gate concrete enough to actually catch a new
   authority, or is it a checkbox?
8. **CODEX_HOME probe deferral (A4).** By making the home lifetime a C3/C6 probe,
   does the fork plan leave F14's guarantee unverifiable within this plan? Can
   F14 pass while the integrated product still co-locates login and mutable data?

For each: concrete inputs/state → wrong outcome. Prefer refuted-by-default;
only raise a finding you can substantiate. Do not edit files.
