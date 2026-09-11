# Contributing to the Covenant Codex fork

Upstream first: if OpenAI accepts a Covenant patch or provides an equivalent
upstream mechanism, remove the fork patch after verifying the upstream behavior.
The fork exists only for deltas that upstream does not yet provide.

## Contribution rules

| Change | Required action |
| --- | --- |
| Upstream-equivalent fix | Follow U1. |
| F11, F12, F13, or F14 | Follow U2. |
| Tool identity or authority | Follow U3. |
| Packaging or catalog | Follow U4. |
| Schema | Follow U5. |

## U1 — upstream-equivalent fix

Propose it upstream first. When accepted and verified, delete the duplicate fork
delta and retain its provenance in the patch index.

## U2 — maintained patch change

Update the matching `COVENANT_PATCHES.md` row, its tests, and re-audit coverage.

## U3 — tool identity or authority

Treat it as denied by default. Propose a new audited patch entry; do not enable
it through configuration or environment.

## U4 — packaging or catalog

Prove constrained `codex exec` remains the only published surface and update
inventory evidence.

## U5 — schema

Coordinate the `decide_v1` version with the harness before implementation.

Every fork-only proposal also requires a plan amendment in the Covenant Harness
repository. The fork must not silently become a general Codex customization
branch.

## Branch and release topology

Upstream `openai/codex` feeds fork `main`, which is a clean adopted
upstream stable tag with no Covenant behavioral patches. The
`covenant` branch is `main` plus F11/F12/F13/F14 and F21/F22. Never
merge `covenant` into `main`. Releases build only from the audited
`covenant` branch/tag.

Adopting a new upstream tag: update clean `main`; merge/rebase `main`
into `covenant`; resolve conflicts; rerun re-audit; rebuild the
Windows artifact; test; publish only when green.

## Daily upstream-tracking routine

**Informational — not implemented by this plan; set up manually by
the maintainer.**

A GitHub Action runs once per day and checks upstream for a new
stable release. If nothing changed it exits. If a new release exists
it updates the fork's clean `main` to match that upstream release
exactly, then creates a temporary upgrade branch from the customized
`covenant` branch and opens a PR. That PR triggers Codex Cloud, which
merges updated `main` into the upgrade branch, resolves conflicts,
preserves the custom changes, performs cleanup for the new version,
runs tests/lint/build/validation, and pushes fixes back to the PR
branch. Once checks pass, the maintainer reviews or auto-merges the
PR into `covenant`.

No workflow file in this repository implements that routine today.
Until it exists, rebase is manual and gated by re-audit CI.

## Re-audit locally

Use the repository CI script or documented test target that performs the same
checks as the re-audit job. A local re-audit must:

1. compare this tree with the recorded upstream tag and commit;
2. confirm the F11 admission, F12 process-start, F13 patch, and F14 auth seams;
3. run the exact-ALLOW and zero-effect failure matrices;
4. inventory registered identities and their effect classes;
5. confirm excluded product surfaces remain absent from the artifact; and
6. produce the candidate hunk digests for `COVENANT_PATCHES.md`.

Local success is evidence, not promotion. The CI re-audit is required before a
release may be consumed by Covenant.

## Review standard

Do not accept a return-shape-only test for a policy boundary. Tests must prove
the observable absence of a process descendant or committed file delta when the
decider is unavailable, malformed, late, denied, or replaced. Keep raw command,
patch, environment, and credential content out of diagnostics by default.

## Prohibited shortcuts

- Do not add a user-controlled policy executable path or a `PATH` lookup.
- Do not treat ordinary Codex hooks as equivalent to the dedicated decider.
- Do not broaden the tool table to make a test convenient.
- Do not copy credentials between homes or emit credential bytes in test output.
- Do not bypass the repository's own CI or release evidence.
