# Covenant Codex fork

Adapted from the [canonical harness context](https://github.com/ChristNata/Covenant-Harness/blob/b6e933a4590a2ef848755c4a593a7e9e8f2072d4/docs/master-plans/cross/codex-fork/repo-context/README-COVENANT.md) at
commit `b6e933a4590a2ef848755c4a593a7e9e8f2072d4`. This copy incorporates the
[approved fork decisions](covenant/IMPLEMENTATION.md); it is not byte-identical to the source.

**Status:** implementation in progress. Component tests and local build-helper
checks are available; no constrained product, full native gate, runtime
certificate, semantic re-audit, fork release or harness adoption is certified.
See the [implementation ledger](covenant/IMPLEMENTATION.md) for accepted stages.

This fork keeps Codex upstream-first. Its target managed Windows `codex exec`
artifact must fail closed at tool admission, final process start, final patch write,
and credential-home selection.

It is for Covenant Harness, a Windows application that coordinates one-shot CLI
children. The fork is not a general-purpose hardened Codex distribution and does
not expand Codex's supported product surface.

## Scope

The target artifact supports constrained `codex exec` and optional JSON output.
It excludes interactive and independent authorities: TUI, public app-server, MCP,
plugins, Code Mode, hosted web, dynamic tools, multi-agent features, and
interactive process input. The internal app-server orchestration required by
upstream exec remains; this is not an entirely removed library.

The integrated fork must ask Covenant's managed decider before every admitted
process start or patch write. Only the exact response `{"decision":"ALLOW"}` authorizes the
already-resolved operation.

## Adoption sequence

`CODEX_PIN.source` becomes `fork` only after an audited Release exists and
the harness verifies its digest, provenance, attestation and required integration
tests. `official` is the fallback for initial testing or a broken rebase; it is
a harness pin choice, not an unconstrained mode inside the published binary.
The local harness source inspected at `c78e2a25` still selects Official 0.153.4.
No fork release or adoption is certified by the current implementation work.
The repository is public; the intended release download path is token-free.

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

The existing [sync-stable workflow](.github/workflows/sync-stable.yml) has
manual and daily triggers. Its source is preserved; this documentation does
not authorize dispatch or an upstream update. See CONTRIBUTING.md for review
and release boundaries.

## Documents

| File | Read when |
| --- | --- |
| `CLAUDE.md` | Working with Claude Code in this repository. |
| `AGENTS.md` | Working with Codex or another agent that reads native instructions. |
| `COVENANT_PATCHES.md` | Auditing a maintained source change or an upstream rebase. |
| `docs/DECIDE_V1.md` | Implementing or testing the decider client and payload. |
| `docs/RELEASE.md` | Building a Windows artifact or preparing its provenance. |
| `docs/RESIDUALS.md` | Assessing accepted risks and their harness controls. |
| `CONTRIBUTING.md` | Proposing a fork change or an upstream contribution. |

## License and attribution

This fork remains Apache-2.0 under Codex's upstream license. Keep upstream
`LICENSE` and `NOTICE` bytes unmodified. Covenant attribution belongs only in
the fork's Covenant documentation and metadata; it does not claim ownership of
upstream Codex. Every rebase confirms the upstream licensing and notice posture
before an artifact is promotable.
