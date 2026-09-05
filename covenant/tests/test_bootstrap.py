"""Behavioral contract for the offline Covenant bootstrap validator.

Production CLI: validate_bootstrap.py --repo ROOT --manifest UPSTREAM.toml.
Success exits 0 and reports the resolved tag and commit. Refusal exits 1 with
the failing field/artifact and a ``remediation:`` instruction on stderr.

Run this file with ``--validator PATH`` to assess a temporary baseline adapter;
the default always exercises the production entrypoint. Every Git repository
and ref manipulated here belongs to a disposable fixture, never the checkout.
"""

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


VALIDATOR = Path(__file__).resolve().parents[1] / "scripts" / "validate_bootstrap.py"


class BootstrapTests(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(prefix="covenant-bootstrap-")
        self.addCleanup(self.scratch.cleanup)
        self.repo = Path(self.scratch.name) / "repo"
        self.repo.mkdir()
        self.git("init", "--initial-branch=covenant-ver")
        self.git("remote", "add", "origin", "https://github.com/fixture/covenant-codex.git")
        self.write("LICENSE", "Fixture license text\n")
        self.write("NOTICE", "Fixture upstream notice\n")
        self.write(
            "codex-rs/rust-toolchain.toml",
            '[toolchain]\nchannel = "1.95.0"\ncomponents = ["clippy", "rustfmt"]\n',
        )
        self.commit("Synthetic upstream")
        self.upstream = self.git("rev-parse", "HEAD")
        self.git("tag", "-a", "rust-v0.999.1", "-m", "Synthetic stable release")
        self.write("COVENANT.txt", "An unrelated fork-only addition\n")
        self.commit("Synthetic fork addition")
        self.manifest = self.repo / "covenant" / "UPSTREAM.toml"
        self.values = {
            "upstream_url": "https://github.com/openai/codex",
            "tag": "rust-v0.999.1",
            "commit": self.upstream,
            "license_spdx": "Apache-2.0",
            "rust_toolchain_path": "codex-rs/rust-toolchain.toml",
            "rust_channel": "1.95.0",
            "fork_url": "https://github.com/fixture/covenant-codex",
            "fork_branch": "covenant-ver",
        }
        self.save_manifest()

    def git(self, *args):
        env = os.environ.copy()
        for key in ("GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_COMMON_DIR"):
            env.pop(key, None)
        env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull)
        result = subprocess.run(
            [
                "git", "-c", "user.name=Bootstrap Fixture", "-c",
                "user.email=bootstrap-fixture@example.invalid", "-c",
                "commit.gpgsign=false", "-c", "tag.gpgsign=false", "-c",
                "core.autocrlf=false", "-c",
                f"core.hooksPath={self.repo / 'absent-hooks'}", *args,
            ],
            cwd=self.repo,
            env=env,
            text=True,
            capture_output=True,
            timeout=20,
            check=True,
        )
        return result.stdout.strip()

    def write(self, relative, content):
        path = self.repo / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8", newline="\n")

    def commit(self, message):
        self.git("add", "--all")
        self.git("commit", "--quiet", "-m", message)

    def save_manifest(self, **changes):
        self.values.update(changes)
        self.write(
            "covenant/UPSTREAM.toml",
            "".join(f"{key} = {json.dumps(value)}\n" for key, value in self.values.items()),
        )

    def validate(self):
        return subprocess.run(
            [sys.executable, str(VALIDATOR), "--repo", str(self.repo), "--manifest", str(self.manifest)],
            text=True,
            capture_output=True,
            timeout=20,
        )

    def assert_allowed(self):
        result = self.validate()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(self.values["tag"], result.stdout)
        self.assertIn(self.values["commit"], result.stdout)

    def assert_denied(self, diagnostic):
        result = self.validate()
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn(diagnostic.lower(), result.stderr.lower())
        self.assertIn("remediation:", result.stderr.lower())
        self.assertNotIn("Traceback", result.stderr)

    def test_accepts_annotated_upstream_tag_with_later_fork_commit(self):
        self.assertNotEqual(self.git("rev-parse", "HEAD"), self.upstream)
        self.assert_allowed()

    def test_accepts_lightweight_tag_and_matching_git_suffix(self):
        self.git("tag", "rust-v0.999.2", self.upstream)
        self.save_manifest(tag="rust-v0.999.2", fork_url="https://github.com/fixture/covenant-codex.git")
        self.assert_allowed()

    def test_accepts_absent_notice_only_when_absent_from_pinned_tree(self):
        (self.repo / "NOTICE").unlink()
        self.commit("Upstream without NOTICE")
        self.git("tag", "rust-v0.999.3")
        self.save_manifest(tag="rust-v0.999.3", commit=self.git("rev-parse", "HEAD"))
        self.assert_allowed()
        self.write("NOTICE", "New attribution not present upstream\n")
        self.assert_denied("NOTICE")

    def test_rejects_missing_or_malformed_manifest_without_traceback(self):
        self.manifest.unlink()
        self.assert_denied("manifest")
        self.manifest.write_text("commit = [unterminated\n", encoding="utf-8")
        self.assert_denied("manifest")

    def test_rejects_missing_or_wrong_type_required_metadata(self):
        del self.values["commit"]
        self.save_manifest()
        self.assert_denied("commit")
        self.save_manifest(commit=[self.upstream])
        self.assert_denied("commit")

    def test_rejects_abbreviated_unknown_or_revspec_commit(self):
        for commit in (self.upstream[:12], "f" * 40, "HEAD^{commit}", "--help"):
            with self.subTest(commit=commit):
                self.save_manifest(commit=commit)
                self.assert_denied("commit")

    def test_rejects_existing_tagged_commit_outside_current_ancestry(self):
        self.git("switch", "--orphan", "unrelated")
        self.write("unrelated.txt", "Separate history\n")
        self.commit("Unrelated history")
        other = self.git("rev-parse", "HEAD")
        self.git("tag", "rust-v0.999.4")
        self.git("switch", "covenant-ver")
        self.save_manifest(tag="rust-v0.999.4", commit=other)
        self.assert_denied("ancestor")

    def test_rejects_unresolvable_mismatched_or_invalid_tag(self):
        for tag in ("rust-v0.999.404", "--help", "HEAD^{commit}"):
            with self.subTest(tag=tag):
                self.save_manifest(tag=tag)
                self.assert_denied("tag")
        self.git("tag", "rust-v0.999.5", "HEAD")
        self.save_manifest(tag="rust-v0.999.5")
        self.assert_denied("tag")

    def test_rejects_substituted_upstream_repository(self):
        self.save_manifest(upstream_url="https://github.com/untrusted/codex")
        self.assert_denied("upstream_url")

    def test_rejects_origin_or_branch_different_from_declared_topology(self):
        self.git("remote", "set-url", "origin", "https://github.com/unrelated/codex")
        self.assert_denied("fork_url")
        self.git("remote", "set-url", "origin", self.values["fork_url"])
        self.git("switch", "-c", "unexpected-branch")
        self.assert_denied("fork_branch")

    def test_rejects_effective_origin_redirects_and_multiple_urls(self):
        config_path = self.repo / ".git" / "config"
        original_config = config_path.read_bytes()
        unexpected = "https://github.com/unrelated/covenant-codex"
        for redirect in (
            "multiple_urls", "duplicate_urls", "pushurl", "insteadOf",
            "pushInsteadOf", "included_insteadOf",
        ):
            with self.subTest(redirect=redirect):
                try:
                    if redirect == "multiple_urls":
                        self.git("remote", "set-url", "origin", unexpected)
                        self.git("config", "--add", "remote.origin.url", self.values["fork_url"])
                    elif redirect == "duplicate_urls":
                        self.git("config", "--add", "remote.origin.url", self.values["fork_url"])
                    elif redirect == "pushurl":
                        self.git("config", "remote.origin.pushurl", unexpected)
                    elif redirect == "included_insteadOf":
                        self.write(
                            ".git/origin-redirect.inc",
                            '[url "https://github.com/unrelated/"]\n'
                            "    insteadOf = https://github.com/fixture/\n",
                        )
                        self.git("config", "include.path", str(self.repo / ".git" / "origin-redirect.inc"))
                    else:
                        self.git(
                            "config", f"url.https://github.com/unrelated/.{redirect}",
                            "https://github.com/fixture/",
                        )
                    self.assert_denied("fork_url")
                finally:
                    config_path.write_bytes(original_config)

    def test_accepts_branch_also_named_by_a_tag(self):
        self.git("tag", self.values["fork_branch"])
        self.assertEqual(self.git("symbolic-ref", "HEAD"), "refs/heads/covenant-ver")
        self.assert_allowed()

    def test_rejects_graft_or_replace_ancestry_overrides(self):
        original_head = self.git("rev-parse", "HEAD")
        tree = self.git("rev-parse", "HEAD^{tree}")
        unrelated = self.git("commit-tree", tree, "-m", "Unrelated root with identical files")
        self.git("update-ref", "refs/heads/covenant-ver", unrelated)
        self.assert_denied("ancestor")
        self.git("replace", unrelated, original_head)
        self.assert_denied("ancestor")
        self.git("replace", "-d", unrelated)
        self.write(".git/info/grafts", f"{unrelated} {self.upstream}\n")
        self.assert_denied("ancestor")

    def test_rejects_head_or_index_artifact_drift_hidden_by_worktree(self):
        original_head = self.git("rev-parse", "HEAD")
        protected = (
            ("LICENSE", "LICENSE"),
            ("NOTICE", "NOTICE"),
            ("codex-rs/rust-toolchain.toml", "rust_toolchain_path"),
        )
        for location in ("index", "HEAD"):
            for relative, diagnostic in protected:
                with self.subTest(location=location, artifact=relative):
                    path = self.repo / relative
                    original = path.read_bytes()
                    try:
                        path.write_bytes(original + b"Unreviewed protected artifact change\n")
                        self.git("add", "--", relative)
                        if location == "HEAD":
                            self.git("commit", "--quiet", "-m", "Changed protected artifact")
                        path.write_bytes(original)
                        self.assert_denied(diagnostic)
                    finally:
                        self.git("reset", "--hard", original_head)

    def test_rejects_deleted_or_modified_upstream_attribution(self):
        for name in ("LICENSE", "NOTICE"):
            with self.subTest(artifact=name):
                path = self.repo / name
                original = path.read_bytes()
                path.unlink()
                self.assert_denied(name)
                path.write_bytes(original + b"Unreviewed attribution change\n")
                self.assert_denied(name)
                path.write_bytes(original)

    def test_rejects_toolchain_metadata_disagreeing_with_pinned_tree(self):
        self.save_manifest(rust_channel="0.0.0")
        self.assert_denied("rust_channel")
        self.save_manifest(rust_channel="1.95.0")
        self.write("codex-rs/rust-toolchain.toml", '[toolchain]\nchannel = "0.0.0"\n')
        self.assert_denied("toolchain")

    def test_rejects_unpinned_or_outside_repository_toolchain_path(self):
        self.write("covenant/rust-toolchain.toml", '[toolchain]\nchannel = "1.95.0"\n')
        for path in ("covenant/rust-toolchain.toml", "../outside.toml", str(self.repo / "LICENSE")):
            with self.subTest(path=path):
                self.save_manifest(rust_toolchain_path=path)
                self.assert_denied("rust_toolchain_path")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--validator", type=Path, default=VALIDATOR)
    options, unittest_args = parser.parse_known_args()
    VALIDATOR = options.validator.resolve()
    unittest.main(argv=[sys.argv[0], *unittest_args])
