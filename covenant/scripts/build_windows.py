"""Record Cargo-reported Windows build bytes; this is not release attestation."""

import argparse
from datetime import date
import hashlib
import json
import os
from pathlib import Path, PureWindowsPath
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import tomllib


QUERY_LIMIT = 64 * 1024
BUILD_LIMIT = 64 * 1024 * 1024
FIXED = {
    "schema_version": 1,
    "target": "x86_64-pc-windows-msvc",
    "manifest": "codex-rs/Cargo.toml",
    "package_manifest": "codex-rs/cli/Cargo.toml",
    "package": "codex-cli",
    "bin": "codex",
    "profile": "release",
    "target_dir": "covenant/windows-build/target",
    "runner": "windows-2022",
    "node": False,
    "repro": "digest-recorded",
    "digest": "sha256",
}
VARIABLE = {
    "upstream_commit",
    "toolchain_file",
    "rust_channel",
    "rustc_commit",
    "rustc_date",
    "cargo_version",
    "cargo_commit",
    "cargo_date",
    "msvc",
}


class BuildError(Exception):
    """A bounded refusal category without submitted values or command output."""


def run_command(argv, *, cwd: Path, limit: int) -> subprocess.CompletedProcess:
    """Drain output through completion, retaining at most limit bytes, without a shell."""
    if type(limit) is not int or limit < 1:
        raise BuildError("output limit")
    env = None
    if argv[0] == "git":
        env = {
            key: value
            for key, value in os.environ.items()
            if not key.upper().startswith("GIT_")
        }
        env.update(
            GIT_CONFIG_NOSYSTEM="1",
            GIT_CONFIG_GLOBAL=os.devnull,
            GIT_TERMINAL_PROMPT="0",
            GIT_NO_LAZY_FETCH="1",
            GIT_GRAFT_FILE=os.devnull,
        )
    output = bytearray()
    overflow = False
    try:
        with subprocess.Popen(
            list(argv),
            cwd=cwd,
            env=env,
            shell=False,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
        ) as child:
            while chunk := child.stdout.read(64 * 1024):
                remaining = limit - len(output)
                output.extend(chunk[:remaining])
                overflow |= len(chunk) > remaining
            returncode = child.wait()
    except OSError:
        raise BuildError("command execution") from None
    if overflow:
        raise BuildError("command output limit")
    return subprocess.CompletedProcess(list(argv), returncode, bytes(output))


def checked_path(root, path, kind):
    """Reject links and reparse points in every component, including root ancestors."""
    path = Path(path)
    if (
        not path.is_absolute()
        or not path.is_relative_to(root)
        or len(str(path).encode("utf-8")) > 4096
        or any(
            part == ".."
            or part.endswith((" ", "."))
            or any(ord(char) < 32 or char in '<>:"|?*' for char in part)
            or PureWindowsPath(part).is_reserved()
            for part in path.parts[1:]
        )
    ):
        raise BuildError("path containment")
    for component in (*reversed(path.parents), path):
        try:
            info = component.lstat()
        except FileNotFoundError:
            if kind in ("absent", "directory-or-absent"):
                continue
            raise BuildError("missing input path") from None
        if (
            stat.S_ISLNK(info.st_mode)
            or getattr(info, "st_file_attributes", 0)
            & stat.FILE_ATTRIBUTE_REPARSE_POINT
        ):
            raise BuildError("linked or reparse path")
        expected_file = component == path and kind == "file"
        if not (
            stat.S_ISREG(info.st_mode) if expected_file else stat.S_ISDIR(info.st_mode)
        ):
            raise BuildError("conflicting path type")
        if component == path and kind == "absent":
            raise BuildError("existing output")
    return path


