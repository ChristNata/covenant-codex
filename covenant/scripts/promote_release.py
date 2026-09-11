"""Assemble the five F33 release assets after independent evidence checks."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import tomllib


EXE_NAME = "codex-x86_64-pc-windows-msvc.exe"
DIGEST_NAME = "codex.exe.sha256"
ASSET_NAMES = (
    EXE_NAME,
    DIGEST_NAME,
    "covenant-inventory.json",
    "provenance.json",
    "COVENANT_PATCHES.md",
)
HEX64 = re.compile(r"[0-9a-f]{64}\Z")
COMMIT = re.compile(r"[0-9a-f]{40}\Z")


class PromotionError(Exception):
    """Raised when promotion evidence is absent, malformed, or inconsistent."""


def _require_sidecar_fixture(path: Path) -> dict:
    try:
        with path.open("rb") as stream:
            fixture = tomllib.load(stream)
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise PromotionError("missing or invalid real-sidecar fixture") from error
    if fixture.get("schema_version") != 1 or fixture.get("status") != "ready":
        raise PromotionError("real-sidecar evidence is not ready")
    url = fixture.get("url")
    if not isinstance(url, str) or not url.startswith("https://"):
        raise PromotionError("invalid real-sidecar URL")
    digest = fixture.get("sha256")
    if not isinstance(digest, str) or HEX64.fullmatch(digest) is None:
        raise PromotionError("invalid real-sidecar digest")
    for name in ("g4_schema_id", "g4_semantics_id"):
        if not isinstance(fixture.get(name), str) or not fixture[name].strip():
            raise PromotionError(f"missing real-sidecar {name}")
    return fixture


def _json(path: Path):
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, ValueError) as error:
        raise PromotionError("invalid JSON evidence") from error
    if not isinstance(value, dict):
        raise PromotionError("JSON evidence must be an object")
    return value


def _digest(path: Path) -> str:
    try:
        digest = hashlib.sha256()
        with path.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(chunk)
    except OSError as error:
        raise PromotionError("missing artifact") from error
    return digest.hexdigest()


def _require_text(value, pattern, name):
    if not isinstance(value, str) or pattern.fullmatch(value) is None:
        raise PromotionError(f"invalid {name}")
    return value


def assemble_provenance(
    *,
    repo: Path,
    artifact_dir: Path,
    inventory_path: Path,
    reaudit_path: Path,
    source_commit: str,
    tag: str,
    toolchain: str,
    runner: str,
    patches_path: Path,
    sidecar_fixture_path: Path,
    output_dir: Path,
):
    """Validate F33 evidence, then create an isolated five-asset directory."""
    source_commit = _require_text(source_commit, COMMIT, "source commit")
    sidecar = _require_sidecar_fixture(sidecar_fixture_path)
    if not isinstance(tag, str) or not tag or any(char in tag for char in "\r\n"):
        raise PromotionError("invalid tag")
    if not isinstance(toolchain, str) or not toolchain:
        raise PromotionError("invalid toolchain")
    if runner != "windows-2022":
        raise PromotionError("invalid runner")
    exe = artifact_dir / EXE_NAME
    digest_path = artifact_dir / DIGEST_NAME
    actual_digest = _digest(exe)
    try:
        recorded_digest = digest_path.read_text(encoding="ascii")
    except (OSError, UnicodeError) as error:
        raise PromotionError("missing digest") from error
    if recorded_digest != actual_digest + "\n":
        raise PromotionError("executable digest mismatch")

    inventory = _json(inventory_path)
    _require_text(inventory.get("binary_digest"), HEX64, "inventory binary digest")
    if inventory["binary_digest"] != actual_digest or not inventory.get("inventory_id"):
        raise PromotionError("inventory does not bind executable")
    reaudit = _json(reaudit_path)
    if reaudit.get("status") != "passed" or reaudit.get("commit") != source_commit:
        raise PromotionError("re-audit is not green on source commit")
    audit_run_id = reaudit.get("run_id")
    if not isinstance(audit_run_id, str) or not audit_run_id:
        raise PromotionError("missing re-audit run id")
    try:
        patches = patches_path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise PromotionError("missing patch index") from error
    if "Final hunk hashes do not exist" in patches:
        raise PromotionError("patch index has placeholder hunk hashes")
    for phase in ("F11", "F12", "F13", "F14"):
        if not re.search(rf"{phase}.*hunk_sha256\s*=\s*[\"']?[0-9a-f]{{64}}", patches):
            raise PromotionError(f"missing {phase} hunk hash")

    output_dir = output_dir.absolute()
    if output_dir.exists():
        raise PromotionError("promotion output already exists")
    output_dir.mkdir(parents=True)
    try:
        shutil.copyfile(exe, output_dir / EXE_NAME)
        shutil.copyfile(digest_path, output_dir / DIGEST_NAME)
        shutil.copyfile(inventory_path, output_dir / "covenant-inventory.json")
        shutil.copyfile(patches_path, output_dir / "COVENANT_PATCHES.md")
        provenance = {
            "schema_version": 1,
            "source_commit": source_commit,
            "tag": tag,
            "toolchain": toolchain,
            "runner": runner,
            "exe_digest": actual_digest,
            "inventory_digest": _digest(inventory_path),
            "re_audit_run_id": audit_run_id,
            "sidecar": {
                "url": sidecar["url"],
                "sha256": sidecar["sha256"],
                "g4_schema_id": sidecar["g4_schema_id"],
                "g4_semantics_id": sidecar["g4_semantics_id"],
            },
        }
        (output_dir / "provenance.json").write_text(
            json.dumps(provenance, sort_keys=True, indent=2) + "\n", encoding="utf-8"
        )
        return {**provenance, "inventory": inventory}
    except (OSError, UnicodeError) as error:
        shutil.rmtree(output_dir, ignore_errors=True)
        raise PromotionError("unable to assemble release assets") from error


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--artifact-dir", type=Path, required=True)
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--reaudit", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--toolchain", required=True)
    parser.add_argument("--runner", default="windows-2022")
    parser.add_argument("--patches", type=Path, required=True)
    parser.add_argument("--sidecar-fixture", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        result = assemble_provenance(
            repo=args.repo,
            artifact_dir=args.artifact_dir,
            inventory_path=args.inventory,
            reaudit_path=args.reaudit,
            source_commit=args.source_commit,
            tag=args.tag,
            toolchain=args.toolchain,
            runner=args.runner,
            patches_path=args.patches,
            sidecar_fixture_path=args.sidecar_fixture,
            output_dir=args.output_dir,
        )
        print(
            json.dumps(
                {"status": "promotion-assets-assembled", **result}, sort_keys=True
            )
        )
        return 0
    except PromotionError as error:
        print(f"promotion refused: {error}", file=__import__("sys").stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
