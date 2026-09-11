# Task: codex-fork — resolve OQ4 (remove code signing)

You are the **planner**. One contained change: the user has decided the fork
requires **no Authenticode code signing**. Rationale to record: this is a
self-forked, single-user, local-only tool consumed only by the user's own
harness; a code signature exists to prove provenance to third-party machines,
of which there are none. Promotion integrity now rests on the **SHA-256 pin +
green F31 re-audit + `provenance.json`** — the signature is dropped, the
verification is not.

Do NOT change anything else from the current plan (all cycle-2 review fixes stay
as they are). Architecture unchanged. Keep plan-lint-clean grammar and per-phase
count limits.

## Files you own

`master.md` and `repo-context/**` (`RELEASE.md`, and any doc that mentions
signing). Do not edit `child-runtime-native/`.

## Edits

1. **OQ4** — change from an open question to a **resolved decision**: no code
   signing; promotion is SHA-256-pinned. State the rationale above. Remove any
   "F33 does not publish until a signer is named" language.

2. **F33 (promotion)** — remove the signing step and the "sign before hash"
   ordering (that ordering only existed because signing rewrites the bytes).
   New sequence: write F11–F14 hunk SHAs into `COVENANT_PATCHES.md`; commit;
   re-run F31 on that immutable commit; on green, build with the F02 toolchain
   on `windows-2022`; hash the built exe; run `codex --covenant-inventory`
   against that exe; assemble `provenance.json`; publish the Release. Release
   assets: `codex-x86_64-pc-windows-msvc.exe`, `codex.exe.sha256`,
   `covenant-inventory.json`, `provenance.json`, `COVENANT_PATCHES.md`.
   `provenance.json` records source commit, toolchain, runner, exe digest,
   inventory digest, and re-audit run id — **no signer field**. Rewrite the F33
   success criteria: drop the "unsigned exe absent / promote job fails closed if
   signer unset" criterion; the digest in `codex.exe.sha256` and the
   `covenant-inventory.json` `binary_digest` both equal the built exe's hash
   (no "post-sign").

3. **F32 (`CODEX_PIN`)** — keep the four fields; `sha256` is the built exe's
   hash from the F33 Release `codex.exe.sha256` (remove any "post-sign" wording).
   No other change.

4. **Constraints / Invariants** — remove "an unsigned `codex.exe` is never
   promotable" and "signing precedes the digest and inventory F32 pins".
   Replace with: "Promotion requires a green F31 re-audit, the SHA-256 pin, and
   `provenance.json`; the fork does not code-sign the binary (single-user local
   tool)."

5. **Scope Out** — remove the "Promoting an unsigned `codex.exe`" line (it is no
   longer a prohibited state).

6. **Success Criteria (top-level list)** — restate the F33/F32 item: F33
   publishes a GitHub Release whose SHA-256 the harness pins; F32
   `CODEX_PIN.source = fork` points at that digest. Drop "signed".

7. **`repo-context/docs/RELEASE.md`** — remove the "Signing and provenance"
   signing content; keep provenance (source, toolchain, runner, digest,
   inventory, re-audit run id) with no signer field; drop `<signer>` and any
   independent-rebuild-as-signature-substitute language. State plainly: no code
   signing; SHA-256 + re-audit + provenance are the integrity basis. Ensure any
   other repo-context doc (CLAUDE.md/AGENTS.md/README-COVENANT.md/CONTRIBUTING.md)
   that references signing is scrubbed consistently.

8. Sweep for stragglers: search all owned files for `sign`, `Authenticode`,
   `signer`, `signature`, `unsigned` and reconcile each to the no-signing policy
   (a mention that "we intentionally do not sign" is fine; a promotion gate on
   signing is not).

## Deliverable

Revise in place. Return a list of every location changed and confirm no
signing-gate language remains. Do not self-certify; the orchestrator re-runs
review-plan.
