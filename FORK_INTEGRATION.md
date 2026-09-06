# Covenant Codex — Upstream Integration Rules

This repository is a customized fork of `openai/codex`.

## Branch roles

- `main` = pristine mirror of the latest stable upstream Codex release.
- `covenant-ver` = long-lived Covenant Codex development branch.
- `upgrade/*` = temporary branches used to integrate new upstream stable releases.

Never commit Covenant-specific changes directly to `main`.

## Upgrade procedure

When working on an upstream upgrade PR:

1. Treat the current `upgrade/*` branch as the new upstream stable version.
2. Merge `covenant-ver` into the current upgrade branch.
3. Resolve conflicts semantically; do not blindly choose "ours" or "theirs".
4. Preserve intentional Covenant Codex behavior and custom features.
5. Prefer adapting Covenant changes to the new upstream architecture rather than reverting new upstream code.
6. Remove obsolete Covenant compatibility code when upstream now provides an equivalent or better implementation.
7. Follow the repository's existing `AGENTS.md` and any nested agent instructions.
8. Run the appropriate formatting, linting, build, and test commands before considering the integration complete.
9. Fix regressions caused by the upstream update where reasonably possible.
10. Never modify or force-push `main`.
11. Only push integration changes to the current `upgrade/*` PR branch.
12. If the intended behavior is ambiguous or a conflict cannot be safely resolved, explain the issue in the PR instead of guessing.

## Covenant Codex goals

Covenant Codex is intended to remain easy to update from upstream while carrying our own modifications.

Release builds of Covenant Codex are intended to target Windows only, but upstream source structure should remain intact unless a Covenant-specific change explicitly requires otherwise.

## Intentional Covenant modifications

Document important custom behavior here as it is added:

- See [COVENANT_PATCHES.md](COVENANT_PATCHES.md) for the draft maintained patch surface and pending acceptance.
