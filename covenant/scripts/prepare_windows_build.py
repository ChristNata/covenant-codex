"""Select a VS2022 x64 environment; this does not build or attest an artifact."""

import argparse
from collections.abc import Mapping
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import stat
import sys


_spec = importlib.util.spec_from_file_location(
    "_covenant_windows_build", Path(__file__).with_name("build_windows.py")
)
_builder = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_builder)
BuildError = _builder.BuildError
run_command = _builder.run_command
QUERY_LIMIT = _builder.QUERY_LIMIT
FAILURE = (
    "Windows build preparation refused; remediation: verify VS2022 x64 tools and SDK"
)
EXPORT_KEYS = (
    "INCLUDE",
    "LIB",
    "LIBPATH",
    "PATH",
    "UCRTVersion",
    "UniversalCRTSdkDir",
    "VCINSTALLDIR",
    "VCToolsInstallDir",
    "WindowsLibPath",
    "WindowsSdkBinPath",
    "WindowsSdkDir",
    "WindowsSDKLibVersion",
    "WindowsSDKVersion",
)
METADATA_KEYS = ("VCToolsVersion", "VSCMD_ARG_HOST_ARCH", "VSCMD_ARG_TGT_ARCH")


class _Parser(argparse.ArgumentParser):
    def error(self, message):
        raise BuildError("arguments")


def _query(argv, cwd):
    result = run_command(argv, cwd=cwd, limit=QUERY_LIMIT)
    if (
        result.returncode != 0
        or not isinstance(result.stdout, bytes)
        or len(result.stdout) > QUERY_LIMIT
    ):
        raise BuildError("native query")
    return result.stdout


def _prepare_msvc(repo: Path, environment: Mapping[str, str]) -> dict:
    """Read selection inputs; the native adapter still inherits the parent environment."""
    try:
        repo = repo.absolute()
        _builder.checked_path(repo, repo, "directory")
        cargo_root = _builder.checked_path(repo, repo / "codex-rs", "directory")
        program_files = environment.get("ProgramFiles(x86)")
        if not isinstance(program_files, str) or not program_files:
            raise BuildError("Visual Studio discovery path")
        program_files = Path(program_files)
        vswhere = _builder.checked_path(
            program_files,
            program_files / "Microsoft Visual Studio/Installer/vswhere.exe",
            "file",
        )
        candidates = json.loads(
            _query(
                [
                    str(vswhere),
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
                ],
                cargo_root,
            ).decode("utf-8-sig")
        )
        if not isinstance(candidates, list) or len(candidates) != 1:
            raise BuildError("Visual Studio selection")
        candidate = candidates[0]
        if not isinstance(candidate, dict):
            raise BuildError("Visual Studio candidate")
        version = candidate.get("installationVersion")
        installation = candidate.get("installationPath")
        if (
            not isinstance(version, str)
            or re.fullmatch(r"17\.[0-9]+\.[0-9]+\.[0-9]+", version) is None
            or not isinstance(installation, str)
            or not installation
            or candidate.get("isComplete") is not True
            or candidate.get("isLaunchable") is not True
            or any(character in installation for character in "&%!^|<>")
        ):
            raise BuildError("Visual Studio candidate")
        installation = Path(installation)
        batch = _builder.checked_path(
            installation, installation / "Common7/Tools/VsDevCmd.bat", "file"
        )
        output = _query(
            [
                "cmd.exe",
                "/d",
                "/u",
                "/s",
                "/c",
                r".\VsDevCmd.bat -no_logo -arch=x64 -host_arch=x64 >nul && set",
            ],
            batch.parent,
        ).decode("utf-16le")
        required = {key.upper(): key for key in (*EXPORT_KEYS, *METADATA_KEYS)}
        selected = {}
        for line in output.split("\r\n"):
            if not line:
                continue
            name, separator, value = line.partition("=")
            if not separator or not name or any(char in line for char in "\r\n\0"):
                raise BuildError("environment output")
            canonical = required.get(name.upper())
            if canonical is not None:
                if canonical in selected or not value:
                    raise BuildError("required environment value")
                selected[canonical] = value
        if set(selected) != set(required.values()):
            raise BuildError("missing environment value")
        if (
            selected["VSCMD_ARG_HOST_ARCH"] != "x64"
            or selected["VSCMD_ARG_TGT_ARCH"] != "x64"
        ):
            raise BuildError("MSVC architecture")
        tools_version = selected["VCToolsVersion"]
        sdk_version = selected["WindowsSDKVersion"].removesuffix("\\")
        if (
            re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", tools_version) is None
            or re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+", sdk_version) is None
        ):
            raise BuildError("native tool version")
        tools = installation / "VC/Tools/MSVC" / tools_version
        if (
            Path(selected["VCToolsInstallDir"]) != tools
            or Path(selected["VCINSTALLDIR"]) != installation / "VC"
        ):
            raise BuildError("selected MSVC location")
        linker = _builder.checked_path(
            installation, tools / "bin/Hostx64/x64/link.exe", "file"
        )
        digest = hashlib.sha256()
        size = 0
        with linker.open("rb") as stream:
            if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
                raise BuildError("nonregular linker")
            while chunk := stream.read(1024 * 1024):
                digest.update(chunk)
                size += len(chunk)
        if size == 0:
            raise BuildError("empty linker")
        exported = {key: selected[key] for key in EXPORT_KEYS}
        exported["CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER"] = str(linker)
        return {
            "status": "msvc-environment-prepared",
            "environment": exported,
            "observations": {
                "vs_version": version,
                "msvc_version": tools_version,
                "sdk_version": sdk_version,
                "linker_path": str(linker),
                "linker_sha256": digest.hexdigest(),
            },
        }
    except (OSError, ValueError, TypeError, RecursionError):
        raise BuildError("MSVC preparation input") from None


def main(argv=None) -> int:
    parser = _Parser(description=__doc__)
    parser.add_argument("--repo", required=True, type=Path)
    try:
        args = parser.parse_args(argv)
        result = _prepare_msvc(args.repo, os.environ)
    except BuildError:
        print(FAILURE, file=sys.stderr)
        return 1
    print(json.dumps(result))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
