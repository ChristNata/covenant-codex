# Covenant fork residuals

Adapted from the [canonical harness context](https://github.com/ChristNata/Covenant-Harness/blob/b6e933a4590a2ef848755c4a593a7e9e8f2072d4/docs/master-plans/cross/codex-fork/repo-context/docs/RESIDUALS.md) at
commit `b6e933a4590a2ef848755c4a593a7e9e8f2072d4`. This copy incorporates the
[approved fork decisions](../covenant/IMPLEMENTATION.md); it is not byte-identical to the source.

**Status:** these controls define required release posture. Their listing is
not evidence that final F12/F13/F21 or harness integration is complete. The
[ledger](../covenant/IMPLEMENTATION.md) distinguishes accepted local components
from outstanding native, product and external gates.

The fork narrows authority; it does not turn a coding agent into
a sandbox. Each accepted residual has a compensating control
owned by Covenant Harness.

| Residual | Control |
| --- | --- |
| Shell-mediated auth-store reads | F1.4 |
| Ordinary hook authority exclusion | F1.2/F1.3 |
| Durable vendor data | H1 |
| Sandbox is not complete containment | H2 |
| Filesystem identity and mutation races | H3 |
| New upstream authority | H4 |
| External decider dependency | H5 |
| Credential bytes remain in Codex | H6 |
| Human rebase maintenance | H7 |

## F1.4 — shell-mediated auth-store reads

A read-only direct-file policy does not make an admitted shell
command unable to inspect a readable login location. Direct file
tools deny the auth path; secret-canary probes exercise hostile
direct and shell reads; retained outputs, errors, tracing,
provider fixtures, and diagnostic material are scanned and
redacted. This remains an accepted residual while native shell
(`exec_command`) is admitted.

## F1.2/F1.3 — ordinary hook authority exclusion

Ordinary Codex command/MCP hook authorities must be excluded from the
promoted artifact and cannot be an enforcement boundary. Stock
hook parsing can fail open; that is not the Covenant boundary.
F11 admits identities, F12 and F13 decide at the final process
and patch sites. If a hook or MCP implementation remains
reachable despite being disabled, that is an F21 packaging
defect, not an accepted fail-open hook residual.

## H1 — durable vendor data

Native login needs a durable location, and not every vendor-owned
path is redirectable. `CODEX_AUTH_HOME` holds login material.
`CODEX_HOME` lifetime is harness-decided: per-attempt disposable
if fully redirectable with no downside, otherwise stable
`CODEX_HOME` plus redirected sub-dirs. Leak-matrix and
bounded-growth probes record remaining retention and clean
disposable roots at settlement.

## H2 — sandbox is not complete containment

A workspace-write child can legitimately alter files inside its
worktree, and shell tools have broad semantics. One-shot
execution, a dedicated worktree, Job Object containment,
declared file-scope settlement, and promotion evidence constrain
the child operationally. Background/detached descendants must
be reaped by Codex supervision or rejected by policy before the
exec tool settles; the outer Job Object at session end is not
the proof. The approved no-breakaway Job covers its descendants, not
broker/service/WMI effects, which remain policy residuals. Plain, legacy and
elevated local Windows backends each need separate acceptance. Preserve their
sandbox choices; do not fall back unsandboxed. Trusted fixed preparation may
precede ALLOW, but denied model payload execution must remain zero.

## H3 — TOCTOU / reparse race after F13 resolution

F13 must bind resolved ancestor/target identity and pre-image content through
the committed mutation. A one-time stat/hash before authorization is insufficient.
Same-path replacement, junction/reparse swaps and effective precommit races must
leave zero committed delta on settled denial. These requirements are not waived
as residuals, and their current native qualification/integration is unfinished.

The bounded design requires transaction capability qualification and refusal on
unsupported volumes/APIs, with no ordinary-writer fallback. Initial update-only
work cannot satisfy add/delete/move coverage. Cancellation before commit admission
rolls back; after owner-admitted commit, settle the actual result without a false
denial or retry of an unknown outcome. Only final independently reviewed native
and real-sidecar evidence can establish the supported mutation contract.

## H4 — new upstream authority

A small maintained fork can drift as Codex adds identities,
configuration layers, or product entry points. Compiled
default-deny admission (closed table; new identities need a
plan amendment), constrained packaging, inventory comparison,
and re-audit semantic-sink tests reject drift. The
`AUTHORITY-INVENTORY.toml` is commit-bound and re-audit fails
on a commit mismatch, but its *completeness* at a new tag is a
human audit, not a machine oracle. The accepted residual is an
upstream authority family omitted from the inventory; the
compensating controls are compiled default-deny (an omitted
identity is denied before its handler runs), the commit-bound
re-audit, and human rebase review (H7).

## H5 — external decider dependency

A missing, swapped, slow, or malformed policy executable can
interrupt a child task. The launcher supplies an absolute path
and digest; the fork must check identity before each use, accept no authorization
after 1000 ms, and own cleanup before settlement.
The deadline is not a hard-real-time cleanup guarantee. Opened-file hashing,
guarded namespace and suspended-image identity checks remain required native
integration; lexical control validation alone cannot prove them.

## H6 — credential bytes remain in Codex

Codex must read and refresh its own native login material.
Covenant does not copy or persist credential bytes. Its secret
canary verifies that retained sinks contain no injected
sentinel. Model-spawned exec environments do not receive
`CODEX_AUTH_HOME` or Covenant control variables.

## H7 — human rebase maintenance

A semantic upstream refactor may require judgment even when
text diffs are small. Rebase CI compares patch seams and
semantic sinks, records hunk digests, and rejects promotion
until a human resolves the audit result.

## Residual acceptance rule

Do not describe a residual as fixed because a unit test observes
a friendly path. It remains accepted only while its compensating
control is executable, documented, and included in
recertification evidence for the pinned artifact.
