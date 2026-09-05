"""Verify Covenant's recorded upstream lineage and attribution without network I/O."""

import argparse
import os
from pathlib import Path, PurePosixPath
import re
import stat
import subprocess
import sys
import tomllib


FIELDS = (
    "upstream_url",
    "tag",
    "commit",
    "license_spdx",
    "rust_toolchain_path",
    "rust_channel",
    "fork_url",
    "fork_branch",
)


class BootstrapError(Exception):
    """A bounded, actionable validation refusal that contains no command output."""


def refuse(field, reason, remediation):
    raise BootstrapError(f"{field}: {reason}\nremediation: {remediation}")


def git(repo, *args, field, allow_failure=False):
    env = {
        key: value for key, value in os.environ.items() if not key.startswith("GIT_")
    }
    env.update(
        GIT_CONFIG_NOSYSTEM="1",
        GIT_CONFIG_GLOBAL=os.devnull,
        GIT_TERMINAL_PROMPT="0",
        GIT_NO_LAZY_FETCH="1",
        GIT_GRAFT_FILE=os.devnull,
    )
    try:
        result = subprocess.run(
            [
                "git",
                "--no-replace-objects",
                "--no-optional-locks",
                "--literal-pathspecs",
                "-c",
                "protocol.allow=never",
                *args,
            ],
            cwd=repo,
            env=env,
            capture_output=True,
            timeout=10,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        refuse(
            field,
            "local Git verification could not run",
            "check Git and the local repository",
        )
    if result.returncode and not allow_failure:
        refuse(
            field,
            "local Git verification failed",
            "restore the required local Git evidence",
        )
    return result


def load_manifest(path):
    try:
        with path.open("rb") as stream:
            manifest = tomllib.load(stream)
    except (OSError, ValueError):
        refuse("manifest", "missing or invalid TOML", "restore a valid UPSTREAM.toml")
    for field in FIELDS:
        value = manifest.get(field)
        if not isinstance(value, str) or not value or len(value) > 1024:
            refuse(
                field,
                "requires a nonempty bounded string",
                "correct the named manifest field",
            )
    if manifest.keys() - set(FIELDS):
        refuse(
            "manifest",
            "contains unknown fields",
            "use only the documented bootstrap fields",
        )
    return manifest


def artifact_entry(repo, source, relative, field):
    args = (
        ("ls-files", "--stage", "-z")
        if source == "index"
        else ("ls-tree", "-z", source)
    )
    output = git(repo, *args, "--", relative, field=field).stdout
    if not output:
        return None
    entries = output.rstrip(b"\0").split(b"\0")
    if len(entries) != 1:
        refuse(
            field,
            "ambiguous or unmerged artifact entry",
            "restore its exact pinned index and tree entry",
        )
    metadata, separator, name = entries[0].partition(b"\t")
    values = metadata.split()
    if not separator or name != relative.encode("utf-8") or len(values) != 3:
        refuse(field, "invalid artifact entry", "restore its pinned repository entry")
    mode, middle, last = values
    if mode not in (b"100644", b"100755") or (
        last != b"0" if source == "index" else middle != b"blob"
    ):
        refuse(
            field,
            "artifact is not a regular, merged file",
            "restore its pinned file type and index entry",
        )
    return mode, middle if source == "index" else last


def artifact(repo, commit, relative, field, required=True):
    path = repo / relative
    try:
        if not path.resolve().is_relative_to(repo):
            refuse(
                field,
                "path escapes the repository",
                "restore the repository-local artifact",
            )
        entry = artifact_entry(repo, commit, relative, field)
        for source in ("HEAD", "index"):
            if artifact_entry(repo, source, relative, field) != entry:
                refuse(
                    field,
                    f"{source} artifact differs from the pinned tree",
                    "restore its pinned existence, mode, and bytes",
                )
        pinned = None
        if entry:
            pinned = git(
                repo, "cat-file", "blob", entry[1].decode("ascii"), field=field
            ).stdout
        if os.path.lexists(path) and not stat.S_ISREG(path.lstat().st_mode):
            refuse(
                field,
                "working artifact is not a regular file",
                "restore the pinned artifact without a link or directory",
            )
        current = path.read_bytes() if os.path.lexists(path) else None
    except (OSError, ValueError, RuntimeError):
        refuse(
            field,
            "artifact cannot be read",
            "restore the pinned artifact inside the repository",
        )
    if required and pinned is None:
        refuse(
            field,
            "artifact is absent from the pinned tree",
            "record its correct upstream path",
        )
    if current != pinned:
        refuse(
            field,
            "working artifact differs from the pinned tree",
            "restore its pinned existence and exact bytes",
        )
    return pinned


def validate(repo, manifest):
    if (
        manifest["upstream_url"].removesuffix(".git")
        != "https://github.com/openai/codex"
    ):
        refuse(
            "upstream_url",
            "unrecognized upstream",
            "record https://github.com/openai/codex",
        )
    if manifest["license_spdx"] != "Apache-2.0":
        refuse(
            "license_spdx",
            "incorrect upstream attribution",
            "record the upstream Apache-2.0 license",
        )
    commit = manifest["commit"]
    tag = manifest["tag"]
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        refuse(
            "commit",
            "requires a full lowercase 40-hex object ID",
            "record the exact upstream commit",
        )
    if not re.fullmatch(r"rust-v[0-9]+\.[0-9]+\.[0-9]+", tag):
        refuse(
            "tag",
            "requires a stable rust-v release tag",
            "record its literal stable tag name",
        )
    branch = manifest["fork_branch"]
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._/-]*", branch):
        refuse(
            "fork_branch",
            "invalid branch name",
            "record the intended literal fork branch",
        )
    git(repo, "check-ref-format", f"refs/heads/{branch}", field="fork_branch")
    fork_url = manifest["fork_url"].removesuffix(".git")
    if not re.fullmatch(
        r"https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", fork_url
    ):
        refuse(
            "fork_url",
            "requires the public GitHub fork URL",
            "record the fork URL without credentials",
        )

    if git(repo, "cat-file", "-t", commit, field="commit").stdout.strip() != b"commit":
        refuse(
            "commit",
            "object is not a commit",
            "record the exact upstream commit object",
        )
    resolved = git(
        repo,
        "rev-parse",
        "--verify",
        "--end-of-options",
        f"refs/tags/{tag}^{{commit}}",
        field="tag",
    ).stdout.strip()
    if resolved != commit.encode("ascii"):
        refuse(
            "tag",
            "local tag does not resolve to commit",
            "reconcile the recorded tag and commit with upstream evidence",
        )
    ancestor = git(
        repo,
        "merge-base",
        "--is-ancestor",
        commit,
        "HEAD",
        field="ancestor",
        allow_failure=True,
    )
    if ancestor.returncode:
        refuse(
            "ancestor",
            "pinned commit is not verified in HEAD ancestry",
            "verify the fork descends from the recorded upstream commit",
        )
    for purpose in ((), ("--push",)):
        urls = git(
            repo, "remote", "get-url", *purpose, "--all", "origin", field="fork_url"
        ).stdout.splitlines()
        if len(urls) != 1 or urls[0].removesuffix(b".git") != fork_url.encode("utf-8"):
            refuse(
                "fork_url",
                "effective origin destinations differ from the single recorded fork",
                "reconcile origin URLs, push URLs, and URL rewrites with the intended fork",
            )
    actual_branch = git(
        repo, "symbolic-ref", "--quiet", "HEAD", field="fork_branch"
    ).stdout.strip()
    if actual_branch != f"refs/heads/{branch}".encode("ascii"):
        refuse(
            "fork_branch",
            "checkout differs from the recorded branch",
            "validate from the intended fork branch",
        )

    artifact(repo, commit, "LICENSE", "LICENSE")
    artifact(repo, commit, "NOTICE", "NOTICE", required=False)
    relative = manifest["rust_toolchain_path"]
    parts = PurePosixPath(relative)
    if (
        parts.is_absolute()
        or "\\" in relative
        or ":" in relative
        or any(part in (".", "..") for part in relative.split("/"))
    ):
        refuse(
            "rust_toolchain_path",
            "requires a repository-relative path",
            "record the pinned upstream toolchain path",
        )
    pinned = artifact(repo, commit, relative, "rust_toolchain_path")
    try:
        channel = tomllib.loads(pinned.decode("utf-8"))["toolchain"]["channel"]
    except (ValueError, KeyError, TypeError):
        refuse(
            "rust_toolchain_path",
            "invalid pinned toolchain TOML",
            "record the upstream toolchain manifest",
        )
    if channel != manifest["rust_channel"]:
        refuse(
            "rust_channel",
            "channel disagrees with the pinned toolchain",
            "record the channel from the pinned upstream toolchain",
        )
    return (
        f"Verified {tag} at {commit}: fork lineage, attribution, and toolchain agree."
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    args = parser.parse_args()
    try:
        repo = args.repo.resolve(strict=True)
        print(validate(repo, load_manifest(args.manifest)))
    except BootstrapError as error:
        print(error, file=sys.stderr)
        return 1
    except (OSError, ValueError, RuntimeError):
        print(
            "repository: cannot read bootstrap inputs\nremediation: check the repository and manifest paths",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
