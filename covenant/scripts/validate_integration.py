"""Check declared F00 structure offline; source completeness requires human review."""

import argparse
from collections import Counter
from pathlib import Path
import re
import stat
import sys
import tomllib

from validate_bootstrap import BootstrapError, FIELDS, artifact_entry, git, refuse


REQUIRED_SEAMS = {
    "cli_entry",
    "config_resolution",
    "managed_requirements",
    "model_catalog",
    "auth_storage",
    "auth_refresh",
    "tool_admission",
    "process_start",
    "patch_runtime",
    "tool_registration",
    "feature_registry",
}
OPTIONAL_SEAMS = {"pre_tool_hook", "hook_runtime", "mcp_launcher", "mcp_manager"}
FAMILIES = {
    "process",
    "filesystem",
    "network",
    "hosted",
    "mcp",
    "hook",
    "generated_code",
}
EFFECTS = {"none", "read", "process", "filesystem", "network", "hosted", "mcp"}
CONTROLS = {
    "COVENANT_DECIDER_PATH",
    "COVENANT_DECIDER_SHA256",
    "COVENANT_CHILD_MARKER",
    "CODEX_AUTH_HOME",
}
MAX_FILE_BYTES = 8 * 1024 * 1024


def fail(category, reason):
    refuse(
        category,
        reason,
        "correct the declared mapping against the pinned source and repeat human review",
    )


def fields(row, required, optional=()):
    if not isinstance(row, dict):
        fail("field", "expected a table")
    for field in required:
        if field not in row:
            fail(field, "missing required field")
    if row.keys() - set(required) - set(optional):
        fail("field", "unknown field in manifest table")


def string(value, category, empty=False):
    if (
        not isinstance(value, str)
        or len(value) > 2048
        or (not value and not empty)
        or any(ord(c) < 32 for c in value)
    ):
        fail(category, "expected a bounded literal string")
    return value


def array(value, category):
    if not isinstance(value, list) or len(value) > 4096:
        fail(category, "expected a bounded array")
    return value


def unique(values, category):
    if len(set(values)) != len(values):
        fail(category, "duplicate identity")
    return set(values)


def choice(value, allowed, category):
    if string(value, category) not in allowed:
        fail(category, "unknown classification")


def manifest(path):
    try:
        if path.stat().st_size > MAX_FILE_BYTES:
            fail("manifest", "TOML exceeds the size bound")
        return tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        fail("manifest", "missing or malformed TOML")


class Source:
    """Cache pinned and working literal anchors without interpreting their semantics."""

    def __init__(self, repo, commit):
        self.repo = repo
        self.commit = commit
        self.cache = {}

    def anchor(self, path, symbol):
        path = string(path, "path")
        symbol = string(symbol, "symbol").encode("utf-8")
        if path not in self.cache:
            parts = path.split("/")
            if (
                "\\" in path
                or ":" in path
                or any(part in ("", ".", "..") for part in parts)
            ):
                fail("path", "expected a normalized repository-relative source path")
            current = self.repo
            try:
                for part in parts:
                    current = current / part
                    info = current.lstat()
                    if stat.S_ISLNK(info.st_mode) or getattr(
                        info, "st_file_attributes", 0
                    ) & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0):
                        fail("path", "source path contains a link or reparse point")
                if not current.resolve(strict=True).is_relative_to(
                    self.repo
                ) or not stat.S_ISREG(info.st_mode):
                    fail("path", "source is not a repository-local regular file")
                entry = artifact_entry(self.repo, self.commit, path, "path")
                if not entry:
                    fail("path", "source is absent from the pinned tree")
                oid = entry[1].decode("ascii")
                size = int(git(self.repo, "cat-file", "-s", oid, field="path").stdout)
                if size > MAX_FILE_BYTES or info.st_size > MAX_FILE_BYTES:
                    fail("path", "source exceeds the size bound")
                pinned = git(self.repo, "cat-file", "blob", oid, field="path").stdout
                self.cache[path] = pinned, current.read_bytes()
            except (OSError, ValueError, RuntimeError):
                fail("path", "cannot read pinned and working source")
        if any(symbol not in view for view in self.cache[path]):
            fail("symbol", "literal anchor is absent from pinned or working source")


