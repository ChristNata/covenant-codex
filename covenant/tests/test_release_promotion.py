"""F33 promotion evidence rejects incomplete or mismatched inputs."""

import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "promote_release.py"


class ReleasePromotionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location("covenant_promote", SCRIPT)
        cls.promote = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.promote)

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="covenant-release-")
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name) / "repo"
        self.artifacts = self.repo / "covenant/windows-build/artifacts/build"
        self.artifacts.mkdir(parents=True)
        self.output = self.artifacts / "promoted"
        self.exe = self.artifacts / self.promote.EXE_NAME
        self.exe.write_bytes(b"synthetic executable bytes")
        digest = hashlib.sha256(self.exe.read_bytes()).hexdigest()
        (self.artifacts / self.promote.DIGEST_NAME).write_text(
            digest + "\n", encoding="ascii"
        )
        self.inventory = self.artifacts / "inventory.json"
        self.inventory.write_text(
            json.dumps({"binary_digest": digest, "inventory_id": "inv-1"}) + "\n",
            encoding="utf-8",
        )
        self.reaudit = self.artifacts / "reaudit.json"
        self.reaudit.write_text(
            json.dumps({"status": "passed", "commit": "a" * 40, "run_id": "run-1"})
            + "\n",
            encoding="utf-8",
        )
        self.patches = self.repo / "COVENANT_PATCHES.md"
        self.patches.write_text(
            "\n".join(
                f'{phase} hunk_sha256 = "{"b" * 64}"'
                for phase in ["F11", "F12", "F13", "F14"]
            )
            + "\n",
            encoding="utf-8",
        )
        self.sidecar = self.repo / "sidecar-fixture.toml"
        self.sidecar.write_text(
            "\n".join(
                [
                    "schema_version = 1",
                    'status = "ready"',
                    'url = "https://example.invalid/covenant-cli.exe"',
                    f'sha256 = "{"d" * 64}"',
                    'g4_schema_id = "decide-v1-sha256:test"',
                    'g4_semantics_id = "g4-codex-v1:test"',
                ]
            )
            + "\n",
            encoding="utf-8",
        )

    def invoke(self):
        return self.promote.assemble_provenance(
            repo=self.repo,
            artifact_dir=self.artifacts,
            inventory_path=self.inventory,
            reaudit_path=self.reaudit,
            source_commit="a" * 40,
            tag="v0.1.0-covenant",
            toolchain="1.95.0",
            runner="windows-2022",
            patches_path=self.patches,
            sidecar_fixture_path=self.sidecar,
            output_dir=self.output,
        )

    def test_valid_inputs_emit_five_release_assets_and_bound_provenance(self):
        provenance = self.invoke()
        self.assertEqual(provenance["source_commit"], "a" * 40)
        self.assertEqual(provenance["tag"], "v0.1.0-covenant")
        self.assertEqual(provenance["re_audit_run_id"], "run-1")
        self.assertEqual(provenance["sidecar"]["g4_semantics_id"], "g4-codex-v1:test")
        self.assertEqual(
            provenance["exe_digest"], provenance["inventory"]["binary_digest"]
        )
        self.assertNotIn("signer", provenance)
        self.assertEqual(
            {path.name for path in self.output.iterdir()},
            {
                self.promote.EXE_NAME,
                self.promote.DIGEST_NAME,
                "covenant-inventory.json",
                "provenance.json",
                "COVENANT_PATCHES.md",
            },
        )

    def test_digest_mismatch_refuses_without_creating_promotion(self):
        self.inventory.write_text(
            json.dumps({"binary_digest": "c" * 64, "inventory_id": "inv-1"}) + "\n",
            encoding="utf-8",
        )
        with self.assertRaises(self.promote.PromotionError):
            self.invoke()
        self.assertFalse(self.output.exists())

    def test_unverified_reaudit_and_placeholder_hunks_refuse(self):
        self.reaudit.write_text(
            json.dumps({"status": "passed", "commit": "d" * 40, "run_id": "run-1"}),
            encoding="utf-8",
        )
        with self.assertRaises(self.promote.PromotionError):
            self.invoke()
        self.assertFalse(self.output.exists())

    def test_workflow_keeps_dispatch_nonpublishing_and_gates_release_on_tags(self):
        workflow = (
            Path(__file__).resolve().parents[2]
            / ".github"
            / "workflows"
            / "covenant-release.yml"
        ).read_text(encoding="utf-8")
        self.assertIn("workflow_dispatch:", workflow)
        self.assertIn("tags:\n      - 'covenant-v*'", workflow)
        self.assertIn("needs: windows-artifact", workflow)
        self.assertIn("startsWith(github.ref, 'refs/tags/covenant-v')", workflow)
        self.assertIn(
            "actions/attest-build-provenance@43d14bc2b83dec42d39ecae14e916627a18bb661",
            workflow,
        )
        self.assertIn("--sidecar-fixture", workflow)

        self.reaudit.write_text(
            json.dumps({"status": "passed", "commit": "a" * 40, "run_id": "run-1"}),
            encoding="utf-8",
        )
        self.patches.write_text(
            "Final hunk hashes do not exist yet.\n", encoding="utf-8"
        )
        with self.assertRaises(self.promote.PromotionError):
            self.invoke()
        self.assertFalse(self.output.exists())

    def test_missing_real_sidecar_evidence_refuses_promotion(self):
        self.sidecar.write_text(
            'schema_version = 1\nstatus = "pending"\n', encoding="utf-8"
        )
        with self.assertRaises(self.promote.PromotionError):
            self.invoke()
        self.assertFalse(self.output.exists())


if __name__ == "__main__":
    unittest.main()
