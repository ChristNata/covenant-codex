"""Validate the independently supplied E01 real-sidecar manifest."""

import argparse
import json
from pathlib import Path
import re
import tomllib


HEX64 = re.compile(r"[0-9a-f]{64}\Z")


class SidecarFixtureError(Exception):
    """Raised when the E01 sidecar evidence is absent or malformed."""


def read_fixture(path: Path) -> dict:
    try:
        with path.open("rb") as stream:
            fixture = tomllib.load(stream)
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise SidecarFixtureError("unable to read sidecar fixture") from error
    if not isinstance(fixture, dict) or fixture.get("schema_version") != 1:
        raise SidecarFixtureError("unsupported sidecar fixture schema")
    status = fixture.get("status")
    if status not in {"ready", "exec-only"}:
        raise SidecarFixtureError("real-sidecar evidence is not ready")
    url = fixture.get("url")
    if status == "ready":
        if not isinstance(url, str) or not url.startswith("https://"):
            raise SidecarFixtureError("sidecar URL must be HTTPS")
    elif url != "":
        raise SidecarFixtureError("exec-only sidecar URL must be empty")
    digest = fixture.get("sha256")
    if status == "ready":
        if not isinstance(digest, str) or HEX64.fullmatch(digest) is None:
            raise SidecarFixtureError("sidecar SHA-256 is invalid")
    elif digest != "":
        raise SidecarFixtureError("exec-only sidecar digest must be empty")
    for name in ("g4_schema_id", "g4_semantics_id"):
        value = fixture.get(name)
        if not isinstance(value, str) or not value.strip():
            raise SidecarFixtureError(f"missing {name}")
    if status == "exec-only":
        authority = fixture.get("patch_authority")
        if not isinstance(authority, str) or not authority.startswith("deferred:"):
            raise SidecarFixtureError(
                "exec-only sidecar must document deferred patch authority"
            )
    return fixture


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("fixture", type=Path)
    args = parser.parse_args(argv)
    try:
        print(json.dumps(read_fixture(args.fixture), sort_keys=True))
    except SidecarFixtureError as error:
        print(f"sidecar fixture refused: {error}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
