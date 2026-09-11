# Review: codex-fork

**Date:** 2026-09-05
**Rigor:** hard
**Phase:** plan
**Reviewers:** alignment

## Verdict

PASS

The four post-adversarial inline fixes are present, mutually
consistent, and do not unwind the preserve-list or Section C.
F31/F32/F33 stay inside the per-phase SC cap of 8. Live CRN
remains the stale sibling; that is a flagged precondition, not
a this-plan defect. Two contained plan-contract nits do not
block: F00 never names the inventory `commit` field F31 SC8
compares, and F33 does not restate F03's 40-hex `uses:` pin on
the attest action it adds to the same workflow file.

## Checks

| Check | Verdict |
|---|---|
| 1. Discriminating G4 tests | PASS — F13 three independent pairs; F12 env key chosen at test time; G4 dependency row adds the `Read` pair |
| 2. F32 cross-plan C3+C6 gate | PASS — F32 SC5 present and coherent (predates this pass) |
| 3. Inventory commit-binding | PASS — F31 KB + SC8 fail on commit mismatch; completeness is H4 residual |
| 4. Build-provenance attestation | PASS — F33 generates, F32 verifies, RELEASE.md documents; no signer/certificate |
| 5. SC cap (≤ 8) | PASS — F31 = 8, F32 = 6, F33 = 6 |
| 6. Preserve-list | PASS — none lost |
| 7. Section C vs live CRN | PASS — every required amendment is named; this plan does not edit CRN |
| 8. Grammar | PASS (manual) — bracketed Depends On, exact-file Produces, observable SCs, acyclic, shared files serialized |

`covenant-cli plan lint` is not on this role's bash allowlist.
No implementation or frozen `tests.md` exists; the suite was
not run.

### 1. Discriminating G4 tests

PASS. F13 Key Behaviors (`master.md:967-972`) and F13 SC1
(`:981-986`) require three independent real-sidecar pairs —
differing only by `volume_serial`, only by `file_index`, and
only by `pre_image_digest` — each producing a different
decision against the exact sidecar, so a partial G4 that
evaluates only one field fails. F12 SC1 (`:890-893`) requires
an env pair whose key is chosen at test time (not fixed in the
sidecar). The Dependencies G4 row (`:210`) lists the same
expanded set plus a `Read` pair on `$CODEX_AUTH_HOME/auth.json`.
F12/F13 do not themselves emit a `Read` event; that pair
belongs on the G4 amendment, which is correct.

F33 SC5 (`:1372-1375`) and F32 SC4 (`:1445-1448`) refuse
promotion/adoption until F12 SC1 and F13 SC1 are green against
the pinned sidecar (URL + SHA-256 + attested G4
schema/semantics id). Fixture IDs remain asserted strings;
the discriminating pairs are the behavioral gate.

### 2. F32 cross-plan gate

PASS. F32 SC5 (`:1449-1453`) and F32 Key Behaviors
(`:1415-1419`) refuse adoption until CRN C3+C6 land with
`CODEX_AUTH_HOME ≠ CODEX_HOME` when the probe permits, plus a
real-adapter separation test (auth I/O under the stable auth
root; probe-classified mutable paths follow the selected home
model). Section C (`:237-245`) matches. The "when the probe
permits" hedge is user decision 1, not a silent weaken. Live
C3 still sets both homes to S5 `config_dir`
(`child-runtime-native/master.md:1497-1503`, `:1541-1545`);
this plan does not edit C3/C6.

### 3. Inventory commit-binding

PASS as the requested machine gate. F31 Key Behaviors
(`:1264-1271`) and F31 SC8 (`:1309-1314`) fail the reaudit job
if `AUTHORITY-INVENTORY.toml`'s recorded commit ≠
`UPSTREAM.toml` commit, and they consume the inventory rather
than rediscovering families. Completeness at a new tag is
documented as a human-audit residual in
`repo-context/docs/RESIDUALS.md` H4 (`:79-85`) and restated in
F31 KB (`:1266-1271`). See Finding T1 for the F00 field-name
gap.

### 4. Build-provenance attestation