def relative_path(root, value, kind):
    if (
        not isinstance(value, str)
        or not value
        or len(value.encode("utf-8")) > 4096
        or "\\" in value
        or ":" in value
        or PureWindowsPath(value).is_absolute()
    ):
        raise BuildError("relative source path")
    parts = value.split("/")
    if any(
        part in ("", ".", "..")
        or part.endswith((" ", "."))
        or any(ord(char) < 32 or char in '<>"|?*' for char in part)
        or PureWindowsPath(part).is_reserved()
        for part in parts
    ):
        raise BuildError("relative source path")
    return checked_path(root, root.joinpath(*parts), kind)


def load_toml(path):
    with path.open("rb") as stream:
        raw = stream.read(QUERY_LIMIT + 1)
    if len(raw) > QUERY_LIMIT:
        raise BuildError("TOML input limit")
    return tomllib.loads(raw.decode("utf-8")), raw


def validate_recipe(repo, recipe_path):
    recipe, raw = load_toml(checked_path(repo, recipe_path, "file"))
    if set(recipe) != set(FIXED) | VARIABLE:
        raise BuildError("recipe fields")
    for key, expected in FIXED.items():
        if type(recipe[key]) is not type(expected) or recipe[key] != expected:
            raise BuildError("recipe fixed field")
    for key in VARIABLE:
        value = recipe[key]
        if (
            not isinstance(value, str)
            or not value
            or len(value.encode("utf-8")) > 4096
            or any(ord(char) < 32 for char in value)
        ):
            raise BuildError("recipe string field")
    for key in ("upstream_commit", "rustc_commit", "cargo_commit"):
        if not re.fullmatch(r"[0-9a-f]{40}", recipe[key]):
            raise BuildError("recipe commit identity")
    for key in ("rust_channel", "cargo_version"):
        if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", recipe[key]):
            raise BuildError("recipe tool version")
    for key in ("rustc_date", "cargo_date"):
        if date.fromisoformat(recipe[key]).isoformat() != recipe[key]:
            raise BuildError("recipe tool date")
    upstream, _ = load_toml(relative_path(repo, "covenant/UPSTREAM.toml", "file"))
    if (
        upstream.get("commit") != recipe["upstream_commit"]
        or upstream.get("rust_toolchain_path") != recipe["toolchain_file"]
        or upstream.get("rust_channel") != recipe["rust_channel"]
        or recipe["cargo_version"] != recipe["rust_channel"]
    ):
        raise BuildError("bootstrap binding")
    toolchain, _ = load_toml(relative_path(repo, recipe["toolchain_file"], "file"))
    if toolchain.get("toolchain", {}).get("channel") != recipe["rust_channel"]:
        raise BuildError("toolchain binding")
    workspace, _ = load_toml(relative_path(repo, recipe["manifest"], "file"))
    workspace_table = workspace.get("workspace")
    if not isinstance(workspace_table, dict):
        raise BuildError("workspace declaration")
    members = workspace_table.get("members")
    if (
        not isinstance(members, list)
        or any(not isinstance(member, str) for member in members)
        or "cli" not in members
    ):
        raise BuildError("workspace package membership")
    package_path = relative_path(repo, recipe["package_manifest"], "file")
    package, _ = load_toml(package_path)
    if package.get("package", {}).get("name") != recipe["package"]:
        raise BuildError("package declaration")
    bins = package.get("bin", [])
    if not isinstance(bins, list) or any(not isinstance(item, dict) for item in bins):
        raise BuildError("binary declaration")
    selected = [item for item in bins if item.get("name") == recipe["bin"]]
    if len(selected) != 1 or selected[0].get("path") != "src/main.rs":
        raise BuildError("binary source declaration")
    source = relative_path(repo, "codex-rs/cli/src/main.rs", "file")
    relative_path(repo, recipe["target_dir"], "directory-or-absent")
    lock = relative_path(repo, "codex-rs/Cargo.lock", "file")
    argv = [
        "cargo",
        "+" + recipe["rust_channel"],
        "build",
        "--locked",
        "--release",
        "--manifest-path",
        str(repo / recipe["manifest"]),
        "--package",
        recipe["package"],
        "--bin",
        recipe["bin"],
        "--target",
        recipe["target"],
        "--target-dir",
        str(repo / recipe["target_dir"]),
        "--message-format=json-render-diagnostics",
    ]
    return recipe, hashlib.sha256(raw).hexdigest(), lock, source, argv


