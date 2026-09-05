"""F03 pair checks use disposable bytes, never a product or hosted upload."""

import argparse
from contextlib import redirect_stderr, redirect_stdout
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


VERIFIER = Path(__file__).resolve().parents[1] / "scripts/verify_windows_artifact.py"
EXE_NAME = "codex-x86_64-pc-windows-msvc.exe"
DIGEST_NAME = "codex.exe.sha256"


class WindowsArtifactTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location(
            "covenant_pair_verifier", VERIFIER
        )
        cls.verifier = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = cls.verifier
        spec.loader.exec_module(cls.verifier)

    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(prefix="covenant-pair-")
        self.addCleanup(self.scratch.cleanup)
        self.repo = Path(self.scratch.name) / "repo with spaces"
        self.artifacts = self.repo / "covenant/windows-build/artifacts"
        self.bundle = self.artifacts / "build 123 attempt 1"
        self.bundle.mkdir(parents=True)
        self.exe = self.bundle / EXE_NAME
        self.digest_file = self.bundle / DIGEST_NAME
        self.sentinel = self.bundle / "keep-unrelated.bin"
        self.sentinel.write_bytes(b"unrelated existing bytes\x00\xff")
        self.payload = b"SYNTHETIC PAIR ONLY\x00\xff\x80\r\n\n"
        self.write_pair(self.bundle, self.payload)

    def write_pair(self, directory, payload):
        directory.mkdir(parents=True, exist_ok=True)
        (directory / EXE_NAME).write_bytes(payload)
        (directory / DIGEST_NAME).write_bytes(
            hashlib.sha256(payload).hexdigest().encode("ascii") + b"\n"
        )

    def snapshot(self):
        result = {}
        for path in [self.exe, self.digest_file, self.sentinel]:
            if path.is_file():
                result[path.name] = path.read_bytes()
            elif path.is_dir():
                result[path.name] = (
                    "directory",
                    tuple(sorted(p.name for p in path.iterdir())),
                )
            else:
                result[path.name] = None
        result["bundle_entries"] = tuple(
            sorted(path.name for path in self.bundle.iterdir())
        )
        return result

    def invoke(self, directory=None):
        before = self.snapshot()
        stdout, stderr = io.StringIO(), io.StringIO()
        with redirect_stdout(stdout), redirect_stderr(stderr):
            code = self.verifier.main(
                [
                    "--repo",
                    str(self.repo),
                    "--artifact-dir",
                    str(directory or self.bundle),
                ]
            )
        self.assertEqual(self.snapshot(), before, "verification mutated fixture inputs")
        return code, stdout.getvalue(), stderr.getvalue()

    def assert_refused(self, result):
        code, stdout, stderr = result
        self.assertEqual(code, 1, (stdout, stderr))
        self.assertEqual(stdout, "")
        self.assertIn("remediation:", stderr.lower())
        self.assertLessEqual(len(stderr.encode("utf-8")), 4096)
        self.assertNotIn("Traceback", stderr)
        self.assertNotIn("UNTRUSTED-DIGEST-CONTENT", stderr)

    def test_valid_pair_reports_exact_bytes_without_mutation_or_promotion_claim(self):
        for payload in [self.payload, self.payload * 65537]:
            with self.subTest(size=len(payload)):
                self.write_pair(self.bundle, payload)
                code, stdout, stderr = self.invoke()
                self.assertEqual(code, 0, stderr)
                self.assertEqual(stderr, "")
                self.assertEqual(
                    json.loads(stdout),
                    {
                        "status": "artifact-pair-verified",
                        "artifact": {
                            "name": EXE_NAME,
                            "size_bytes": len(payload),
                            "sha256": hashlib.sha256(payload).hexdigest(),
                        },
                        "digest_file": DIGEST_NAME,
                    },
                )

    def test_each_missing_member_refuses_while_preserving_the_other(self):
        for name in [EXE_NAME, DIGEST_NAME]:
            with self.subTest(missing=name):
                self.write_pair(self.bundle, self.payload)
                (self.bundle / name).unlink()
                self.assert_refused(self.invoke())

    def test_changed_executable_cannot_match_the_old_digest(self):
        self.exe.write_bytes(self.payload + b"changed after build")
        self.assert_refused(self.invoke())

    def test_digest_requires_exact_lowercase_hex_and_one_lf(self):
        digest = hashlib.sha256(self.payload).hexdigest().encode("ascii")
        for value in [
            b"",
            digest,
            digest.upper() + b"\n",
            digest + b"\r\n",
            digest + b"\n\n",
            digest + b"\ntrailing",
            b"g" * 64 + b"\n",
            b"\xff" * 64 + b"\n",
            b"UNTRUSTED-DIGEST-CONTENT" * 4096,
        ]:
            with self.subTest(length=len(value), prefix=value[:8]):
                self.digest_file.write_bytes(value)
                self.assert_refused(self.invoke())

    def test_empty_or_nonregular_members_refuse(self):
        self.write_pair(self.bundle, b"")
        with self.subTest(empty_executable=True):
            self.assert_refused(self.invoke())
        self.write_pair(self.bundle, self.payload)
        for name in [EXE_NAME, DIGEST_NAME]:
            with self.subTest(directory_instead_of=name):
                path = self.bundle / name
                path.unlink()
                path.mkdir()
                self.assert_refused(self.invoke())
                path.rmdir()
                self.write_pair(self.bundle, self.payload)

    def test_outside_namespace_root_traversal_and_missing_paths_refuse(self):
        outside = Path(self.scratch.name) / "outside bundle"
        self.write_pair(outside, self.payload)
        self.write_pair(self.artifacts, self.payload)
        outside_before = {
            name: (outside / name).read_bytes() for name in [EXE_NAME, DIGEST_NAME]
        }
        for directory in [
            outside,
            self.artifacts,
            self.bundle / ".." / self.bundle.name,
            self.artifacts / "missing bundle",
            self.sentinel,
        ]:
            with self.subTest(directory=str(directory)):
                self.assert_refused(self.invoke(directory))
        self.assertEqual(
            {name: (outside / name).read_bytes() for name in [EXE_NAME, DIGEST_NAME]},
            outside_before,
        )

    def test_linked_or_reparse_bundle_refuses_even_with_a_valid_pair(self):
        link = self.artifacts / "linked bundle"
        if os.name == "nt":
            # Junction fixture creation needs no symlink privilege. Values are data.
            environment = os.environ.copy()
            environment["COVENANT_TEST_LINK"] = str(link)
            environment["COVENANT_TEST_TARGET"] = str(self.bundle)
            result = subprocess.run(
                [
                    "powershell",
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:COVENANT_TEST_LINK -Target $env:COVENANT_TEST_TARGET | Out-Null",
                ],
                env=environment,
                capture_output=True,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
        else:
            link.symlink_to(self.bundle, target_is_directory=True)
        self.assertEqual((link / EXE_NAME).read_bytes(), self.payload)
        self.assert_refused(self.invoke(link))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--verifier", type=Path)
    args, remaining = parser.parse_known_args()
    if args.verifier:
        VERIFIER = args.verifier
    unittest.main(argv=[sys.argv[0], *remaining])
