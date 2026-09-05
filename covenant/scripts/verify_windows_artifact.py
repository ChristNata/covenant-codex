"""Verify development artifact bytes and their digest; this is not attestation."""

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import stat
import sys


# Load the fixed sibling helper for CLI and importlib callers without changing
# sys.path or accepting a caller-selected validation implementation.
_spec = importlib.util.spec_from_file_location(
    "_covenant_windows_build", Path(__file__).with_name("build_windows.py")
)
_builder = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_builder)

EXE_NAME = "codex-x86_64-pc-windows-msvc.exe"
DIGEST_NAME = "codex.exe.sha256"


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", required=True, type=Path)
    parser.add_argument("--artifact-dir", required=True, type=Path)
    args = parser.parse_args(argv)
    try:
        repo = args.repo.absolute()
        _builder.checked_path(repo, repo, "directory")
        directory = args.artifact_dir.absolute()
        artifacts = repo / "covenant/windows-build/artifacts"
        if directory == artifacts or not directory.is_relative_to(artifacts):
            raise _builder.BuildError("artifact directory containment")
        _builder.checked_path(repo, directory, "directory")
        executable = _builder.checked_path(repo, directory / EXE_NAME, "file")
        digest_file = _builder.checked_path(repo, directory / DIGEST_NAME, "file")

        with digest_file.open("rb") as stream:
            if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
                raise _builder.BuildError("nonregular digest file")
            expected = stream.read(66)
        if re.fullmatch(rb"[0-9a-f]{64}\n", expected) is None:
            raise _builder.BuildError("digest file format")

        digest = hashlib.sha256()
        size = 0
        with executable.open("rb") as stream:
            if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
                raise _builder.BuildError("nonregular executable")
            while chunk := stream.read(1024 * 1024):
                digest.update(chunk)
                size += len(chunk)
        if size == 0:
            raise _builder.BuildError("empty executable")
        checksum = digest.hexdigest()
        if checksum.encode("ascii") + b"\n" != expected:
            raise _builder.BuildError("artifact digest mismatch")

        print(
            json.dumps(
                {
                    "status": "artifact-pair-verified",
                    "artifact": {
                        "name": EXE_NAME,
                        "size_bytes": size,
                        "sha256": checksum,
                    },
                    "digest_file": DIGEST_NAME,
                }
            )
        )
        return 0
    except _builder.BuildError as error:
        print(
            f"artifact pair refused ({error}); remediation: verify the contained regular executable and exact digest file",
            file=sys.stderr,
        )
        return 1
    except (OSError, ValueError, TypeError, AttributeError, RecursionError):
        print(
            "artifact pair input or I/O failure; remediation: verify readable local artifact files",
            file=sys.stderr,
        )
        return 1


if __name__ == "__main__":
    sys.exit(main())