PASS. F33 Key Behaviors (`:1338-1344`) and F33 SC6
(`:1376-1380`) generate `actions/attest-build-provenance`
binding the exe digest to the workflow run, source commit, and
tag; the Release does not publish if generation fails. F32 Key
Behaviors (`:1408-1412`) and F32 SC6 (`:1454-1457`) require
`gh attestation verify` before writing the pin; missing,
failed, or unbound digest refuses. `repo-context/docs/RELEASE.md`
Provenance (`:59-76`) documents the same. No Authenticode
signer, no signer field (`F33 SC2 :1367`), no certificate as
a promotion input. Attestation is named as not a code
signature (`master.md:1341-1344`; `RELEASE.md:71-72`).

### 5. SC cap

| Phase | Produces | Success criteria |
|---|---|---|
| F01 | 1 | 3 |
| F00 | 2 | 6 |
| F02 | 1 | 2 |
| F03 | 1 | 3 |
| F10 | 2 | 3 |
| F11 | 2 | 8 |
| F12 | 3 | 8 |
| F13 | 1 | 7 |
| F14 | 2 | 7 |
| F21 | 6 | 5 |
| F22 | 2 | 3 |
| F31 | 2 | **8** |
| F33 | 2 | **6** (SC6 = attest generate) |
| F32 | 1 | **6** (SC5 = C3+C6; SC6 = attest verify) |
| F40 | 9 | 3 |

No phase exceeds 8 criteria. F11 / F12 / F31 sit at the cap
(size signal only).

### 6. Preserve-list

| Decision | Where it still holds |
|---|---|
| F11 admission-only | `:655-658`, `:667-694`, two-row table, SC1–2 |
| F12/F13 final boundaries | F12 before both start branches (`:790-793`); F13 before `apply_patch_with_options` (`:950-951`) |
| Exact ALLOW | `:813-814`; envelope docs |
| `ALLOW_WITH_CONTEXT` = DENY | F12 SC2 `:896-897`; Constraints `:1605` |
| Non-retriable `CovenantDenied` | `:670-671`, `:832`; Constraints `:1606-1607` |
| No agent-loop rewrite | Scope Out `:318`; Constraints `:1601` |
| Windows-only | Scope Out `:320`; Constraints `:1600` |
| F31 mandatory | F33 Depends On F31 (`:1320`); F33 SC3 `:1368-1369`; Constraints `:1649-1651` |
| F33 sole release path | F03 never publishes (`:525-526`, `:540`, SC3 `:550`); F32 Depends On F33 (`:1386`) |
| New identities default DENY | F11 closed table `:691-701`, SC4/SC7; Constraints `:1633-1635` |
| No code-signing | What `:35`; Why `:175-182`; F33 `:1341-1344`; Constraints `:1649-1651`; `RELEASE.md:5-7,33-38,61-72` |

### 7. Section C vs live CRN

Demand matches the live gap; this plan does not edit CRN.

| CRN phase | Live sibling | This plan's demand |
|---|---|---|
| G4 | `env_allowlist` (`child-runtime-native/master.md:2013-2014`, `:2048-2049`); identities omit `volume_serial` / `file_index` / `pre_image_digest` (`:2015-2018`, `:2055-2059`); Read-deny `$CODEX_HOME/auth.json` (`:2066-2067`) | Complete post-overlay `env`; evaluate `volume_serial` / `file_index` / `pre_image_digest`; fork Read-deny `$CODEX_AUTH_HOME/auth.json` (`codex-fork/master.md:231-236`, `:210`) |
| C3+C6 | C3 sets both homes to S5 `config_dir` (`:1449-1458`, `:1497-1503`); C6 probes that model (`:1343-1345`) | Probe, then `CODEX_AUTH_HOME ≠ CODEX_HOME` when permitted (`:237-245`); F32 SC5 gates adoption |
| C7 | Fixture `codex exec --json` capture (`:1393-1396`); no `--covenant-inventory` | Must invoke `codex --covenant-inventory` when `source = fork` (`:213`, `:253-256`) |
| C2 | `deny_read` `$CODEX_HOME/auth.json` (`:1276-1278`) | Fork-arm `deny_read` `$CODEX_AUTH_HOME/auth.json` (`:212`, `:247-249`) |
| P6 | Sentinel into stable-home auth file (`:2669-2670`) | Canary follows `$CODEX_AUTH_HOME/auth.json`; F1.4 stays accepted (`:214`, `:258-261`) |
| S2 | Official pin `0.150.1` (`:535-537`, SC1 `:570-571`); `source: official \| fork` already in schema | Bump official pin `0.150.1` → `v0.153.4`; keep `source` discriminant (`:250-251`, `:91`) |

