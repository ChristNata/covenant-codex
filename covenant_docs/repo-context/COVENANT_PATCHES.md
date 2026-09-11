# Covenant patch index

This index is the complete maintained Covenant patch surface. A source change
outside these rows is either upstream work or a new audited proposal.

| ID | File and symbol | Purpose | Failure | Tests | Rebase |
| --- | --- | --- | --- | --- | --- |
| F11 | `registry.rs`: dispatch | admission | deny unknown | unit + audit | enumerate |
| F12 | `process_manager.rs`: open | final exec | deny else | matrix + audit | pre-start |
| F13 | `apply_patch.rs`: run | final patch | zero delta | matrix + audit | pre-write |
| F14 | `auth.rs`: auth I/O | auth home | deny leak | probe + audit | upstream |
| F21 | Cargo.toml / features / cli / config / spec_plan / model-catalog | constrained packaging/profile | excluded surface reachable | hostile-override + audit | flags/profile |
| F22 | `covenant_inventory.rs` + cli main | inventory/certificate | unknown identity | inventory JSON + audit | C7 fields |

## F11 — tool identity admission

**File and symbol:** `codex-rs/core/src/tools/registry.rs`,
`ToolRegistry::dispatch_any_with_terminal_outcome`; helper:
`codex-rs/core/src/covenant_admission.rs`.

**Purpose and failure:** The compiled canonical identity-to-effect table admits
only approved identities. Unknown, namespaced, malformed, dynamic, extension,
MCP, hosted, agent, permission, and `write_stdin` identities return terminal
non-retriable `CovenantDenied` before `pre_tool_use` and handler dispatch.

**Tests and rebase:** The registry unit matrix proves an unknown identity calls
no handler; `exec_command` and custom `apply_patch` reach downstream runtime;
writable config, environment, and allowlist fixtures cannot admit an identity.
Re-audit asserts admission precedes `handle_any_tool`. Re-enumerate every
registered tool; a new process, filesystem, network, hosted, or MCP identity
remains denied until separately audited.

## F12 — exec decision

**File and symbol:** `codex-rs/core/src/unified_exec/process_manager.rs`,
`UnifiedExecProcessManager::open_session_with_prepared_exec_env`; helper:
`codex-rs/core/src/covenant_gate.rs`.

**Purpose and failure:** Decide the final exec request at the shared local and
remote process boundary. A missing or replaced decider, timeout, cancellation,
malformed output, extra keys, non-zero exit, `DENY`, or `ALLOW_WITH_CONTEXT`
returns `CovenantDenied` before `backend.start` or
`codex_sandboxing::spawn_process`.

**Tests and rebase:** The table-driven failure matrix asserts zero command
descendants and no surviving policy child. Real-sidecar integration proves the
frozen exec envelope permits or denies the matching start. Re-audit asserts the
call precedes both start branches. Reconfirm the final `ExecRequest` and
`ToolCtx`; any moved call remains before every start branch.

## F13 — patch decision

**File and symbol:** `codex-rs/core/src/tools/runtimes/apply_patch.rs`,
`ApplyPatchRuntime::run`.

**Purpose and failure:** Decide a resolved structured patch before the patch
engine mutates files. Every non-exact ALLOW produces `CovenantDenied`, no
patch-engine call, and zero committed byte delta.

**Tests and rebase:** Add, update, delete, and move tests cover absent decider,
timeout, malformed output, explicit denial, and real-sidecar allow/deny.
Re-audit asserts the call precedes `apply_patch_with_options`. Preserve resolved
`PathUri` values, operation kind, hunks, and permissions; re-audit ancestor and
symlink handling when upstream path resolution changes.

## F14 — auth-path indirection

**File and symbol:** `codex-rs/core/src/auth.rs`, auth load, refresh, and write;
helper: `codex-rs/core/src/covenant_auth_home.rs`.

**Purpose and failure:** Honor `CODEX_AUTH_HOME` as the login store under
either harness `CODEX_HOME` model. With `CODEX_AUTH_HOME`, all auth reads,
refreshes, and writes use only that directory; failed refresh leaves prior
credentials intact and fork sinks retain no sentinel.

**Tests and rebase:** The auth-path probe covers both-home separation and
upstream fallback. After writing auth, deleting `CODEX_HOME` still lets the
auth probe succeed from `CODEX_AUTH_HOME`. Refresh and five-concurrent-probe
tests preserve one well-formed auth file. Sentinel tests cover fork logs,
errors, mutable home, and retained output. Re-audit asserts the resolver
precedes every auth read or write.
If upstream supplies equivalent support, drop this patch, adopt its variable and
tests, and do not create a credential-copying compatibility path.

## F21 — constrained packaging / profile

**File and symbol:** `codex-rs/Cargo.toml`,
`codex-rs/features/src/lib.rs`, `codex-rs/cli/src/main.rs`,
`codex-rs/core/src/config/mod.rs`,
`codex-rs/core/src/tools/spec_plan.rs`,
`covenant/model-catalog.json`.

**Purpose and failure:** Smallest upstream diff that removes
excluded product surfaces: Covenant compile/build profile or
feature flags, immutable managed clamps, and F11 compiled
default-deny. Do not rewrite MCP/hook implementation files
unless tests prove they remain reachable or perform startup
side effects. A hostile override that re-enables an excluded
surface is a packaging failure.

**Tests and rebase:** Hostile-override matrix plus fixture
hook/MCP servers that must produce no process and no request.
Re-audit fails if app-server, TUI default, MCP management, or
`write_stdin` returns on the published exe.

## F22 — inventory / certificate

**File and symbol:** `codex-rs/cli/src/covenant_inventory.rs`,
wired from `codex-rs/cli/src/main.rs`.

**Purpose and failure:** `codex --covenant-inventory` emits the
C7 certificate contract. An unclassified registered identity
makes inventory fail and prevents `codex exec` from starting.

**Tests and rebase:** JSON field names and digest inputs match
the C7 certificate. `evidence.tool_schema` identities equal the
F11 const table. Re-audit diffs inventory against that table.

## Packaging invariants

The index relies on constrained packaging, not four more policy
calls. The published binary must omit or disable app-server,
TUI, MCP, plugins, Code Mode, hosted web, dynamic tools,
multi-agent operation, `write_stdin`, and ordinary command/MCP
hooks as an enforcement/execution surface. The pinned model
catalog and managed requirements fail closed if absent or
invalid.

`codex --covenant-inventory` emits the binary digest, profile
digest, inventory identifier, optional schema residual, and
effective tool schema, instruction sources, skill sources, and
feature values. An unclassified registered identity makes
inventory fail and prevents `codex exec` from starting.

## Re-audit completion rule

For every upstream rebase, record each row's hunk digest after
the re-audit CI passes. The job must prove that the four
insertions remain present, F21 packaging and F22 inventory still
hold, every model-originated process, filesystem, network, MCP,
hosted, and dynamic route is absent or dominated by F11 through
F13, and no admitted identity gained a new effect class.