def command_output(argv, cwd, limit=QUERY_LIMIT):
    result = run_command(argv, cwd=cwd, limit=limit)
    if result.returncode != 0:
        raise BuildError("command failed")
    if not isinstance(result.stdout, bytes) or len(result.stdout) > limit:
        raise BuildError("command output limit")
    return result.stdout.decode("utf-8")


def tool_identity(name, recipe, cwd):
    output = command_output(
        [name, "+" + recipe["rust_channel"], "-vV" if name == "rustc" else "-Vv"], cwd
    )
    fields = {}
    for line in output.splitlines():
        key, separator, value = line.partition(": ")
        if separator and key in ("release", "commit-hash", "commit-date", "host"):
            if key in fields:
                raise BuildError("duplicate tool identity")
            fields[key] = value
    expected = {
        "release": recipe["rust_channel" if name == "rustc" else "cargo_version"],
        "commit-hash": recipe[name + "_commit"],
        "commit-date": recipe[name + "_date"],
        "host": recipe["target"],
    }
    if fields != expected:
        raise BuildError("toolchain identity mismatch")
    return {
        "release": fields["release"],
        "commit": fields["commit-hash"],
        "date": fields["commit-date"],
        "host": fields["host"],
    }


def json_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise BuildError("duplicate Cargo JSON field")
        result[key] = value
    return result


def reject_constant(_value):
    raise BuildError("non-JSON Cargo number")


def select_artifact(output, repo, recipe, package_id, source):
    executable = repo / recipe["target_dir"] / recipe["target"] / "release/codex.exe"
    finished = False
    matches = 0
    for line in output.splitlines():
        if not line:
            continue
        if finished:
            raise BuildError("Cargo event after final result")
        event = json.loads(
            line, object_pairs_hook=json_object, parse_constant=reject_constant
        )
        if not isinstance(event, dict) or not isinstance(event.get("reason"), str):
            raise BuildError("Cargo event shape")
        if event["reason"] == "build-finished":
            if event.get("success") is not True:
                raise BuildError("Cargo final result")
            finished = True
        elif event["reason"] == "compiler-artifact":
            target = event.get("target")
            if not isinstance(target, dict):
                raise BuildError("Cargo artifact target")
            if (
                event.get("package_id") != package_id
                or target.get("name") != recipe["bin"]
            ):
                continue
            if (
                target.get("kind") != ["bin"]
                or not isinstance(target.get("src_path"), str)
                or Path(target["src_path"]) != source
                or not isinstance(event.get("executable"), str)
                or Path(event["executable"]) != executable
            ):
                raise BuildError("Cargo artifact identity")
            matches += 1
    if not finished or matches != 1:
        raise BuildError("missing or ambiguous Cargo artifact")
    checked_path(repo, executable, "file")
    if executable.stat().st_size == 0:
        raise BuildError("empty Cargo artifact")
    return executable


