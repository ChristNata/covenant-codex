# Covenant Codex fork

This fork keeps Codex upstream-first while making the managed Windows `codex exec`
artifact fail closed at tool admission, final process start, final patch write,
and credential-home selection.

It is for Covenant Harness, a Windows application that coordinates one-shot CLI
children. The fork is not a general-purpose hardened Codex distribution and does
not expand Codex's supported product surface.

## Scope

The artifact supports constrained `codex exec` and optional JSON output. It
removes interactive and independent authority surfaces: TUI, app-server, MCP,
plugins, Code Mode, hosted web, dynamic tools, multi-agent features, and
interactive process input.

The fork asks Covenant's managed decider before every admitted process start or
patch write. Only the exact decider response `{"decision":"ALLOW"}` authorizes the
already-resolved operation.

## Adoption sequence

`CODEX_PIN.source` default is `fork` once a Release exists.
`official` is the documented fallback: initial testing, and when a
rebase is mid-flight or broken. This is a harness-side pin setting,
not a mode inside the binary. The repo is public; the pull path is
token-free.

## Branch and release topology

Upstream `openai/codex` feeds fork `main` (clean adopted upstream
stable tag, no Covenant behavioral patches). The `covenant` branch is
`main` plus F11/F12/F13/F14 and F21/F22. Never merge `covenant` into
`main`. Releases build only from the audited `covenant` branch/tag.

Daily upstream-tracking automation is documented in `CONTRIBUTING.md`.
It is informational — not implemented in this repository's workflows;
the maintainer sets it up manually.

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
