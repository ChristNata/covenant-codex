# Review: codex-fork

**Date:** 2026-09-05
**Rigor:** hard
**Phase:** plan
**Reviewers:** adversarial

## Verdict

FAIL

## Findings

Found 4 blocker, 0 structural, 0 targeted, 0 trivial.

### The G4 identifier is self-asserted and the behavioral tests cover only a subset of its claim

**Severity:** blocker
**Reviewers:** adversarial
**Type:** fakery
**Location:** `docs/master-plans/cross/codex-fork/master.md:834-845,874-878,885-892,965-983,1333-1357,1418-1421`; `docs/master-plans/cross/child-runtime-native/master.md:2009-2068,2075-2088`
**Issue:** The fixture's `g4_schema_id` and `g4_semantics_id` are strings supplied
beside the sidecar URL and digest; nothing derives them from the sidecar's loaded
schema or binds them to the G4 source commit. The real-sidecar pairs improve the
gate, but F12 varies only one environment pair, F13 varies only one of three
identity fields, and neither pair tests the claimed fork `Read` denial under
`$CODEX_AUTH_HOME/auth.json`. The exact fully stale G4 currently described in
CRN cannot pass a genuine env-only discriminating pair, but a partial G4 can.
**Evidence:** Build a sidecar that generically evaluates one chosen env key and
`pre_image_digest`, leaves `volume_serial` and `file_index` unused, and retains
the current `$CODEX_HOME/auth.json` Read rule. Put the current schema/semantics
strings beside that executable's real SHA-256. F12 SC1 passes on the chosen env
key; F13 SC1 passes because it requires only “one of” the three fields; F33 SC5
and F32 SC4 see the asserted IDs and green tests. Promotion/adoption succeeds
although two replacement identities and the auth-root Read rule remain inert.
**Suggested fix:** Make the sidecar emit a build-derived contract record bound to
its binary digest and G4 source/schema digest, then verify it. Add independent,
order-insensitive real-sidecar pairs for `volume_serial`, `file_index`, and
`pre_image_digest`, a randomized-key complete-env rule, and a real `Read` pair
for `$CODEX_AUTH_HOME/auth.json`. Gate F33/F32 on every pair, not one selected
representative.
**Routing:** human

### F32 cannot establish the claimed integrated home separation against the canonical CRN plan

**Severity:** blocker
**Reviewers:** adversarial
**Type:** correctness
**Location:** `docs/master-plans/cross/codex-fork/master.md:211,237-246,1385-1426,1545-1549,1561-1563`; `docs/master-plans/cross/child-runtime-native/master.md:1325-1367,1431-1460,1497-1503,1541-1545`
**Issue:** The new F32 criterion is prose in a phase whose sole Produce is
`pinned_versions.rs`; no canonical CRN phase currently produces the required
redirectability result, amended adapter, or real-adapter separation evidence.
Current C6 inventories writes under a stable home and may record shared paths;
it has no “probe permits” result. Current C3 sets both `CODEX_HOME` and
`CODEX_AUTH_HOME` to the S5 `config_dir`. The conditional “when the probe
permits” is therefore ungrounded and can be treated as false, while the selected
stable-home model still satisfies the weaker path-classification wording.
**Evidence:** Complete F14 with two caller-supplied roots, leave canonical C6/C3
unchanged, publish F33, and write the F32 pin. The actual fork launch receives
the same root for both variables. C6 can classify remaining mutable paths as
shared and cap concurrency rather than require separation. Nothing in F32's
Produce can change that launch or supply the missing probe branch; the master
claim of landed “distinct-root separation” is not an output of either plan as
currently written.
**Suggested fix:** Amend and re-review canonical CRN C6/C3 before this plan can
pass. C6 must emit an explicit redirectability decision with closed mutable-path
coverage. C3 must consume it. A CRN-owned real-adapter test must exercise both
branches and prove physical root inequality on the permitted branch plus zero
auth I/O under every mutable root. Make F32 depend on that named, landed CRN
artifact/gate rather than restating it as an external condition.
**Routing:** human

### The authority inventory can certify its own omission

**Severity:** blocker
**Reviewers:** adversarial
**Type:** fakery
**Location:** `docs/master-plans/cross/codex-fork/master.md:400-421,465-477,1257-1275,1296-1302,1554-1557`
**Issue:** F00 authors the authority list and the procedure that is supposed to
prove that list complete. F31 explicitly consumes that list and does not
rediscover constructors or sink families. One mutation per *inventoried* family
proves only that the harness detects mutations for rows already present. It
cannot detect an omitted new upstream family, and SC7's promise to catch new
callers contradicts the inventory-only input unless an independent discovery
surface is specified.
**Evidence:** At the F01 tag, add a non-model startup authority through a new FFI
wrapper family and omit that family from `AUTHORITY-INVENTORY.toml`. Keep every
listed row and audit-procedure query correct. F00 SC6 has no independent oracle
that fails the omission. F31 instruments every listed row and every listed
family mutation, so SC8 is green; the new startup effect is outside F11 registry
domination and executes in the promoted binary. The same false-green occurs on
a later rebase if `UPSTREAM.toml` advances while the inventory remains complete
only for its prior vocabulary.
**Suggested fix:** Define an independent, source-derived discovery gate and bind
the inventory to the exact upstream commit. It must enumerate and reject every
unclassified constructor and sink call site, including new dependency/FFI and
startup entry paths, before the disposition file is consumed. Mutate every sink
entry as well as every family adapter, and make a commit mismatch between
`UPSTREAM.toml` and the inventory fail F31.
**Routing:** human

