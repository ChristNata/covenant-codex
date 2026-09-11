# G4 decide contract — implementation reference (for downstream routes)

What Phase G4 (`child-runtime-native`, branch `covenant/01a07451/child-runtime-native`,
checkpoint V2.4.3) actually implemented, so other routes (codex-fork, covenant-worker,
broker/writer phases) can build on it. Status: **PASS-with-issues**, 217 cli tests
green. Two accepted residuals (see §Boundaries).

## Surface delivered

`covenant-cli hook decide` gains three new `hook_event_name` values —
**`Exec`**, **`Patch`**, **`Read`** — beside the existing six (PreToolUse,
UserPromptSubmit, PostToolUse, Stop, SessionStart, ConfigChange). All settle one
raw `Decision` JSON (`{decision: ALLOW|DENY, reason?, remediation?}`) and exit 0.
Every Codex-side `DENY` carries a non-empty `remediation`.

Files: `backend/covenant-cli/src/hooks/decision.rs` (evaluation),
`backend/covenant-cli/schema/decide-v1.json` (harness copy of the F10 schema).

## Envelope

```text
{ hook_event_name: "Exec" | "Patch",
  cwd, command?, path?,        // command/path are DISPLAY-ONLY
  decide_v1: DecideV1 }        // the authoritative object

{ hook_event_name: "Read", cwd, path }   // credential-path deny
```

`command`/`path` on the envelope are never the policy input for Exec/Patch — the
typed `decide_v1` object is. Missing / malformed / unknown-version `decide_v1`
=> `DENY` + remediation.

## `decide_v1` schema (v1)

Full schema: `schema/decide-v1.json` (`$id` .../codex-fork/schema/decide-v1.json;
**byte-identical to the codex-fork owner copy** — change both together). Rust
mirror: `DecideV1`/`ExecCall`/`PatchCall` in `decision.rs:601-640`. Shape:

- `DecideV1 { version:1, kind:"exec"|"patch", exec?|patch? }` (exactly one arm).
- `ExecCall { program, argv[], cwd, env{}, sandbox, network, tty }`. `env` is the
  **complete post-overlay post-scrub** process environment (the exact map the
  child sees), NOT an allowlist subset.
- `PatchCall { paths[], operations[], permissions{sandbox,write_roots[],network,
  tty}, resolved_identities[] }`. `operations[] = {path, op:add|update|delete|move,
  destination?, hunks[], pre_image_digest?}` (move needs destination;
  update/delete/move need pre_image_digest). `resolved_identities[] = {path, kind:
  file|directory|symlink|junction, target?, ancestor_identities[], win32_normalized,
  volume_serial, file_index, pre_image_digest?}`.

## evaluate() behavior (what a consumer can rely on)

- **Exec (`evaluate_decide_exec`, decision.rs:2490):** applies the FULL structured
  command policy (`structured_exec_reason`, :2638) to `program`+`argv`+`cwd`+the
  complete `env` — NOT a re-stringified shell command. Denied classes: marked-child
  orchestrator verbs (all managed `git`, routing promote/revoke, dispatch-run,
  gate, push, merge, worktree, state-advance, CI/CD; case-insensitive; `gh`
  global-option forms), image-wide process kills incl. name AND `/FI` filter
  forms (`direct_image_wide_kill`, :2784), direct child starts
  (`opencode`/`cursor-agent`/`agent`), curated runners, restricted cargo, and the
  closed provider/login secret set present in `env`. Shell/interpreter tails
  (`cmd`/PowerShell incl. `-EncodedCommand`, and code interpreters
  `node -e`/`python -c`/`ruby|perl -e`) are evaluated through their capability
  grammar with the strict flag preserved through recursion; **any interpreter form
  the evaluator cannot precisely reduce fails closed (DENY).**
- **Patch (`evaluate_decide_patch`, :2517):** currently **fail-closed** — every
  Patch is DENIED (`trusted_patch_produces`, :3003, always errors). Live
  patch-ALLOW is deferred (§Boundaries). Non-following-handle identity code
  (`validate_ancestor_chain`, :3148) exists but is not reachable-to-ALLOW yet.
- **Read (`evaluate_decide_read`, :2553):** `path` resolved to an absolute path
  and compared by opened non-following-handle filesystem identity against the
  stable-home auth files (`stable_home_auth_path`, :3256): `$GROK_HOME/auth.json`,
  `$CODEX_AUTH_HOME/auth.json` (fork arm), `$CODEX_HOME/auth.json` (official arm).
  Relative / trailing-dot-or-space / 8.3 / symlink-junction aliases to those
  objects all DENY. This is the closed credential-read guard.

## How to invoke (broker / fork / writer consumers)

`covenant-cli hook decide --client codex` with the envelope on stdin. The G5 fanin
mutation broker (`fanin_broker.rs`) is the reference consumer: it freezes an
immutable `ResolvedCall`, submits it to `hook decide`, and performs the effect
only on exact `ALLOW` of that same object. Exec: resolve the executable to an
absolute path BEFORE decide and start exactly that (no re-resolution).

## Boundaries — accepted residuals (do NOT assume these are done)

Recorded in `issue-g4-patch-authority.md`; owner = production/broker phases
**G5/C5/P5 after P4** + a plan revision:

1. **Live patch-ALLOW + full write/exec object-digest DECISION binding.** G4 does
   not consume the top-level `resolved_call`/digest for write-scope; write-scope
   is path-scoped and Patch is fail-closed. A trusted, launcher-bound authority
   packet (child/session identity + canonical worktree + phase/plan identity +
   exact Produces + freshness) is required before Patch/write can be ALLOWed with
   full object binding. Do not rely on G4 to bind the content digest.
2. **Structured-Exec interpreter-argv permutation tail** — obscure interpreter
   argv permutations beyond the closed classes; accepted per "strict enough" on a
   not-yet-live (writers promote at P5), defense-in-depth path. Future fix: a
   canonical-argv normalizer + property/fuzz test (fail-closed on anything it
   can't reduce).

Everything else in §evaluate() is enforced now.
