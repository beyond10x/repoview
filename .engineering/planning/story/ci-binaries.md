---
format: aep.planning-md/3
id: story:ci-binaries
kind: story
status: implemented
title: CI builds downloadable binaries with the web app embedded
summary: ci.yml runs task check and uploads a Linux build per main push; release-build.yml builds three targets with smoke tests and SHA256SUMS on a tag.
relations:
- decomposes: epic:release
- serves: vision:repoview
scope:
- confidence: cited
  path: .github/workflows/ci.yml
- confidence: cited
  path: .github/workflows/release-build.yml
- confidence: cited
  path: crates/repoview/tests/workflows.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T16:15:09Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-05T16:15:10Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-10-06T00:19:13Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":3,"verification":2}}}
---
## Story

As an engineer, I download a `repoview` binary with the web app embedded from GitHub: every `main`
push leaves a Linux build as a workflow artifact, and a tag builds the release archives.

## Acceptance

1. `.github/workflows/ci.yml` runs on pull requests and on pushes to `main`: installs Rust 1.98.1,
   Node 22, pnpm (the version in `web/package.json` `packageManager`), go-task, `aep` and `ess` at the
   versions the gate needs (from their GitHub releases, checksum-verified), then `task check`. On a
   `main` push it also runs `task build` and uploads
   `repoview-<short-sha>-x86_64-unknown-linux-gnu.tar.gz` (the binary, `README.md`, `LICENSE` if
   present) as a workflow artifact kept 30 days.
2. `.github/workflows/release-build.yml` runs on tags matching `0.*`: builds `x86_64-unknown-linux-gnu`
   (ubuntu-22.04), `aarch64-unknown-linux-gnu` (ubuntu-22.04-arm) and `aarch64-apple-darwin`
   (macos-14), each with the web app built first; smoke-tests each binary: `repoview --version` equals
   `repoview <tag>`, `repoview snapshot --format json --root .` exits 0 with `vcs` `Present`, and
   `repoview open --no-browser --port 0` serves `/` with the embedded `index.html` (HTTP 200,
   `text/html`); uploads the three archives and `SHA256SUMS` as workflow artifacts. Permissions are
   `contents: read`; no workflow publishes a release.
3. Every third-party action is pinned to a full commit SHA (as in
   `codegate/.github/workflows/release-build.yml`); `persist-credentials: false` on checkout.
4. A Rust test in `crates/repoview/tests/workflows.rs` parses both workflow files (YAML) and asserts:
   pinned actions only, read-only permissions, the three targets, the smoke-test commands present, the
   artifact names. The real proof is the CI run on the wave's pull request, which the coordinator
   records.
5. `AGENTS.md` § Publishing gains the release steps: tag, wait for `release-build.yml`, download its
   artifacts, `sha256sum -c SHA256SUMS`, publish the GitHub Release and assets through
   `b10x-gates api` as `b10x-bot[bot]` (coordinator writes this; not the unit).

## Out of scope

Publishing a release; Homebrew or other package channels; Windows.