def publish(repo, output, executable, receipt):
    checked_path(repo, output.parent, "directory")
    checked_path(repo, output, "absent")
    staging = Path(tempfile.mkdtemp(prefix=".covenant-build-", dir=output.parent))
    try:
        checked_path(repo, staging, "directory")
        name = "codex-x86_64-pc-windows-msvc.exe"
        digest = hashlib.sha256()
        size = 0
        checked_path(repo, executable, "file")
        with (
            executable.open("rb") as source,
            (staging / name).open("xb") as destination,
        ):
            if not stat.S_ISREG(os.fstat(source.fileno()).st_mode):
                raise BuildError("nonregular Cargo artifact")
            while chunk := source.read(1024 * 1024):
                destination.write(chunk)
                digest.update(chunk)
                size += len(chunk)
        if size == 0:
            raise BuildError("empty copied artifact")
        receipt["artifact"] = {
            "name": name,
            "size_bytes": size,
            "sha256": digest.hexdigest(),
        }
        serialized = json.dumps(receipt, sort_keys=True) + "\n"
        (staging / "codex.exe.sha256").write_bytes(
            (digest.hexdigest() + "\n").encode("ascii")
        )
        (staging / "build-output.json").write_bytes(serialized.encode("utf-8"))
        checked_path(repo, staging, "directory")
        checked_path(repo, output, "absent")
        # Build mode is Windows-only: rename refuses even an existing empty directory.
        staging.rename(output)
        return serialized
    finally:
        if os.path.lexists(staging):
            checked_path(repo, staging, "directory")
            shutil.rmtree(staging)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", required=True, type=Path)
    parser.add_argument("--recipe", required=True, type=Path)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--output-dir", type=Path)
    args = parser.parse_args(argv)
    try:
        repo = args.repo.absolute()
        checked_path(repo, repo, "directory")
        recipe_path = args.recipe.absolute()
        recipe, recipe_hash, lock, source, build_argv = validate_recipe(
            repo, recipe_path
        )
        if args.check:
            print(
                json.dumps(
                    {"status": "recipe-validated", "argv": build_argv, "artifact": None}
                )
            )
            return 0
        if os.name != "nt":
            raise BuildError("Windows build platform required")
        output = args.output_dir.absolute()
        artifacts = repo / "covenant/windows-build/artifacts"
        if output == artifacts or not output.is_relative_to(artifacts):
            raise BuildError("output containment")
        checked_path(repo, output.parent, "directory")
        checked_path(repo, output, "absent")
        cwd = repo / "codex-rs"
        rustc = tool_identity("rustc", recipe, cwd)
        cargo = tool_identity("cargo", recipe, cwd)
        package_id = command_output(
            [
                "cargo",
                "+" + recipe["rust_channel"],
                "pkgid",
                "--locked",
                "--manifest-path",
                str(repo / recipe["manifest"]),
                "--package",
                recipe["package"],
            ],
            cwd,
        ).strip()
        if (
            not package_id
            or len(package_id.encode("utf-8")) > 4096
            or any(char.isspace() for char in package_id)
        ):
            raise BuildError("package identity output")
        head = command_output(
            [
                "git",
                "--no-replace-objects",
                "--no-optional-locks",
                "-c",
                "protocol.allow=never",
                "-C",
                str(repo),
                "rev-parse",
                "--verify",
                "HEAD^{commit}",
            ],
            repo,
        ).strip()
        if not re.fullmatch(r"[0-9a-f]{40}", head):
            raise BuildError("observed HEAD output")
        with lock.open("rb") as stream:
            lock_hash = hashlib.file_digest(stream, "sha256").hexdigest()
        build_output = command_output(build_argv, cwd, BUILD_LIMIT)
        executable = select_artifact(build_output, repo, recipe, package_id, source)
        receipt = {
            "schema_version": 1,
            "status": "build-output-recorded",
            "upstream_commit": recipe["upstream_commit"],
            "observed_head": head,
            "recipe_sha256": recipe_hash,
            "lock_sha256": lock_hash,
            "rustc": rustc,
            "cargo": cargo,
            "target": recipe["target"],
            "package": recipe["package"],
            "bin": recipe["bin"],
            "profile": recipe["profile"],
            "argv": build_argv,
            "repro": recipe["repro"],
            "runner": recipe["runner"],
            "msvc": recipe["msvc"],
            "source_evidence": "working-tree-unattested",
        }
        print(publish(repo, output, executable, receipt), end="")
        return 0
    except BuildError as error:
        print(
            f"build refused ({error}); remediation: verify the recipe, local paths, pinned tools, and Cargo result",
            file=sys.stderr,
        )
        return 1
    except (OSError, ValueError, TypeError, AttributeError, RecursionError):
        print(
            "build input or I/O failure; remediation: verify readable inputs, writable output parents, and valid producer data",
            file=sys.stderr,
        )
        return 1


if __name__ == "__main__":
    sys.exit(main())