def validate(repo, upstream, integration, inventory):
    fields(upstream, {"commit"}, set(FIELDS) - {"commit"})
    fields(integration, {"schema_version", "commit", "absent_seams", "seams", "tools"})
    fields(
        inventory,
        {
            "schema_version",
            "commit",
            "scrub_keys",
            "scrub_prefixes",
            "authorities",
            "secrets",
            "audit_queries",
        },
    )
    commits = []
    for model in (upstream, integration, inventory):
        commit = string(model["commit"], "commit")
        if not re.fullmatch(r"[0-9a-f]{40}", commit):
            fail("commit", "expected a lowercase full commit identifier")
        commits.append(commit)
    if len(set(commits)) != 1:
        fail("commit", "manifest bindings disagree")
    commit = commits[0]
    if git(repo, "cat-file", "-t", commit, field="commit").stdout.strip() != b"commit":
        fail("commit", "binding is not a local commit object")
    for model in (integration, inventory):
        if type(model["schema_version"]) is not int or model["schema_version"] != 1:
            fail("schema_version", "expected integer version 1")
    source = Source(repo, commit)
    seams = array(integration["seams"], "seams")
    seam_ids = []
    for row in seams:
        fields(row, {"id", "phase", "path", "symbols"})
        seam_ids.append(string(row["id"], "seam"))
        choice(row["id"], REQUIRED_SEAMS | OPTIONAL_SEAMS, "seam")
        choice(row["phase"], {"F11", "F12", "F13", "F14", "F21", "F22"}, "phase")
        symbols = array(row["symbols"], "symbol")
        if not symbols:
            fail("symbol", "seam requires a literal anchor")
        for symbol in symbols:
            source.anchor(row["path"], symbol)
    present = unique(seam_ids, "seam")
    absent = unique(
        [string(value, "seam") for value in array(integration["absent_seams"], "seam")],
        "seam",
    )
    if (
        not REQUIRED_SEAMS <= present
        or present & absent
        or present | absent != REQUIRED_SEAMS | OPTIONAL_SEAMS | {"build_metadata"}
    ):
        fail("seam", "required or optional seam accounting disagrees")

    tools = array(integration["tools"], "tools")
    tool_ids = []
    identities = []
    for row in tools:
        fields(row, {"id", "namespace", "name", "form", "effect", "path", "symbol"})
        tool_ids.append(string(row["id"], "tool"))
        identities.append(
            (
                string(row["namespace"], "tool", empty=True),
                string(row["name"], "tool"),
                string(row["form"], "form"),
            )
        )
        choice(row["form"], {"function", "custom", "hosted", "tool_search"}, "form")
        choice(row["effect"], EFFECTS, "effect")
        source.anchor(row["path"], row["symbol"])
    declared_tools = unique(tool_ids, "tool")
    wire = unique(identities, "tool")
    if not {("", "exec_command", "function"), ("", "apply_patch", "custom")} <= wire:
        fail("tool", "missing intended public identity")

    authorities = array(inventory["authorities"], "authorities")
    authority_ids, classified, kinds, families = [], [], set(), set()
    for row in authorities:
        fields(
            row,
            {"id", "kind", "family", "disposition", "path", "symbol", "reason"},
            {"tool_id", "gate"},
        )
        authority_ids.append(string(row["id"], "authority"))
        choice(row["kind"], {"tool", "constructor", "sink"}, "kind")
        choice(row["family"], FAMILIES | EFFECTS, "family")
        choice(
            row["disposition"],
            {"gated", "excluded", "trusted_support", "accepted_residual"},
            "disposition",
        )
        kinds.add(row["kind"])
        families.add(row["family"])
        string(row["reason"], "reason")
        if row["kind"] == "tool":
            tool_id = string(row.get("tool_id"), "tool")
            if tool_id not in declared_tools:
                fail("tool", "authority references an undeclared tool")
            classified.append(tool_id)
        elif "tool_id" in row:
            fail("tool", "only tool authorities reference a tool identity")
        if row["disposition"] == "gated":
            if string(row.get("gate"), "gate") not in present:
                fail("gate", "authority references an absent seam")
        elif "gate" in row:
            fail("gate", "only gated dispositions reference a gate")
        source.anchor(row["path"], row["symbol"])
    unique(authority_ids, "authority")
    if Counter(classified) != Counter({tool_id: 1 for tool_id in declared_tools}):
        fail("tool", "each declared tool needs exactly one authority classification")
    if not {"constructor", "sink"} <= kinds:
        fail("kind", "constructor and sink coverage is required")
    if not FAMILIES <= families:
        fail("family", "declared authority family coverage is incomplete")

    scrub = {}
    for field in ("scrub_keys", "scrub_prefixes"):
        values = [string(value, "scrub") for value in array(inventory[field], "scrub")]
        if any(not re.fullmatch(r"[A-Z_][A-Z0-9_]*", value) for value in values):
            fail(
                "scrub",
                "keys and prefixes must use canonical uppercase environment names",
            )
        scrub[field] = unique(values, "scrub")
    if not CONTROLS <= scrub["scrub_keys"]:
        fail("scrub", "required control keys are missing")
    secrets = array(inventory["secrets"], "secrets")
    secret_keys, classes = [], set()
    for row in secrets:
        fields(row, {"key", "class", "path", "symbol"})
        key = string(row["key"], "secret")
        if not re.fullmatch(r"[A-Z_][A-Z0-9_]*", key):
            fail("secret", "expected a canonical uppercase environment key")
        secret_keys.append(key)
        choice(row["class"], {"control", "provider_login"}, "secret")
        classes.add(row["class"])
        if key not in scrub["scrub_keys"] and not any(
            key.startswith(prefix) for prefix in scrub["scrub_prefixes"]
        ):
            fail("secret", "declared secret lacks scrub coverage")
        source.anchor(row["path"], row["symbol"])
    unique(secret_keys, "secret")
    if "provider_login" not in classes:
        fail("secret", "provider/login key class is missing")
    audit_families = set()
    queries = array(inventory["audit_queries"], "audit_queries")
    query_ids = []
    for row in queries:
        fields(row, {"family", "path", "symbol"})
        choice(row["family"], FAMILIES | EFFECTS, "family")
        source.anchor(row["path"], row["symbol"])
        audit_families.add(row["family"])
        query_ids.append((row["family"], row["path"], row["symbol"]))
    unique(query_ids, "audit_queries")
    if not FAMILIES <= audit_families:
        fail("family", "declared audit query coverage is incomplete")
    return f"Structure verified at {commit}: {len(seams)} seams, {len(tools)} tools, {len(authorities)} authorities; semantic source review remains required."


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("repo", "upstream", "integration", "inventory"):
        parser.add_argument(f"--{name}", type=Path, required=True)
    args = parser.parse_args()
    try:
        repo = args.repo.resolve(strict=True)
        print(
            validate(
                repo,
                manifest(args.upstream),
                manifest(args.integration),
                manifest(args.inventory),
            )
        )
    except BootstrapError as error:
        print(error, file=sys.stderr)
        return 1
    except (OSError, ValueError, RuntimeError):
        print(
            "manifest: cannot read integration inputs\nremediation: check repository and manifest paths",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
