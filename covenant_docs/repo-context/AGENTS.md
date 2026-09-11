This is Covenant's minimal fork of Codex; it exists to make four things fail closed and nothing else.

# Covenant fork instructions

## What Covenant Harness is

Covenant Harness is a Windows desktop application that coordinates CLI coding
agents. An orchestrator starts bounded, one-shot child sessions through its
managed dispatch wrapper. A child completes one assigned task and exits; it
does not certify its own work or become a long-lived service.

This fork is one managed child binary. Covenant supplies its executable, launch
environment, role instructions, and policy decider. The fork supplies a narrow
`codex exec` surface and asks the decider before its two mutating authorities.
The policy response is an input to execution, not a report the child may judge.

## Why this fork exists

Stock Codex hooks can fail open, and its broader product surface contains
authorities outside the two final execution points. Stock `CODEX_HOME` also
combines login material with mutable per-attempt data. Covenant needs a small,
re-auditable binary that removes these residuals without rewriting Codex's
reasoning, turn, or agent loop.

The supported artifact is Windows x64 constrained `codex exec` and optional
`codex exec --json`. It is not an interactive Codex distribution.

## Adoption sequence

`CODEX_PIN.source` default is `fork` once a Release exists.
`official` is the documented fallback: initial testing, and when a
rebase is mid-flight or broken. This is a harness-side pin setting,
not a mode inside the binary. The repo is public; the pull path is
token-free.

## Branch and release topology

Upstream `openai/codex` feeds fork `main`, which is a clean adopted
upstream stable tag with no Covenant behavioral patches. The
`covenant` branch is `main` plus F11/F12/F13/F14 and F21/F22. Never
merge `covenant` into `main`. Releases build only from the audited
`covenant` branch/tag. Adopting a new upstream tag: update clean
`main`; merge/rebase `main` into `covenant`; resolve conflicts;
re-audit; rebuild; publish only when green.

## The four patches

- **F11 — `codex-rs/core/src/tools/registry.rs`,
  `ToolRegistry::dispatch_any_with_terminal_outcome`.** Admits only compiled
  canonical tool identities before a handler runs. It never decides command or
  patch semantics.
- **F12 — `codex-rs/core/src/unified_exec/process_manager.rs`,
  `UnifiedExecProcessManager::open_session_with_prepared_exec_env`.** Decides a
  final frozen exec object before either process-start branch. It never reparses
  model text or uses stock hooks as authority.
- **F13 — `codex-rs/core/src/tools/runtimes/apply_patch.rs`,
  `ApplyPatchRuntime::run`.** Decides resolved patch paths and structured hunks
  before the patch engine writes. It never permits a partial delta after denial.
- **F14 — `codex-rs/core/src/auth.rs`, auth load, refresh, and write.** Uses
  `CODEX_AUTH_HOME` for login material when supplied. It never exposes credential
  bytes to Covenant or copies them into `CODEX_HOME`.

## Never touch

- Do not rewrite the agent loop, reasoning loop, turn handling, or model client.
- Do not add an executor, tool identity, policy override, or runtime allowlist
  extension without a new audit and an updated patch index.
- Do not re-enable interactive terminals, `write_stdin`, TUI, app-server, MCP,
  plugins, Code Mode, hosted web, dynamic tools, or multi-agent features.
- Do not use stock `PreToolUse` or ordinary hooks as an enforcement boundary.
- Do not change files outside `COVENANT_PATCHES.md`'s patch index without an
  explicit upstream or Covenant decision.

## Runtime contract with Covenant

The managed launcher supplies these variables only for a fork-selected child:

| Variable | Meaning |
| --- | --- |
| `COVENANT_DECIDER_PATH` | Absolute launcher-owned path to the policy executable. |
| `COVENANT_DECIDER_SHA256` | Expected SHA-256 of that executable. |
| `COVENANT_CHILD_MARKER` | Opaque, presence-checked marker binding this process to a managed child launch. Capability upgrade (nonce + WorkerContract) is a follow-up, not this cycle. |

The fork canonicalizes the decider path and verifies its digest immediately
before every policy process creation. It ignores `COVENANT_CLI`, never searches
`PATH`, and does not let model input, project configuration, or environment
overrides replace the decider.

For each exec or patch authority, execute the absolute path in
`COVENANT_DECIDER_PATH` with arguments `hook`, `decide`, `--client`, and `codex`.

Write one JSON event envelope to stdin, close stdin, and enforce a 1000 ms
deadline. The envelope has `hook_event_name` of `Exec` or `Patch`, `cwd`, an
optional display-only `command` or `path`, and a frozen `decide_v1` object.
See `docs/DECIDE_V1.md` for the exact schema.

Execution may proceed only when stdout is exactly:

```json
{"decision":"ALLOW"}
```

Any other byte sequence, extra key, malformed object, absent executable,
identity mismatch, timeout, cancellation, crash, disconnect, or non-zero exit
is a non-retriable `CovenantDenied` with zero intended effect. `ALLOW_WITH_CONTEXT`
is denial. Kill and reap the policy child before returning from a failed call.

## Rebase onto upstream

1. Select the next audited `rust-v*` source tag and record its tag and commit.
2. Re-apply F11 through F14, then the constrained packaging defaults and
   inventory command. Do not broaden the artifact to resolve a conflict.
3. Update each patch-index row with its exact seam, diff, and re-audit result.
4. Run the fork CI, including the re-audit job and zero-effect failure matrix.
5. Compare the inventory JSON and its digest with the prior promoted artifact.
6. Publish only a tree whose re-audit is green; otherwise retain the prior pin.

Expected breakage is renamed call sites, new tool identities, and feature-flag
churn. A missing or moved seam is a re-audit failure, not permission to choose a
nearby insertion point.

## Sharp edges

BAD: Treat a successful process spawn of the decider as authorization.

GOOD: Accept only the one exact ALLOW object; every other outcome returns
`CovenantDenied` before the irreversible operation.

BAD: Add a config or environment allowlist so a project can enable one more tool.

GOOD: Keep the admission table compiled into the binary. An unreviewed identity
is denied before its handler runs.

BAD: Use `PreToolUse` because it already observes tool calls.

GOOD: Decide after the final exec request or resolved patch request exists and
before process creation or patch application.

BAD: Put credentials in a disposable `CODEX_HOME` copy for a child attempt.

GOOD: Let Codex read and refresh its own login material under `CODEX_AUTH_HOME`.

## Read next

| File | Purpose |
| --- | --- |
| `README-COVENANT.md` | Human overview, license, and document map. |
| `COVENANT_PATCHES.md` | Maintained patch inventory and test commitments. |
| `docs/DECIDE_V1.md` | Exec and patch decider wire contract. |
| `docs/RELEASE.md` | Build, provenance, and harness-consumption rules. |
| `docs/RESIDUALS.md` | Accepted risks and Covenant compensating controls. |
| `CONTRIBUTING.md` | Upstream-first contribution and re-audit policy. |
