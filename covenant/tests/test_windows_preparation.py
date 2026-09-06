"""MSVC discovery fixtures exercise owner logic, never a real tool installation.

The injected mapping is discovery input, not the native child's environment.
The reused adapter inherits stderr; these tests do not claim child-output secrecy.
"""

from contextlib import redirect_stderr, redirect_stdout
import copy
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch


HELPER = Path(__file__).resolve().parents[1] / "scripts/prepare_windows_build.py"
FAILURE = (
    "Windows build preparation refused; remediation: verify VS2022 x64 tools and SDK\n"
)
CANARY = "PRIVATE-DISCOVERY-CANARY"
QUERY_LIMIT = 64 * 1024


class WindowsPreparationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location("covenant_msvc_setup", HELPER)
        cls.setup_module = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = cls.setup_module
        spec.loader.exec_module(cls.setup_module)

    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(prefix="covenant-msvc-")
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name)
        self.repo = self.root / "repo with spaces"
        (self.repo / "codex-rs").mkdir(parents=True)
        self.program_files = self.root / "Program Files (x86)"
        self.vswhere = (
            self.program_files / "Microsoft Visual Studio/Installer/vswhere.exe"
        )
        self.vswhere.parent.mkdir(parents=True)
        self.vswhere.write_bytes(b"SYNTHETIC QUERY FILE, NEVER EXECUTED")
        self.discovery = {
            "ProgramFiles(x86)": str(self.program_files),
            "PRIVATE_PARENT": CANARY,
        }
        self.install = self.root / "Visual Studio 2022 Ω with spaces"
        self.batch = self.install / "Common7/Tools/VsDevCmd.bat"
        self.tools = self.install / "VC/Tools/MSVC/14.42.34433"
        self.linker = self.tools / "bin/Hostx64/x64/link.exe"
        self.payload = b"SYNTHETIC LINKER ONLY\x00\xff\r\n"
        for path, content in [
            (self.batch, b"REM synthetic batch, never executed\r\n"),
            (self.linker, self.payload),
            (self.install / "VC/Tools/Llvm/x64/bin/lld-link.exe", b"not selected"),
            (self.repo / "keep.bin", b"existing unrelated bytes"),
        ]:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content)
        sdk = self.root / "Windows Kits/10"
        sdk.mkdir(parents=True)
        self.values = {
            "INCLUDE": str(self.tools / "include"),
            "LIB": str(self.tools / "lib/x64"),
            "LIBPATH": str(self.tools / "lib/x64") + ";" + str(sdk / "UnionMetadata"),
            "PATH": str(self.tools / "bin/Hostx64/x64") + ";C:\\Windows\\System32",
            "UCRTVersion": "10.0.22621.0",
            "UniversalCRTSdkDir": str(sdk),
            "VCINSTALLDIR": str(self.install / "VC"),
            "VCToolsInstallDir": str(self.tools),
            "WindowsLibPath": str(sdk / "UnionMetadata"),
            "WindowsSdkBinPath": str(sdk / "bin"),
            "WindowsSdkDir": str(sdk),
            "WindowsSDKLibVersion": "10.0.22621.0",
            "WindowsSDKVersion": "10.0.22621.0",
        }
        self.metadata = {
            "VCToolsVersion": "14.42.34433",
            "VSCMD_ARG_HOST_ARCH": "x64",
            "VSCMD_ARG_TGT_ARCH": "x64",
        }
        self.candidates = [
            {
                "installationPath": str(self.install),
                "installationVersion": "17.12.35527.113",
                "isComplete": True,
                "isLaunchable": True,
            }
        ]
        self.calls = []
        self.query_exit = 0
        self.batch_exit = 0
        self.query_bytes = None
        self.environment_bytes = None

    def query_argv(self):
        return [
            str(self.vswhere),
            "-latest",
            "-products",
            "*",
            "-version",
            "[17.0,18.0)",
            "-requires",
            "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
            "-format",
            "json",
            "-utf8",
        ]

    def batch_argv(self):
        return [
            "cmd.exe",
            "/d",
            "/u",
            "/s",
            "/c",
            r".\VsDevCmd.bat -no_logo -arch=x64 -host_arch=x64 >nul && set",
        ]

    def environment_output(self):
        values = {
            **self.values,
            **self.metadata,
            "UNSELECTED_SECRET": CANARY,
            "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER": "unselected-linker.exe",
        }
        return "".join(f"{name}={value}\r\n" for name, value in values.items()).encode(
            "utf-16le"
        )

    def native_query(self, argv, *, cwd, limit):
        argv = list(argv)
        self.calls.append((argv, Path(cwd), limit))
        self.assertEqual(limit, QUERY_LIMIT)
        if argv[0] == str(self.vswhere):
            self.assertEqual(
                (argv, Path(cwd)), (self.query_argv(), self.repo / "codex-rs")
            )
            output = self.query_bytes
            if output is None:
                output = json.dumps(self.candidates, ensure_ascii=False).encode("utf-8")
            returncode = self.query_exit
        else:
            self.assertEqual((argv, Path(cwd)), (self.batch_argv(), self.batch.parent))
            output = self.environment_bytes
            if output is None:
                output = self.environment_output()
            returncode = self.batch_exit
        if len(output) > limit:
            raise self.setup_module.BuildError("command output limit")
        return subprocess.CompletedProcess(argv, returncode, output)

    def snapshot(self):
        return {
            str(path.relative_to(self.root)): (
                ("file", path.read_bytes()) if path.is_file() else ("directory",)
            )
            for path in self.root.rglob("*")
        }

    def invoke(self):
        before = self.snapshot()
        mapping = copy.deepcopy(self.discovery)
        stdout, stderr = io.StringIO(), io.StringIO()
        self.calls.clear()
        with patch.object(
            self.setup_module, "run_command", side_effect=self.native_query
        ):
            try:
                with redirect_stdout(stdout), redirect_stderr(stderr):
                    return self.setup_module._prepare_msvc(self.repo, self.discovery)
            finally:
                self.assertEqual(self.snapshot(), before)
                self.assertEqual(self.discovery, mapping)
                self.assertEqual((stdout.getvalue(), stderr.getvalue()), ("", ""))

    def refused(self, calls):
        with self.assertRaises(self.setup_module.BuildError) as failure:
            self.invoke()
        self.assertLessEqual(len(str(failure.exception).encode("utf-8")), 256)
        self.assertNotIn(CANARY, str(failure.exception))
        self.assertEqual(len(self.calls), calls)

    def test_valid_selection_preserves_complete_output_and_inputs(self):
        expected = {
            "status": "msvc-environment-prepared",
            "environment": {
                **self.values,
                "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER": str(self.linker),
            },
            "observations": {
                "vs_version": "17.12.35527.113",
                "msvc_version": "14.42.34433",
                "sdk_version": "10.0.22621.0",
                "linker_path": str(self.linker),
                "linker_sha256": hashlib.sha256(self.payload).hexdigest(),
            },
        }
        actual = self.invoke()
        self.assertEqual(actual, expected)
        self.assertNotIn(CANARY, json.dumps(actual))
        self.assertEqual(
            self.calls,
            [
                (self.query_argv(), self.repo / "codex-rs", QUERY_LIMIT),
                (self.batch_argv(), self.batch.parent, QUERY_LIMIT),
            ],
        )
        self.linker.write_bytes(self.payload + b"changed fixture bytes")
        expected["observations"]["linker_sha256"] = hashlib.sha256(
            self.payload + b"changed fixture bytes"
        ).hexdigest()
        self.assertEqual(self.invoke(), expected)

    def test_missing_ambiguous_wrong_version_and_malformed_candidates_refuse(self):
        original = copy.deepcopy(self.candidates)
        for candidates in [
            [],
            original * 2,
            [{**original[0], "installationVersion": "18.0.0.0"}],
            [{**original[0], "installationVersion": "170.1.0"}],
            [{**original[0], "installationVersion": "17.bad"}],
            [{"installationVersion": "17.12.35527.113"}],
            {"installationPath": str(self.install)},
        ]:
            with self.subTest(candidates=candidates):
                self.candidates = candidates
                self.refused(1)
        self.candidates = original
        for output in [
            b"not JSON",
            b"\xff",
            b"null",
            b"[" + b" " * QUERY_LIMIT,
            b"[" * 2000 + b"]" * 2000,
        ]:
            with self.subTest(length=len(output)):
                self.query_bytes = output
                self.refused(1)

    def test_failed_queries_and_malformed_environment_refuse(self):
        self.query_exit = 7
        self.refused(1)
        self.query_exit = 0
        self.batch_exit = 23
        self.refused(2)
        self.batch_exit = 0
        valid = self.environment_output()
        for output in [
            b"",
            b"\x00",
            b"\x00\xd8",
            "malformed line\r\n".encode("utf-16le") + valid,
            valid + "pAtH=duplicate\r\n".encode("utf-16le"),
            valid + "VCToolsVersion=duplicate\r\n".encode("utf-16le"),
            b"x" * (QUERY_LIMIT + 1),
        ]:
            with self.subTest(length=len(output)):
                self.environment_bytes = output
                self.refused(2)

    def test_required_values_and_x64_architectures_cannot_be_missing_or_substituted(
        self,
    ):
        original_values, original_metadata = self.values.copy(), self.metadata.copy()
        for name in [*original_values, *original_metadata]:
            for replacement in [None, ""]:
                with self.subTest(name=name, replacement=replacement):
                    self.values, self.metadata = (
                        original_values.copy(),
                        original_metadata.copy(),
                    )
                    target = self.values if name in self.values else self.metadata
                    if replacement is None:
                        del target[name]
                    else:
                        target[name] = replacement
                    self.refused(2)
        self.values, self.metadata = original_values.copy(), original_metadata.copy()
        for name in ["VSCMD_ARG_HOST_ARCH", "VSCMD_ARG_TGT_ARCH"]:
            with self.subTest(name=name):
                self.metadata[name] = "arm64"
                self.refused(2)
                self.metadata[name] = "x64"
        self.values["PATH"] += "\x00bad"
        self.refused(2)

    def test_missing_or_outside_linker_never_falls_back(self):
        self.linker.unlink()
        self.refused(2)
        self.linker.mkdir()
        self.refused(2)
        self.linker.rmdir()
        self.linker.write_bytes(self.payload)
        outside = self.root / "other VS/VC/Tools/MSVC/14.42.34433"
        alternative = outside / "bin/Hostx64/x64/link.exe"
        alternative.parent.mkdir(parents=True)
        alternative.write_bytes(self.payload)
        self.values["VCToolsInstallDir"] = str(outside)
        self.refused(2)

    def test_command_expansion_paths_refuse_before_batch_execution(self):
        original_install = self.install
        original_values = self.values.copy()
        for hazard in ["&", "%", "!", "^"]:
            with self.subTest(hazard=hazard):
                candidate = self.root / f"VS{hazard}unsafe"
                shutil.copytree(original_install, candidate)
                self.install = candidate
                self.batch = candidate / "Common7/Tools/VsDevCmd.bat"
                self.candidates[0]["installationPath"] = str(candidate)
                self.values = {
                    name: value.replace(str(original_install), str(candidate))
                    for name, value in original_values.items()
                }
                self.assertTrue(self.batch.is_file())
                self.assertTrue(
                    (
                        candidate / "VC/Tools/MSVC/14.42.34433/bin/Hostx64/x64/link.exe"
                    ).is_file()
                )
                self.refused(1)

    def test_cli_missing_repo_has_only_the_fixed_owned_error(self):
        before = self.snapshot()
        stdout, stderr = io.StringIO(), io.StringIO()
        with patch.object(self.setup_module, "run_command") as native:
            with redirect_stdout(stdout), redirect_stderr(stderr):
                code = self.setup_module.main(
                    ["--repo", str(self.root / "missing repo")]
                )
        self.assertEqual((code, stdout.getvalue(), stderr.getvalue()), (1, "", FAILURE))
        native.assert_not_called()
        self.assertEqual(self.snapshot(), before)


if __name__ == "__main__":
    unittest.main()
