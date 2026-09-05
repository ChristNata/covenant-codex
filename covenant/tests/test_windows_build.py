"""F02 helper behavior; synthetic producer bytes are never product-build evidence."""

import argparse
from contextlib import redirect_stderr, redirect_stdout
import copy
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch


BUILDER = Path(__file__).resolve().parents[1] / "scripts" / "build_windows.py"


class WindowsBuildTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location(
            "covenant_windows_builder", BUILDER
        )
        cls.builder = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = cls.builder
        spec.loader.exec_module(cls.builder)

    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(prefix="covenant-build-")
        self.addCleanup(self.scratch.cleanup)
        self.repo = Path(self.scratch.name) / "repo with spaces"
        self.repo.mkdir()
        (self.repo / ".git").mkdir()
        self.recipe = self.repo / "covenant/windows-repro.toml"
        self.recipe.parent.mkdir()
        self.output = self.repo / "covenant/windows-build/artifacts/first build"
        self.output.parent.mkdir(parents=True)
        self.model = {
            "schema_version": 1,
            "upstream_commit": "a" * 40,
            "toolchain_file": "codex-rs/rust-toolchain.toml",
            "rust_channel": "1.95.0",
            "rustc_commit": "59807616e1fa2540724bfbac14d7976d7e4a3860",
            "rustc_date": "2026-04-14",
            "cargo_version": "1.95.0",
            "cargo_commit": "f2d3ce0bd7f24a49f8f72d9000448f8838c4e850",
            "cargo_date": "2026-03-21",
            "target": "x86_64-pc-windows-msvc",
            "manifest": "codex-rs/Cargo.toml",
            "package_manifest": "codex-rs/cli/Cargo.toml",
            "package": "codex-cli",
            "bin": "codex",
            "profile": "release",
            "target_dir": "covenant/windows-build/target",
            "runner": "windows-2022",
            "msvc": "windows-2022 VS Build Tools",
            "node": False,
            "repro": "digest-recorded",
            "digest": "sha256",
        }
        self.sources = {
            "covenant/UPSTREAM.toml": (
                f'commit = "{self.model["upstream_commit"]}"\n'
                'rust_toolchain_path = "codex-rs/rust-toolchain.toml"\n'
                'rust_channel = "1.95.0"\n'
            ),
            "codex-rs/rust-toolchain.toml": '[toolchain]\nchannel = "1.95.0"\n',
            "codex-rs/Cargo.toml": '[workspace]\nmembers = ["cli"]\n',
            "codex-rs/Cargo.lock": "version = 4\n# synthetic lock only\n",
            "codex-rs/cli/Cargo.toml": (
                '[package]\nname = "codex-cli"\nversion = "0.153.4"\n'
                '[[bin]]\nname = "codex"\npath = "src/main.rs"\n'
            ),
            "codex-rs/cli/src/main.rs": "// Synthetic identity anchor, never compiled.\n",
        }
        for name, content in self.sources.items():
            path = self.repo / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content, encoding="utf-8", newline="\n")
        self.save_recipe()
        self.target_exe = (
            self.repo
            / self.model["target_dir"]
            / self.model["target"]
            / "release/codex.exe"
        )
        self.package_id = (self.repo / "codex-rs/cli").as_uri() + "#codex-cli@0.153.4"
        self.payload = b"SYNTHETIC OUTPUT ONLY\x00\xff\x80\r\n\n"
        self.head = "b" * 40
        self.command_calls = []
        self.build_exit = 0
        self.produce = True
        self.message_change = None
        self.version_change = None
        self.raw_build_output = None

    def save_recipe(self):
        self.recipe.write_text(
            "".join(
                f"{key} = {json.dumps(value)}\n" for key, value in self.model.items()
            ),
            encoding="utf-8",
            newline="\n",
        )

    def build_argv(self):
        return [
            "cargo",
            "+1.95.0",
            "build",
            "--locked",
            "--release",
            "--manifest-path",
            str(self.repo / "codex-rs/Cargo.toml"),
            "--package",
            "codex-cli",
            "--bin",
            "codex",
            "--target",
            "x86_64-pc-windows-msvc",
            "--target-dir",
            str(self.repo / "covenant/windows-build/target"),
            "--message-format=json-render-diagnostics",
        ]

    def synthetic_runner(self, argv, *, cwd, limit):
        """A fake producer used only to exercise helper behavior, never Rust itself."""
        argv = list(argv)
        self.command_calls.append((argv, Path(cwd), limit))
        if argv[0] == "git":
            output = self.head + "\n"
        elif "pkgid" in argv:
            output = self.package_id + "\n"
        elif "build" in argv:
            if self.produce:
                self.target_exe.parent.mkdir(parents=True, exist_ok=True)
                self.target_exe.write_bytes(self.payload)
            messages = [
                {
                    "reason": "compiler-artifact",
                    "package_id": self.package_id,
                    "target": {
                        "kind": ["bin"],
                        "name": "codex",
                        "src_path": str(self.repo / "codex-rs/cli/src/main.rs"),
                    },
                    "executable": str(self.target_exe),
                    "fresh": False,
                },
                {"reason": "build-finished", "success": True},
            ]
            if self.message_change:
                self.message_change(messages)
            output = "\n".join(json.dumps(message) for message in messages) + "\n"
            if self.raw_build_output is not None:
                output = self.raw_build_output
            return subprocess.CompletedProcess(argv, self.build_exit, output.encode())
        else:
            name = argv[0]
            release = self.model["rust_channel" if name == "rustc" else "cargo_version"]
            commit = self.model[f"{name}_commit"]
            date = self.model[f"{name}_date"]
            output = (
                f"{name} {release} ({commit[:9]} {date})\nrelease: {release}\n"
                f"commit-hash: {commit}\ncommit-date: {date}\nhost: {self.model['target']}\n"
            )
            if self.version_change:
                output = self.version_change(name, output)
        return subprocess.CompletedProcess(argv, 0, output.encode())

    def invoke(self, *, check=False, output=None):
        argv = ["--repo", str(self.repo), "--recipe", str(self.recipe)]
        argv += ["--check"] if check else ["--output-dir", str(output or self.output)]
        stdout, stderr = io.StringIO(), io.StringIO()
        with patch.object(
            self.builder, "run_command", side_effect=self.synthetic_runner
        ):
            with redirect_stdout(stdout), redirect_stderr(stderr):
                result = self.builder.main(argv)
        return result, stdout.getvalue(), stderr.getvalue()

    def assert_refused(self, result):
        code, stdout, stderr = result
        self.assertEqual(code, 1, (stdout, stderr))
        self.assertIn("remediation:", stderr.lower())
        self.assertLessEqual(len(stderr.encode()), 4096)
        self.assertNotIn("Traceback", stderr)
        self.assertFalse((self.output / "codex.exe.sha256").exists())
        self.assertFalse((self.output / "build-output.json").exists())

    def test_check_resolves_source_bound_invocation_without_producing_artifacts(self):
        code, stdout, stderr = self.invoke(check=True)
        self.assertEqual(code, 0, stderr)
        self.assertEqual(
            json.loads(stdout),
            {
                "status": "recipe-validated",
                "argv": self.build_argv(),
                "artifact": None,
            },
        )
        self.assertEqual(self.command_calls, [])
        self.assertFalse(self.output.exists())

    def test_malformed_or_substituted_recipe_refuses_before_any_command(self):
        original = copy.deepcopy(self.model)
        for field, value in [
            ("schema_version", True),
            ("rustc_commit", "bad"),
            ("node", "false"),
            ("target", "x86_64-pc-windows-msvc;echo unsafe"),
            ("upstream_commit", "c" * 40),
            ("rust_channel", "1.94.0"),
            ("package", "unrelated-package"),
            ("bin", "alternate-bin"),
            ("attestation", "caller-supplied-promotion-proof"),
            ("rustc_date", "2026-99-99"),
            ("digest", "md5"),
        ]:
            with self.subTest(field=field):
                self.model = copy.deepcopy(original)
                self.model[field] = value
                self.save_recipe()
                self.assert_refused(self.invoke(check=True))
                self.assertEqual(self.command_calls, [])
        for content in ["[broken", "", "x" * (64 * 1024 + 1)]:
            with self.subTest(content_length=len(content)):
                self.recipe.write_text(content, encoding="utf-8")
                self.assert_refused(self.invoke(check=True))
                self.assertEqual(self.command_calls, [])

    def test_unsafe_missing_and_conflicting_source_paths_refuse(self):
        original = copy.deepcopy(self.model)
        for field, value in [
            ("manifest", "../outside.toml"),
            ("manifest", "C:/outside.toml"),
            ("manifest", "codex-rs/missing.toml"),
            ("toolchain_file", "codex-rs/cli/Cargo.toml"),
            ("package_manifest", "codex-rs"),
            ("target_dir", "covenant/UPSTREAM.toml"),
            ("target_dir", "../target"),
            ("manifest", "x" * 4097),
        ]:
            with self.subTest(field=field, value=value[:40]):
                self.model = copy.deepcopy(original)
                self.model[field] = value
                self.save_recipe()
                self.assert_refused(self.invoke(check=True))
                self.assertEqual(self.command_calls, [])

    def test_workspace_members_require_an_array_of_package_paths(self):
        manifest = self.repo / "codex-rs/Cargo.toml"
        for members in ['"cli"', '["cli", 42]', "{cli = true}"]:
            with self.subTest(members=members):
                manifest.write_text(
                    f"[workspace]\nmembers = {members}\n",
                    encoding="utf-8",
                    newline="\n",
                )
                self.assert_refused(self.invoke(check=True))
                self.assertEqual(self.command_calls, [])

    def test_existing_or_outside_output_refuses_without_deleting_existing_data(self):
        self.output.mkdir()
        sentinel = self.output / "keep.bin"
        sentinel.write_bytes(b"pre-existing unrelated output")
        self.assert_refused(self.invoke())
        self.assertEqual(sentinel.read_bytes(), b"pre-existing unrelated output")
        self.assert_refused(self.invoke(output=Path(self.scratch.name) / "outside"))
        self.assertEqual(self.command_calls, [])

    def test_failed_build_preserves_stale_exe_and_never_records_its_digest(self):
        self.target_exe.parent.mkdir(parents=True)
        self.target_exe.write_bytes(b"stale output must survive failure")
        self.produce = False
        self.build_exit = 23
        self.assert_refused(self.invoke())
        self.assertEqual(
            self.target_exe.read_bytes(), b"stale output must survive failure"
        )
        builds = [argv for argv, _, _ in self.command_calls if "build" in argv]
        self.assertEqual(builds, [self.build_argv()])

    def test_success_without_a_real_matching_artifact_refuses(self):
        self.produce = False
        self.assert_refused(self.invoke())
        self.target_exe.parent.mkdir(parents=True, exist_ok=True)
        self.target_exe.write_bytes(b"stale with no artifact message")
        self.raw_build_output = '{"reason":"build-finished","success":true}\n'
        self.assert_refused(self.invoke())
        self.raw_build_output = None
        self.produce = True
        for field, replacement in [
            ("package_id", "wrong-package"),
            ("executable", str(self.recipe)),
            (
                "target",
                {"kind": ["lib"], "name": "codex", "src_path": str(self.recipe)},
            ),
        ]:
            with self.subTest(field=field):
                self.message_change = lambda rows, field=field, value=replacement: rows[
                    0
                ].update({field: value})
                self.assert_refused(self.invoke())
        self.message_change = lambda rows: rows[1].update(success=False)
        self.assert_refused(self.invoke())
        self.message_change = lambda rows: rows.insert(1, copy.deepcopy(rows[0]))
        self.assert_refused(self.invoke())
        self.message_change = None
        self.payload = b""
        self.assert_refused(self.invoke())
        self.payload = b"synthetic output"
        self.raw_build_output = "not Cargo JSON\n"
        self.assert_refused(self.invoke())

    def test_exact_published_bytes_and_receipt_agree_without_promotion_claim(self):
        code, stdout, stderr = self.invoke()
        self.assertEqual(code, 0, stderr)
        digest = hashlib.sha256(self.payload).hexdigest()
        receipt = {
            "schema_version": 1,
            "status": "build-output-recorded",
            "artifact": {
                "name": "codex-x86_64-pc-windows-msvc.exe",
                "size_bytes": len(self.payload),
                "sha256": digest,
            },
            "upstream_commit": self.model["upstream_commit"],
            "observed_head": self.head,
            "recipe_sha256": hashlib.sha256(self.recipe.read_bytes()).hexdigest(),
            "lock_sha256": hashlib.sha256(
                (self.repo / "codex-rs/Cargo.lock").read_bytes()
            ).hexdigest(),
            "target": self.model["target"],
            "package": "codex-cli",
            "bin": "codex",
            "profile": "release",
            "argv": self.build_argv(),
            "repro": "digest-recorded",
            "runner": "windows-2022",
            "msvc": self.model["msvc"],
            "source_evidence": "working-tree-unattested",
        }
        for name in ["rustc", "cargo"]:
            receipt[name] = {
                "release": self.model[
                    "rust_channel" if name == "rustc" else "cargo_version"
                ],
                "commit": self.model[f"{name}_commit"],
                "date": self.model[f"{name}_date"],
                "host": self.model["target"],
            }
        self.assertEqual(json.loads(stdout), receipt)
        self.assertEqual(
            json.loads((self.output / "build-output.json").read_text()), receipt
        )
        self.assertEqual(
            (self.output / receipt["artifact"]["name"]).read_bytes(), self.payload
        )
        self.assertEqual(
            (self.output / "codex.exe.sha256").read_bytes(), (digest + "\n").encode()
        )
        builds = [(argv, cwd) for argv, cwd, _ in self.command_calls if "build" in argv]
        self.assertEqual(builds, [(self.build_argv(), self.repo / "codex-rs")])

    def test_toolchain_release_commit_date_and_host_must_match_before_build(self):
        for name in ["rustc", "cargo"]:
            for old, new in [
                ("release: 1.95.0", "release: 1.94.0"),
                (self.model[f"{name}_commit"], "d" * 40),
                (self.model[f"{name}_date"], "2020-01-01"),
                (self.model["target"], "aarch64-pc-windows-msvc"),
            ]:
                with self.subTest(tool=name, old=old):
                    self.command_calls.clear()
                    self.version_change = (
                        lambda tool, text, name=name, old=old, new=new: (
                            text.replace(old, new) if tool == name else text
                        )
                    )
                    self.assert_refused(self.invoke())
                    self.assertFalse(
                        any("build" in argv for argv, _, _ in self.command_calls)
                    )

    def test_command_adapter_preserves_literal_arguments_without_a_shell(self):
        literals = ["two words", 'quote"text', "$()", "& echo not-a-command", "世界"]
        code = "import json,sys; print(json.dumps(sys.argv[1:], ensure_ascii=True))"
        result = self.builder.run_command(
            [sys.executable, "-c", code, *literals],
            cwd=self.repo,
            limit=4096,
        )
        self.assertEqual(result.returncode, 0)
        self.assertEqual(json.loads(result.stdout), literals)

    def test_command_adapter_refuses_oversize_output_after_child_completion(self):
        marker = self.repo / "child-completed"
        code = "import pathlib,sys; print('x'*5000); pathlib.Path(sys.argv[1]).write_text('done')"
        with self.assertRaisesRegex(self.builder.BuildError, "(?i)(output|limit)"):
            self.builder.run_command(
                [sys.executable, "-c", code, str(marker)],
                cwd=self.repo,
                limit=128,
            )
        self.assertEqual(marker.read_text(), "done")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--builder", type=Path)
    args, remaining = parser.parse_known_args()
    if args.builder:
        BUILDER = args.builder
    unittest.main(argv=[sys.argv[0], *remaining])