### 8. Grammar

PASS (manual). Frontmatter complete. 15 phases, each with
Rigor, bracketed Depends On, exact-file Produces, observable
Phase Success Criteria. Graph is acyclic. Shared files
serialize: `cli/src/main.rs` F21→F22; `covenant-release.yml`
F03→F33; `COVENANT_PATCHES.md` F40→F31→F33. F12 Depends On
includes F00 (`:764`). F32 serializes with CRN S2 on
`pinned_versions.rs`.

## Findings

### T1 — F00 does not require the inventory commit field F31 compares

**Severity:** targeted
**Reviewers:** alignment
**Type:** alignment
**Location:** `master.md:408-421,470-477` vs `:1264-1266,1309-1314`
**Issue:** F31 SC8 fails the job if `AUTHORITY-INVENTORY.toml`'s
recorded commit ≠ `UPSTREAM.toml` commit, but F00 (the writer)
never requires that TOML to record the F01 40-hex. "Commit-bound"
and "at the F01 commit" describe the scan target, not a field.
Without a named key, F00 and F31 can disagree on how the bind
is stored, and SC8 is not one-pass implementable from F00's
contract.
**Suggested fix:** F00 Key Behaviors + SC6 require
`AUTHORITY-INVENTORY.toml` to record the same 40-hex as
`UPSTREAM.toml` `commit`.
**Routing:** planner

### T2 — F33 adds `attest-build-provenance` without F03's 40-hex pin

**Severity:** targeted
**Reviewers:** alignment
**Type:** alignment
**Location:** `master.md:548-549` (F03 SC2) vs `:1338-1344,1376-1380`
**Issue:** F03 SC2 requires every `uses:` in
`covenant-release.yml` pinned to a 40-hex SHA. F33 edits that
same file to add `actions/attest-build-provenance` and does
not restate the pin. A floating major tag on that action would
satisfy F33 SC6 while violating the file's F03 invariant, which
is not re-gated.
**Suggested fix:** F33 Key Behaviors + SC6 require the attest
action pinned to a 40-hex SHA, same as every other `uses:` in
that YAML.
**Routing:** planner

### T3 — Promotion-integrity restatements omit attestation

**Severity:** trivial
**Reviewers:** alignment
**Type:** alignment
**Location:** `master.md:159-160,1649-1651,1589-1594`;
`repo-context/docs/RELEASE.md:5-7` vs `:61-76`
**Issue:** Why, Constraints, and whole-plan SC11 still describe
promotion as SHA-256 + F31 + `provenance.json`. RELEASE.md's
opening sentence omits attestation that the Provenance section
then requires. Not a contradiction — F32 SC6 / F33 SC6 are the
gates — but the restatements are stale relative to the new
trust anchor. Whole-plan SC7 also does not restate F13's three
independent pairs (phase SC1 does).
**Suggested fix:** Add attestation to Why / Constraints /
whole-plan SC11 and to RELEASE.md's opening integrity sentence.
Optional: name the three F13 pairs in whole-plan SC7.
**Routing:** planner

## Acknowledged residuals

Not defects. Single-user local tool; the plan now accepts them.

- **Inventory completeness (H4).** F00/F31 cannot machine-oracle
  an omitted upstream family. Compensating: F11 compiled
  default-deny, commit-bound re-audit, human rebase review (H7).
- **Mutable Release metadata beyond attestation.** Attestation
  binds the exe digest to workflow/commit/tag. `provenance.json`
  and inventory JSON stay Release assets, not pin fields
  (`RELEASE.md:108-110`). A later asset swap of *different*
  bytes fails `gh attestation verify`; same-digest replacement
  is out of scope.
- **Live CRN still stale.** G4 `env_allowlist`, C3 same-root,
  C7 fixture `codex exec --json`, C2/P6 `$CODEX_HOME/auth.json`,
  S2 official pin `0.150.1`. Flagged preconditions; this plan
  does not edit CRN.
- **Fixture G4 IDs are asserted strings.** Discriminating
  real-sidecar pairs are the behavioral proof; deriving
  `g4_schema_id` / `g4_semantics_id` from the sidecar binary
  was not in this pass's fix list.

## Disagreements

(none)