### Mutable unsigned Release metadata can authorize bytes that never passed F31

**Severity:** blocker
**Reviewers:** adversarial
**Type:** correctness
**Location:** `docs/master-plans/cross/codex-fork/master.md:1320-1357,1374-1392,1410-1421,1618-1620`; `docs/master-plans/cross/codex-fork/repo-context/docs/RELEASE.md:26-43,45-66,68-104`
**Issue:** The no-Authenticode sequence is internally ordered correctly—build,
hash, inventory, provenance, publish—and no signer field remains dangling. Its
integrity claim is still circular. The executable, checksum, inventory, and
unsigned `provenance.json` are mutable assets controlled by the same Release
publisher. F32 trusts the checksum currently served by that Release and does not
bind it to an immutable F33 workflow result, source commit, or independently
verified attestation. SHA-256 proves byte equality only after a trusted digest
exists; it does not authenticate that initial digest.
**Evidence:** Let legitimate F33 publish asset A after F31. Before F32, replace
the Release assets with executable B plus regenerated checksum, inventory, and
provenance naming the expected source. F32 SC1 sees the expected URL and SC2
copies B's matching 64-hex digest. The sidecar and home tests are unrelated, so
SC4/SC5 do not detect the swap. The harness then pins, downloads, and runs B even
though F31 never examined it. A later swap would be caught by the pin; the
initial adoption and each future pin bump remain vulnerable.
**Suggested fix:** Preserve the no-Authenticode decision but add an independent
trust anchor: require immutable GitHub Releases plus a verified GitHub artifact
attestation bound to the exact tag, source commit, workflow, and executable
digest, or derive the F32 digest from an independently reproduced/verified build.
F32 must verify that binding and the provenance/inventory digests before writing
the pin; matching mutable Release files is insufficient.
**Routing:** human

## Success-criterion assessment

`PASS` here means the criterion is buildable and discriminating at plan level;
it is not implementation evidence.

| Scope | Criterion verdicts |
|---|---|
| F01 | SC1 PASS; SC2 PASS; SC3 PASS |
| F00 | SC1 PASS; SC2 PASS; SC3 PASS; SC4 PASS; SC5 PASS; SC6 FAIL — completeness is self-attested |
| F02 | SC1 PASS; SC2 PASS |
| F03 | SC1 PASS; SC2 PASS; SC3 PASS |
| F10 | SC1 PASS; SC2 PASS; SC3 PASS — schema conditionals now bite |
| F11 | SC1 PASS; SC2 PASS; SC3 PASS; SC4 PASS; SC5 PASS; SC6 PASS; SC7 PASS; SC8 PASS — sufficient plumbing proof for the general `exec_command` fallback, not a claim about model quality |
| F12 | SC1 FAIL — partial/self-asserted G4 can pass; SC2 PASS; SC3 PASS; SC4 PASS; SC5 PASS; SC6 PASS; SC7 PASS — the case-insensitive immutable-block fix closes S3 at plan level; SC8 PASS |
| F13 | SC1 FAIL — “one of” does not prove all identity semantics; SC2 PASS; SC3 PASS; SC4 PASS; SC5 PASS; SC6 PASS; SC7 PASS |
| F14 | SC1 PASS; SC2 PASS; SC3 PASS; SC4 PASS; SC5 PASS; SC6 PASS; SC7 PASS — isolated fork behavior only; integrated failure is F32/CRN |
| F21 | SC1 PASS; SC2 PASS; SC3 PASS; SC4 PASS; SC5 PASS |
| F22 | SC1 PASS; SC2 PASS; SC3 PASS |
| F31 | SC1 PASS; SC2 PASS; SC3 PASS; SC4 PASS; SC5 PASS; SC6 PASS; SC7 FAIL; SC8 FAIL — omitted families remain invisible |
| F33 | SC1 PASS with integrity gap; SC2 PASS with integrity gap; SC3 PASS; SC4 PASS; SC5 FAIL — incomplete G4 evidence |
| F32 | SC1 PASS; SC2 FAIL — digest source is mutable/self-authenticating; SC3 PASS; SC4 FAIL — incomplete G4 evidence; SC5 FAIL — canonical CRN has no such result/test |
| F40 | SC1 PASS; SC2 PASS; SC3 PASS |
| Master | SC1 FAIL; SC2 PASS; SC3 PASS; SC4 PASS; SC5 PASS — F11 viability fix is adequate; SC6 FAIL; SC7 FAIL; SC8 FAIL; SC9 PASS; SC10 FAIL; SC11 FAIL; SC12 PASS |

## Verification record

- Test suite: not run; this is `review-plan`, and no implementation or frozen
  `tests.md` exists in the reviewed workspace.
- Plan lint: not run; reviewer Bash policy denied `covenant-cli plan lint`.
- Adversarial-fakery checklist: applied. The material failures are
  self-attested IDs/inventory and mutable self-authenticating release evidence;
  no implementation exists yet for code-level constant, early-return, stub,
  dead-code, match-arm, dependency-call, or side-effect inspection.

## Disagreements

(none)
