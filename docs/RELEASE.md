# Covenant fork releases

Adapted from the [canonical harness context](https://github.com/ChristNata/Covenant-Harness/blob/b6e933a4590a2ef848755c4a593a7e9e8f2072d4/docs/master-plans/cross/codex-fork/repo-context/docs/RELEASE.md) at
commit `b6e933a4590a2ef848755c4a593a7e9e8f2072d4`. This copy incorporates the
[approved fork decisions](../covenant/IMPLEMENTATION.md); it is not byte-identical to the source.

**Status:** Option-B E01 is green: exec is real-decider gated and Patch is
explicitly deny-only until the Harness trusted-authority packet lands. The
tagged workflow performs the pinned F02 Windows build, F22 inventory, F31
re-audit/test receipt, and F33 provenance/attestation before publication.
Harness adoption remains a separate F32 handoff.

A Covenant Codex release is a Windows x64 artifact with a recorded
binary digest, inventory evidence, and green re-audit. A release
without a green re-audit is never promotable. The fork does not
code-sign the binary; SHA-256 + re-audit + provenance + a
GitHub build-provenance attestation (verified before pinning)
are the integrity basis.

## Build inputs

| Input | Required value |
| --- | --- |
| Source baseline | Recorded `rust-v0.153.4` pin and an immutable audited fork tree; see [UPSTREAM.toml](../covenant/UPSTREAM.toml). |
| Runner | `windows-2022`. |
| Target | `x86_64-pc-windows-msvc`. |
| Toolchain | Rust 1.95.0 and exact rustc/Cargo identities recorded by [windows-repro.toml](../covenant/windows-repro.toml). |
| MSVC | Windows 2022 VS Build Tools. |
| Node | No product build step; pinned Actions use their own action runtime. |
| Artifact | `codex-x86_64-pc-windows-msvc.exe`. |

The build recipe is the repository's recorded Cargo invocation
for `codex.exe`. Run it in CI with the recorded toolchain, hash
the resulting executable with SHA-256, and write `codex.exe.sha256`
as one lowercase 64-hex line. [build_windows.py](../covenant/scripts/build_windows.py)
uses the locked release command and refuses invalid/missing/stale artifacts.
The manual inputless [artifact workflow](../.github/workflows/covenant-release.yml)
uploads only that executable and digest after the read-only pair verifier passes.
Its build-output receipt remains unattested development evidence, not a release
asset. Local syntax checks do not prove a hosted build or byte reproducibility.

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

## Required verification before promotion

1. Complete the product graph, locked dependencies, native gates and separately
   verified build prerequisites. Then build the Windows executable from recorded
   source and toolchain inputs on the audited `covenant-ver` branch/tag.
2. Run the constrained fork test suite, including exact-ALLOW exec and
   real-decider Patch DENY with zero bytes written. Patch ALLOW is deferred to
   the Harness trusted-authority packet under Option B.
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

The promote job emits a GitHub build-provenance attestation
(`actions/attest-build-provenance`) binding the exe digest to the
immutable workflow run, source commit, and tag. This is an
attestation, not a code signature — it needs no certificate and
proves the bytes came from this CI build on this commit. The
harness verifies it (`gh attestation verify`) before pinning; a
missing or failed attestation, or a digest not bound to the
F31-green build, refuses the pin. This closes the mutable-Release-
metadata gap that SHA-256-from-the-same-Release alone leaves open.

The first attested release is [`covenant-v0.1.4`](https://github.com/ChristNata/covenant-codex/releases/tag/covenant-v0.1.4).
Its executable URL is
[`codex-x86_64-pc-windows-msvc.exe`](https://github.com/ChristNata/covenant-codex/releases/download/covenant-v0.1.4/codex-x86_64-pc-windows-msvc.exe),
with SHA-256
`0e24376add35ffb3b72766632b98433e5cd4aca4d891d9b91ad725fb99e459f8`.
The attestation was verified with `gh attestation verify` against the
release workflow and `refs/tags/covenant-v0.1.4`.

## GitHub Release layout

Publish under `ChristNata/covenant-codex` only after the required
verification is green. Build only from the audited `covenant-ver`
branch/tag. Never merge `covenant-ver` into clean `main`.

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

The inspected local harness at `c78e2a25` has the four-field CodexPin and
Official/Fork source type, plus `--client codex`; its active pin is Official
0.153.4. This source observation is not a fresh remote artifact verification.
Real G4 decide_v1/complete-env/identity semantics, the home adapter and C7
inventory/attestation integration remain missing.

The final harness consumption contract uses exactly four fields:

| Pin field | Release source |
| --- | --- |
| `source` | `fork` only after audited release and harness acceptance; `official` remains the fallback |
| `version` | Fork tag without `v` |
| `url` | GitHub Release URL for `codex-x86_64-pc-windows-msvc.exe` under `ChristNata/covenant-codex` |
| `sha256` | Exact contents of `codex.exe.sha256` |

`source_commit` and `inventory_digest` are **not** pin fields.
They live in Release `provenance.json` and inventory evidence.

The harness installs only the named release asset whose SHA-256
matches the row. Any source, digest, inventory, or provenance
mismatch rejects promotion and requires recertification before
managed use. The repo is public; the pull path is token-free.
