# Covenant fork residuals

The fork narrows authority; it does not turn a coding agent into
a sandbox. Each accepted residual has a compensating control
owned by Covenant Harness.

| Residual | Control |
| --- | --- |
| Shell-mediated auth-store reads | F1.4 |
| Ordinary hooks compiled out / non-authoritative | F1.2/F1.3 |
| Durable vendor data | H1 |
| Sandbox is not complete containment | H2 |
| TOCTOU / reparse race after F13 resolution | H3 |
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

## F1.2/F1.3 — ordinary hooks compiled out / non-authoritative

Ordinary Codex command/MCP hooks are compiled out of the
promoted artifact and are not an enforcement boundary. Stock
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
the proof.

## H3 — TOCTOU / reparse race after F13 resolution

F13 resolves ancestor, link, and junction identities before
authorization, including `\\?\` normalization, Windows file
identity (`volume_serial`, `file_index`), and pre-image content
digest. The remaining filesystem residual is the TOCTOU /
reparse race **after** that resolution, not "identity is
delegated to the harness." Same-path same-kind replacement
between freeze and write is denied. Re-audit fixtures cover
symlink, junction, move, and replacement cases before a new
artifact is promoted.

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
and digest; the fork checks identity before each use, caps the
call at 1000 ms, and denies with zero intended effect on
failure.

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
