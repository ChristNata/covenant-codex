"""E01 manifest validation stays fail-closed until real evidence exists."""

import importlib.util
from pathlib import Path
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "validate_sidecar_fixture.py"


class SidecarFixtureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location("sidecar_fixture", SCRIPT)
        cls.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.module)

    def test_repository_manifest_is_pending(self):
        fixture = Path(__file__).resolve().parents[1] / "sidecar-fixture.toml"
        with self.assertRaises(self.module.SidecarFixtureError):
            self.module.read_fixture(fixture)

    def test_ready_manifest_requires_all_attested_contract_fields(self):
        with tempfile.TemporaryDirectory(prefix="covenant-sidecar-") as root:
            fixture = Path(root) / "fixture.toml"
            fixture.write_text(
                "\n".join(
                    [
                        "schema_version = 1",
                        'status = "ready"',
                        'url = "https://example.invalid/covenant-cli.exe"',
                        f'sha256 = "{"a" * 64}"',
                        'g4_schema_id = "decide-v1-sha256:test"',
                        'g4_semantics_id = "g4-codex-v1:test"',
                    ]
                ),
                encoding="utf-8",
            )
            self.assertEqual(self.module.read_fixture(fixture)["status"], "ready")


if __name__ == "__main__":
    unittest.main()
