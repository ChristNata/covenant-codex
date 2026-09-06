This repository is developing Covenant's minimal Codex fork: four fail-closed
boundaries with constrained packaging and auditable release evidence.

# Covenant fork instructions

Adapted from the [canonical harness context](https://github.com/ChristNata/Covenant-Harness/blob/b6e933a4590a2ef848755c4a593a7e9e8f2072d4/docs/master-plans/cross/codex-fork/repo-context/CLAUDE.md) at
commit `b6e933a4590a2ef848755c4a593a7e9e8f2072d4`. This copy incorporates the
[approved fork decisions](covenant/IMPLEMENTATION.md); it is not byte-identical to the source.

**Implementation status:** the fork is unfinished. Local tests cover accepted
admission, auth, configuration and standalone runtime components. Final native
exec/patch integration, the constrained product, inventory, re-audit, release
and harness adoption are not certified. The runtime rules below are required
behavior, not a claim that every gate is deployed.

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

## The four patches

- **F11 — tool identity admission.** The compiled table in
  `codex-rs/tools/src/covenant_admission.rs` admits only unqualified function
  `exec_command` and custom `apply_patch`. For covered local tool calls, core
  router, parallel and registry guards reject other identities before callbacks
  or handlers. This classifies
  identity; it grants no process-start or patch permission.
- **F12 — final exec.** The process manager is the routing gate; authorization
  must dominate each final prepared local Windows backend. Standalone launch,
  environment, envelope, exact-reply and command-line components exist under
  `covenant/runtime/`; native creation, image/Job ownership and full backend
  integration remain pending. Remote/foreign and snapshot routes must refuse.
- **F13 — final patch.** `ApplyPatchRuntime::run` is the planned integration
  seam. Authorization must bind a structured, identity-guarded batch through
  mutation. Every settled denial leaves zero committed delta. This gate is
  unfinished; a preflight stat/hash check alone cannot satisfy it.
- **F14 — native auth.** `codex-rs/login/src/auth/` owns home routing, refresh
  serialization/cancellation ownership and file replacement. Keep credentials
  in Codex's native auth flow. Remaining route/sink and harness-home acceptance
  is tracked in the [draft patch index](COVENANT_PATCHES.md).

F21 must constrain packaging/configuration and F22 must supply runtime inventory.
Neither the six-row index nor a component test substitutes for final re-audit.

## Never touch

- Do not rewrite the agent loop, reasoning loop, turn handling, or model client.
- Do not add an executor, tool identity, policy override, or runtime allowlist
  extension without a new audit and an updated patch index.
- Do not re-enable interactive terminals, `write_stdin`, TUI entrypoints, public
  app-server entrypoints, MCP, plugins, Code Mode, hosted web, dynamic tools or
  multi-agent authorities in the published artifact. The internal app-server
  library required by upstream `codex exec` remains; clamp its alternate effects.
- Do not use stock `PreToolUse` or ordinary hooks as an enforcement boundary.
- Do not change files outside `COVENANT_PATCHES.md`'s patch index without an
  explicit upstream or Covenant decision.

## Required runtime contract with Covenant

The managed launcher supplies these variables only for a fork-selected child:

| Variable | Meaning |
| --- | --- |
| `COVENANT_DECIDER_PATH` | Absolute launcher-owned path to the policy executable. |
| `COVENANT_DECIDER_SHA256` | Expected SHA-256 of that executable. |
| `COVENANT_CHILD_MARKER` | Opaque, presence-checked marker binding this process to a managed child launch. Capability upgrade (nonce + WorkerContract) is a follow-up, not this cycle. |

Freeze launcher controls once. Before each policy creation, the native design
requires an opened-file digest, guarded immutable namespace and suspended-image
path/file identity checks for a local non-reparse installation. This is not a
claim of access to a kernel mapped-image handle. Ignore `COVENANT_CLI`, never
search `PATH`, and reject model/configuration replacement of launcher controls.

For each exec or patch authority, execute the absolute path in
`COVENANT_DECIDER_PATH` with arguments `hook`, `decide`, `--client`, and `codex`.

Write one bounded JSON event envelope to stdin and close stdin. Accept no
authorization after 1000 ms; complete owned cleanup before settlement, without
promising that cleanup itself finishes within exactly 1000 ms. The envelope has
`hook_event_name` of `Exec` or `Patch`, `cwd`, an
optional display-only `command` or `path`, and a frozen `decide_v1` object.
See `docs/DECIDE_V1.md` for the exact schema.

Execution may proceed only when stdout is exactly:

```json
{"decision":"ALLOW"}
```

Any other byte sequence, extra key, malformed object, absent executable,
identity mismatch, timeout, cancellation, crash, disconnect, or non-zero exit
must deny before model payload execution. `ALLOW_WITH_CONTEXT` is denial.
The Job design must kill/reap owned descendants before tool settlement;
brokered/service effects remain a policy residual. Fixed trusted sandbox
preparation may precede ALLOW and is inventoried separately. Preserve the chosen
sandbox and never fall back to unsandboxed execution. F13 cancellation after
commit admission must settle the actual result, not falsely report denial.

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
