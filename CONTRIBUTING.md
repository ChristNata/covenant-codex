# Contributing to the Covenant Codex fork

Adapted from the [canonical harness context](https://github.com/ChristNata/Covenant-Harness/blob/b6e933a4590a2ef848755c4a593a7e9e8f2072d4/docs/master-plans/cross/codex-fork/repo-context/CONTRIBUTING.md) at
commit `b6e933a4590a2ef848755c4a593a7e9e8f2072d4`. This copy incorporates the
[approved fork decisions](covenant/IMPLEMENTATION.md); it is not byte-identical to the source.

The patch index is a draft during implementation. Follow the current ledger's
accepted stages and outstanding gates; component green does not authorize
promotion. Preserve existing upstream and nested agent instructions.

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

Prove constrained `codex exec` remains the only model task interface, with only
the separately reviewed native login/inventory support commands. Update
inventory evidence.

## U5 — schema

Coordinate the `decide_v1` version with the harness before implementation.

Record fork decisions in the implementation ledger and patch index. Coordinate
harness contract changes before release; edits to another checkout and remote
operations require their own authorized scope. This fork must not silently
become a general Codex customization branch.

## Branch and release topology

This repository is `ChristNata/covenant-codex`. `main` is the clean upstream
mirror, `covenant-ver` is the long-lived fork branch, and `upgrade/*` branches
carry proposed upstream integrations. Never commit Covenant behavior to `main`.
The current recorded baseline is `rust-v0.153.4`; see
[UPSTREAM.toml](covenant/UPSTREAM.toml) for its exact commit.

Follow [FORK_INTEGRATION.md](FORK_INTEGRATION.md) for the actual upgrade
procedure. An upgrade starts from the selected upstream baseline and merges
the existing Covenant branch into the temporary upgrade branch. Re-audit before
promotion. This implementation task does not authorize an upstream upgrade or
an intermediate push. Releases must identify an immutable, audited fork tree.

## Daily upstream-tracking routine

The user-owned [sync-stable workflow](.github/workflows/sync-stable.yml) already
has manual and daily triggers. It tracks upstream stable releases using the
actual `main` and `covenant-ver` branch roles. Its source and existing
[integration procedure](FORK_INTEGRATION.md) govern the upgrade flow.

Preserve that workflow. Do not dispatch it, update upstream, or push branches
as an incidental part of a fork feature task. Review semantic conflicts and
run the required fork checks on an explicitly authorized upgrade; automation
is not evidence that the resulting product passed re-audit.

## Re-audit locally

F31's final semantic re-audit is not implemented yet. Once available, use its
documented fork-only targets against the frozen candidate tree. It must:

1. compare this tree with the recorded upstream tag and commit;
2. confirm the F11 admission, F12 process-start, F13 patch, and F14 auth seams;
3. run the exact-ALLOW and zero-effect failure matrices;
4. inventory registered identities and their effect classes;
5. confirm excluded product surfaces remain absent from the artifact; and
6. produce the candidate hunk digests for `COVENANT_PATCHES.md`.

Current validation is scoped to newly added fork behavior, using `just test`
for Rust and the reviewed Python test selectors. Do not run the upstream suite
or repair unrelated baseline code. The ledger records the scoped-formatting and
isolated-lock amendments. Local success is evidence, not promotion; the final
CI re-audit is required before a release may be consumed by Covenant.

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
