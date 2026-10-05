---
format: aep.planning-md/3
id: epic:release
kind: epic
status: draft
title: Downloadable binaries from GitHub Actions
summary: CI uploads a Linux binary on every main push; a tag builds three targets with the SPA embedded and SHA256SUMS.
relations:
- serves: vision:repoview
- implements: architecture-design:repoview
- depends_on: epic:shell
revision: 1
---
## Outcome

A pushed tag produces downloadable `repoview` binaries with the SPA embedded, and every `main` push
produces a Linux binary as a workflow artifact.

## Scope

- `.github/workflows/ci.yml`: `task check` on pull requests and `main`; on `main`, build the web app,
  build `repoview` in release mode and upload `repoview-<sha>-x86_64-unknown-linux-gnu.tar.gz`.
- `.github/workflows/release-build.yml`: on tag `0.*`, build for `x86_64-unknown-linux-gnu`,
  `aarch64-unknown-linux-gnu` and `aarch64-apple-darwin`, smoke-test each, upload the archives and
  `SHA256SUMS`.
- The release procedure in `AGENTS.md`: verify the tag run's checksums, publish the GitHub Release
  and assets through `b10x-gates api` as `b10x-bot[bot]`.

## Acceptance

- The `main` workflow artifact, extracted on a machine with no Node and no checkout, runs
  `repoview open --no-browser` and serves the SPA's `index.html` at `/`.
- `repoview --version` in each tagged archive prints the tag.
- `sha256sum -c SHA256SUMS` passes over the release assets.

## Depends on

`epic:shell`.
