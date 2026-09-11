# Covenant fork releases

A Covenant Codex release is a Windows x64 artifact with a recorded
binary digest, inventory evidence, and green re-audit. A release
without a green re-audit is never promotable. The fork does not
code-sign the binary; SHA-256 + re-audit + provenance + a
GitHub build-provenance attestation (verified before pinning)
are the integrity basis.

## Build inputs

| Input | Required value |
| --- | --- |
| Source baseline | Recorded audited `rust-v*` tag and commit on the `covenant` branch. |
| Runner | `windows-2022`. |
| Target | `x86_64-pc-windows-msvc`. |
| Toolchain | Exact Rust toolchain named by the repository toolchain file. |
| MSVC | Windows 2022 VS Build Tools. |
| Node | Not used. |
| Artifact | `codex-x86_64-pc-windows-msvc.exe`. |

The build recipe is the repository's recorded Cargo invocation
for `codex.exe`. Run it in CI with the recorded toolchain, hash
the resulting executable with SHA-256, and write `codex.exe.sha256`
as one lowercase 64-hex line.

## Identity policy

One coherent policy, in this order:

1. Reproducibility is desirable and tested where possible.
2. SHA-256 identifies the promoted CI artifact.
3. Provenance records source tag/commit, toolchain, and runner.
4. The fork does not Authenticode-sign. A code signature exists
   to prove provenance to third-party machines, of which there
   are none: this is a self-forked, single-user, local-only tool
   consumed only by the user's own harness. Promotion integrity
   is the SHA-256 pin, a green F31 re-audit, and
   `provenance.json`.

Prefer byte identity across independent builds. If MSVC output
prevents it, set the recorded digest from the green CI build as
the artifact identity. That digest is the promotion pin F33
captures after a green re-audit.

## Required verification

1. Build the Windows executable from the recorded source and
   toolchain inputs on the audited `covenant` branch/tag.
2. Run the constrained fork test suite, including exact-ALLOW
   and zero-effect matrices for exec and patch authority.
3. Run the re-audit against the recorded upstream commit. It
   must verify all four patch insertions, constrained packaging,
   inventory equivalence, admitted tool effects, and
   model-originated semantic sinks.
4. Hash the built exe. Run `codex --covenant-inventory` against
   that exe and record its `inventory_id` and evidence digests.
5. Record provenance.

## Provenance

The fork does not code-sign. SHA-256 + re-audit + provenance +
build-provenance attestation are the integrity basis. The release
provenance records source tag and commit, toolchain identity,
runner image, executable digest, inventory digest, and re-audit
run identity. No signer field. Those fields live in
`provenance.json` and inventory evidence, not in the harness pin.

The promote job also emits a GitHub build-provenance attestation
(`actions/attest-build-provenance`) binding the exe digest to the
immutable workflow run, source commit, and tag. This is an
attestation, not a code signature — it needs no certificate and
proves the bytes came from this CI build on this commit. The
harness verifies it (`gh attestation verify`) before pinning; a
missing or failed attestation, or a digest not bound to the
F31-green build, refuses the pin. This closes the mutable-Release-
metadata gap that SHA-256-from-the-same-Release alone leaves open.

## GitHub Release layout

Publish under `covenant-capital/codex` only after the required
verification is green. Build only from the audited `covenant`
branch/tag. Never merge `covenant` into clean `main`.

| Asset | Content |
| --- | --- |
| `codex-x86_64-pc-windows-msvc.exe` | Promoted Windows executable. |
| `codex.exe.sha256` | One 64-hex SHA-256 line for the built executable. |
| `covenant-inventory.json` | Output from `codex --covenant-inventory`. |
| `provenance.json` | Source, toolchain, CI, digest, inventory, re-audit run id. |
| `COVENANT_PATCHES.md` | Patch rows and re-audit hunk digests for this tree. |

Do not use ephemeral CI artifact URLs as release inputs. Do not
publish a promotable release for a tree that has not passed
re-audit.

## Harness consumption

The harness consumes one `CODEX_PIN` row with exactly four
fields:

| Pin field | Release source |
| --- | --- |
| `source` | `fork` once a Release exists; `official` is the documented fallback |
| `version` | Fork tag without `v` |
| `url` | GitHub Release URL for `codex-x86_64-pc-windows-msvc.exe` under `covenant-capital/codex` |
| `sha256` | Exact contents of `codex.exe.sha256` |

`source_commit` and `inventory_digest` are **not** pin fields.
They live in Release `provenance.json` and inventory evidence.

The harness installs only the named release asset whose SHA-256
matches the row. Any source, digest, inventory, or provenance
mismatch rejects promotion and requires recertification before
managed use. The repo is public; the pull path is token-free.
