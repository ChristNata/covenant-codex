"""Offline F00 structural checks using disposable Git fixtures.

Literal source anchors and declared coverage are structural evidence only.
These tests do not establish authority completeness or semantic gate domination.
An explicit --validator PATH may select a temporary permissive baseline adapter;
the default always invokes the real production CLI.
"""

import argparse
import copy
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import uuid


VALIDATOR = Path(__file__).resolve().parents[1] / "scripts" / "validate_integration.py"
PHASES = {
    "cli_entry": "F21",
    "config_resolution": "F21",
    "managed_requirements": "F21",
    "model_catalog": "F21",
    "auth_storage": "F14",
    "auth_refresh": "F14",
    "tool_admission": "F11",
    "process_start": "F12",
    "patch_runtime": "F13",
    "tool_registration": "F21",
    "feature_registry": "F21",
}
FAMILIES = (
    "process",
    "filesystem",
    "network",
    "hosted",
    "mcp",
    "hook",
    "generated_code",
)
CONTROLS = (
    "COVENANT_DECIDER_PATH",
    "COVENANT_DECIDER_SHA256",
    "COVENANT_CHILD_MARKER",
    "CODEX_AUTH_HOME",
)


class IntegrationTests(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(prefix="covenant-integration-")
        self.addCleanup(self.scratch.cleanup)
        self.repo = Path(self.scratch.name) / "repo"
        self.repo.mkdir()
        self.git("init", "--initial-branch=covenant-ver")
        suffix = uuid.uuid4().hex[:8]
        self.provider_key = f"PROVIDER_{suffix.upper()}_TOKEN"
        self.source_path = "codex-rs/source.rs"
        self.source = self.repo / self.source_path
        self.source.parent.mkdir()
        symbols = [*PHASES, "exec_handler", "patch_handler", "constructor"]
        symbols += [f"{family}_sink" for family in FAMILIES]
        self.source_text = "\n".join(f"fn {symbol}() {{}}" for symbol in symbols)
        self.source_text += f'\nconst {self.provider_key}: &str = "fixture-only";\n'
        self.source.write_text(self.source_text, encoding="utf-8", newline="\n")
        self.git("add", "--all")
        self.git("commit", "--quiet", "-m", "Synthetic source inventory")
        self.commit = self.git("rev-parse", "HEAD")
        tools = [
            {
                "id": f"exec_{suffix}",
                "namespace": "",
                "name": "exec_command",
                "form": "function",
                "effect": "process",
                "path": self.source_path,
                "symbol": "exec_handler",
            },
            {
                "id": f"patch_{suffix}",
                "namespace": "",
                "name": "apply_patch",
                "form": "custom",
                "effect": "filesystem",
                "path": self.source_path,
                "symbol": "patch_handler",
            },
        ]
        authorities = [
            {
                "id": f"authority_{tool['id']}",
                "kind": "tool",
                "tool_id": tool["id"],
                "family": tool["effect"],
                "disposition": "gated",
                "gate": gate,
                "path": self.source_path,
                "symbol": tool["symbol"],
                "reason": "Synthetic declared gate.",
            }
            for tool, gate in zip(
                tools, ("process_start", "patch_runtime"), strict=True
            )
        ]
        authorities += [
            {
                "id": f"sink_{family}",
                "kind": "sink",
                "family": family,
                "disposition": "trusted_support",
                "path": self.source_path,
                "symbol": f"{family}_sink",
                "reason": "Synthetic classified sink.",
            }
            for family in FAMILIES
        ]
        authorities.append(
            {
                "id": "factory",
                "kind": "constructor",
                "family": "process",
                "disposition": "trusted_support",
                "path": self.source_path,
                "symbol": "constructor",
                "reason": "Synthetic constructor.",
            }
        )
        self.models = {
            "upstream": {"commit": self.commit},
            "integration": {
                "schema_version": 1,
                "commit": self.commit,
                "absent_seams": [
                    "pre_tool_hook",
                    "hook_runtime",
                    "mcp_launcher",
                    "mcp_manager",
                    "build_metadata",
                ],
                "seams": [
                    {
                        "id": name,
                        "phase": phase,
                        "path": self.source_path,
                        "symbols": [name],
                    }
                    for name, phase in PHASES.items()
                ],
                "tools": tools,
            },
            "inventory": {
                "schema_version": 1,
                "commit": self.commit,
                "scrub_keys": [*CONTROLS, self.provider_key],
                "scrub_prefixes": [],
                "authorities": authorities,
                "secrets": [
                    {
                        "key": self.provider_key,
                        "class": "provider_login",
                        "path": self.source_path,
                        "symbol": self.provider_key,
                    }
                ],
                "audit_queries": [
                    {
                        "family": family,
                        "path": self.source_path,
                        "symbol": f"{family}_sink",
                    }
                    for family in FAMILIES
                ],
            },
        }
        self.paths = {name: self.repo / f"{name}.toml" for name in self.models}
        self.save()

    def git(self, *args):
        env = {
            key: value
            for key, value in os.environ.items()
            if not key.startswith("GIT_")
        }
        env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull)
        result = subprocess.run(
            [
                "git",
                "-c",
                "user.name=Integration Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.autocrlf=false",
                "-c",
                f"core.hooksPath={self.repo / 'absent-hooks'}",
                *args,
            ],
            cwd=self.repo,
            env=env,
            capture_output=True,
            text=True,
            timeout=20,
            check=True,
        )
        return result.stdout.strip()

    def save(self):
        for name, model in self.models.items():
            scalar = []
            tables = []
            for field, value in model.items():
                if isinstance(value, list) and value and isinstance(value[0], dict):
                    for row in value:
                        tables.append(f"\n[[{field}]]\n")
                        tables.extend(
                            f"{key} = {json.dumps(item)}\n" for key, item in row.items()
                        )
                else:
                    scalar.append(f"{field} = {json.dumps(value)}\n")
            self.paths[name].write_text(
                "".join(scalar + tables), encoding="utf-8", newline="\n"
            )

    def validate(self):
        return subprocess.run(
            [
                sys.executable,
                str(VALIDATOR),
                "--repo",
                str(self.repo),
                "--upstream",
                str(self.paths["upstream"]),
                "--integration",
                str(self.paths["integration"]),
                "--inventory",
                str(self.paths["inventory"]),
            ],
            capture_output=True,
            text=True,
            timeout=20,
        )

    def assert_allowed(self):
        result = self.validate()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(self.commit, result.stdout)
        self.assertRegex(result.stdout.lower(), r"\b(seams|tools|authorities)\b")

    def assert_denied(self, category):
        result = self.validate()
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn(category.lower(), result.stderr.lower())
        self.assertIn("remediation:", result.stderr.lower())
        self.assertNotIn("Traceback", result.stderr)
        self.assertNotIn(self.source_text, result.stderr)

    def test_accepts_declared_synthetic_mapping_without_claiming_source_completeness(
        self,
    ):
        self.assert_allowed()

    def test_accepts_explicit_optional_absence_and_provider_prefix_coverage(self):
        self.models["inventory"]["scrub_keys"].remove(self.provider_key)
        self.models["inventory"]["scrub_prefixes"] = ["PROVIDER_"]
        self.save()
        self.assert_allowed()

    def test_accepts_source_anchored_tool_search_payload_form(self):
        tool = {
            "id": f"search_{uuid.uuid4().hex[:8]}",
            "namespace": "",
            "name": "tool_search",
            "form": "tool_search",
            "effect": "mcp",
            "path": self.source_path,
            "symbol": "mcp_sink",
        }
        self.models["integration"]["tools"].append(tool)
        self.models["inventory"]["authorities"].append(
            {
                "id": f"authority_{tool['id']}",
                "kind": "tool",
                "tool_id": tool["id"],
                "family": "mcp",
                "disposition": "excluded",
                "path": self.source_path,
                "symbol": "mcp_sink",
                "reason": "Inventoried separate payload form; no admission granted.",
            }
        )
        self.save()
        self.assert_allowed()

    def test_rejects_mismatched_missing_malformed_and_noncommit_bindings(self):
        baseline = copy.deepcopy(self.models)
        for name in self.models:
            for value in ("f" * 40, "HEAD", [self.commit], None):
                with self.subTest(manifest=name, value=value):
                    self.models = copy.deepcopy(baseline)
                    if value is None:
                        del self.models[name]["commit"]
                    else:
                        self.models[name]["commit"] = value
                    self.save()
                    self.assert_denied("commit")
        for invalid in ("f" * 40, self.git("rev-parse", f"HEAD:{self.source_path}")):
            with self.subTest(common_commit=invalid):
                self.models = copy.deepcopy(baseline)
                for model in self.models.values():
                    model["commit"] = invalid
                self.save()
                self.assert_denied("commit")

    def test_rejects_missing_pinned_or_working_source_files(self):
        self.source.unlink()
        self.assert_denied("path")
        self.source.write_text(self.source_text, encoding="utf-8")
        untracked = self.repo / "codex-rs" / "untracked.rs"
        untracked.write_text(self.source_text, encoding="utf-8")
        self.models["integration"]["seams"][0]["path"] = "codex-rs/untracked.rs"
        self.save()
        self.assert_denied("path")

    def test_rejects_missing_anchor_in_either_pinned_or_working_view(self):
        self.source.write_text(
            self.source_text.replace("fn process_start() {}", ""), encoding="utf-8"
        )
        self.assert_denied("symbol")
        self.source.write_text(
            self.source_text + "\nfn unpinned_symbol() {}", encoding="utf-8"
        )
        self.models["integration"]["seams"][0]["symbols"] = ["unpinned_symbol"]
        self.save()
        self.assert_denied("symbol")

    def test_rejects_repository_escapes_and_unnormalized_source_paths(self):
        for path in (
            "../outside.rs",
            "/outside.rs",
            "C:\\outside.rs",
            "codex-rs/../source.rs",
        ):
            with self.subTest(path=path):
                self.models["integration"]["seams"][0]["path"] = path
                self.save()
                self.assert_denied("path")

    def test_rejects_absent_required_unknown_duplicate_and_unaccounted_seams(self):
        baseline = copy.deepcopy(self.models)
        for fault in (
            "missing",
            "unknown",
            "duplicate",
            "unaccounted",
            "present-and-absent",
        ):
            with self.subTest(fault=fault):
                self.models = copy.deepcopy(baseline)
                model = self.models["integration"]
                if fault == "missing":
                    model["seams"].pop()
                elif fault == "unknown":
                    model["seams"][0]["id"] = "unknown_seam"
                elif fault == "duplicate":
                    model["seams"].append(copy.deepcopy(model["seams"][0]))
                elif fault == "unaccounted":
                    model["absent_seams"].remove("mcp_manager")
                else:
                    model["absent_seams"].append(model["seams"][0]["id"])
                self.save()
                self.assert_denied("seam")

    def test_rejects_duplicate_tool_ids_and_duplicate_wire_identities(self):
        original = copy.deepcopy(self.models["integration"]["tools"][0])
        for duplicate_id in (True, False):
            with self.subTest(duplicate_id=duplicate_id):
                duplicate = copy.deepcopy(original)
                if not duplicate_id:
                    duplicate["id"] += "_different"
                self.models["integration"]["tools"] = [
                    *self.models["integration"]["tools"][:2],
                    duplicate,
                ]
                self.save()
                self.assert_denied("tool")

    def test_rejects_unknown_classification_values_and_duplicate_authority_ids(self):
        baseline = copy.deepcopy(self.models)
        for collection, field, category in (
            ("tools", "effect", "effect"),
            ("tools", "form", "form"),
            ("authorities", "kind", "kind"),
            ("authorities", "family", "family"),
            ("authorities", "disposition", "disposition"),
        ):
            with self.subTest(collection=collection, field=field):
                self.models = copy.deepcopy(baseline)
                owner = "integration" if collection == "tools" else "inventory"
                self.models[owner][collection][0][field] = "unknown_classification"
                self.save()
                self.assert_denied(category)
        self.models = copy.deepcopy(baseline)
        rows = self.models["inventory"]["authorities"]
        rows.append(copy.deepcopy(rows[-1]))
        self.save()
        self.assert_denied("authorit")

    def test_rejects_foreign_tool_gate_and_missing_or_duplicate_tool_classification(
        self,
    ):
        baseline = copy.deepcopy(self.models)
        for fault in ("foreign-tool", "foreign-gate", "missing-tool", "duplicate-tool"):
            with self.subTest(fault=fault):
                self.models = copy.deepcopy(baseline)
                rows = self.models["inventory"]["authorities"]
                if fault == "foreign-tool":
                    rows[0]["tool_id"] = "undeclared_tool"
                elif fault == "foreign-gate":
                    rows[0]["gate"] = "undeclared_gate"
                elif fault == "missing-tool":
                    rows.pop(0)
                else:
                    duplicate = copy.deepcopy(rows[0])
                    duplicate["id"] += "_different"
                    rows.append(duplicate)
                self.save()
                self.assert_denied("gate" if fault == "foreign-gate" else "tool")

    def test_rejects_missing_declared_family_and_audit_query_coverage(self):
        baseline = copy.deepcopy(self.models)
        for field in ("authorities", "audit_queries"):
            with self.subTest(field=field):
                self.models = copy.deepcopy(baseline)
                self.models["inventory"][field] = [
                    row
                    for row in self.models["inventory"][field]
                    if row["family"] != "network"
                ]
                self.save()
                self.assert_denied("family")

    def test_rejects_unknown_uncovered_and_case_colliding_secret_definitions(self):
        baseline = copy.deepcopy(self.models)
        for fault in (
            "uncovered",
            "unknown-class",
            "duplicate-secret",
            "scrub-collision",
            "missing-control",
        ):
            with self.subTest(fault=fault):
                self.models = copy.deepcopy(baseline)
                model = self.models["inventory"]
                if fault == "uncovered":
                    model["scrub_keys"].remove(self.provider_key)
                elif fault == "unknown-class":
                    model["secrets"][0]["class"] = "unknown"
                elif fault == "duplicate-secret":
                    duplicate = copy.deepcopy(model["secrets"][0])
                    duplicate["key"] = duplicate["key"].lower()
                    model["secrets"].append(duplicate)
                elif fault == "scrub-collision":
                    model["scrub_keys"].append(self.provider_key.lower())
                else:
                    model["scrub_keys"].remove("CODEX_AUTH_HOME")
                self.save()
                self.assert_denied(
                    "scrub"
                    if fault in ("scrub-collision", "missing-control")
                    else "secret"
                )

    def test_rejects_manifest_commands_unknown_fields_and_wrong_types(self):
        baseline = copy.deepcopy(self.models)
        for fault in (
            "command",
            "unknown-field",
            "type",
            "empty-symbols",
            "unknown-phase",
            "boolean-version",
        ):
            with self.subTest(fault=fault):
                self.models = copy.deepcopy(baseline)
                if fault == "command":
                    self.models["inventory"]["audit_queries"][0]["command"] = (
                        "do-not-execute"
                    )
                elif fault == "unknown-field":
                    self.models["inventory"]["unexpected"] = "do-not-consume"
                elif fault == "type":
                    self.models["integration"]["tools"] = "wrong type"
                elif fault == "empty-symbols":
                    self.models["integration"]["seams"][0]["symbols"] = []
                elif fault == "unknown-phase":
                    self.models["integration"]["seams"][0]["phase"] = "F99"
                else:
                    self.models["inventory"]["schema_version"] = True
                self.save()
                self.assert_denied(
                    "field"
                    if fault in ("command", "unknown-field")
                    else "symbol"
                    if fault == "empty-symbols"
                    else "phase"
                    if fault == "unknown-phase"
                    else "schema_version"
                    if fault == "boolean-version"
                    else "tools"
                )

    def test_rejects_missing_or_malformed_inputs_without_echoing_source(self):
        for name, path in self.paths.items():
            with self.subTest(manifest=name):
                path.unlink()
                self.assert_denied("manifest")
                path.write_text("commit = [unterminated\n", encoding="utf-8")
                self.assert_denied("manifest")
                self.save()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--validator", type=Path, default=VALIDATOR)
    options, unittest_args = parser.parse_known_args()
    VALIDATOR = options.validator.resolve()
    unittest.main(argv=[sys.argv[0], *unittest_args])
