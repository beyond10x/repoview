---
format: aep.planning-md/3
id: story:docs-site
kind: story
status: active
title: Public docs site with a live export of repoview itself
summary: Docusaurus site at beyond10x.github.io/repoview in Loom's shape, repoview-docs crate for the generated CLI reference and status, and a repoview export of this repository under /repoview/demo/.
relations:
- decomposes: epic:static-export
- serves: vision:repoview
- depends_on: story:static-export
scope:
- confidence: cited
  path: .github/workflows/b10x-docs-site.yml
- confidence: cited
  path: .github/workflows/pages.yml
- confidence: cited
  path: .gitignore
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/repoview-docs/
- confidence: cited
  path: crates/repoview/src/cli.rs
- confidence: cited
  path: crates/repoview/src/lib.rs
- confidence: cited
  path: crates/repoview/src/main.rs
- confidence: cited
  path: crates/repoview/tests/workflows.rs
- confidence: cited
  path: website/
revision: 15
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T00:15:18Z", actor: "human:timo", revision: 14}
- {from: "proposed", to: "active", at: "2026-10-06T00:15:18Z", actor: "human:timo", revision: 15}
---
## Story

As someone deciding whether to use repoview, I open `https://beyond10x.github.io/repoview/` and find
what it is, how to install and run it, what each page shows, the CLI reference, what is shipped,
and a live example: repoview's own repository, exported with `repoview export`, browsable in the
same site.

## Acceptance

1. `website/` is a Docusaurus site in the shape of Loom's (`loom/website/`): `docusaurus.config.ts`
   with `url: 'https://beyond10x.github.io'`, `baseUrl: '/repoview/'`, `projectName: 'repoview'`,
   `withProductSite` from `@beyond10x/docs-system` pinned to the commit Loom pins
   (`9d35e6324653d659e6ab3d46b761ee4b3bf8405c` on 2026-10-05; re-read `loom/website/package.json`),
   `product.json` (`b10x-product-landing/1`), `package-lock.json` committed.
2. Pages under `website/docs/`: `index.md` (what it is, what it is not, neighbours: AEP, ESS,
   Codegate, each linked to its public docs and its GitHub repository), `getting-started.md` (install
   from a release archive or build with `task build`, then `repoview` in a project, with real
   output), `concepts/sources.md` (the five sources, availability states, read-through-the-owning-CLI),
   `concepts/security.md` (127.0.0.1, run token, Host check, read-only, untrusted repository content),
   `guides/static-export.md`, `guides/ci-binaries.md`, `reference/cli.md` (generated),
   `status.mdx` + `data/status.json` (generated). Front matter as the `docs` skill §2 says;
   maturity marked with `:::caution[Planned]` where needed.
3. `crates/repoview-docs` (workspace member, `publish = false`) with `generate`, `generate --check`
   and `provenance --site <dir> --commit <sha>`: the CLI reference from the clap `Cli` (moved from
   `main.rs` into a `cli` module of the `repoview` library so the docs crate can depend on it), and
   `data/status.json` (`b10x-status/1`) from a capability list in the crate where every `shipped`
   item names its test as `file::function`, which `generate` checks exists. `task check` runs
   `cargo run -p repoview-docs -- generate --check`.
4. The live example: `.github/workflows/pages.yml` (workflow name `Documentation validation`) builds
   the web app and `repoview` (`task build`), runs `repoview export --out website/static/demo --root .`,
   then `npm --prefix website ci && npm --prefix website run build`, `cargo test -p repoview-docs`,
   `repoview-docs provenance --site website/build --commit "$GITHUB_SHA"`, and on a push to `main`
   uploads `website/build` as `b10x-project-site`. The export is not committed (`website/static/demo/`
   git-ignored). The docs link the example at `/repoview/demo/` (served as static files, hash routes).
5. `.github/workflows/b10x-docs-site.yml` (workflow name `Documentation site`) is Loom's caller with
   `repository: repoview`, `route_base: /repoview/`, the same `project-site.yml` pin Loom uses
   (`fb4024ef7846729e5456591b9070db3d48c87e64` on 2026-10-05), guarded on
   `github.repository == 'beyond10x/repoview'` and the bot as actor and triggering actor.
6. Every command shown on a page was run in this tree and its output pasted; the site build passes
   with `onBrokenLinks: 'throw'`.
7. `tests/workflows.rs` (from story:ci-binaries) covers the two new workflows' pins and permissions.

## Out of scope

Stage C (Atlas moves repoview from undocumented to independent documentation) and Stage D (live
checks): the coordinator does both after this lands. README and AGENTS.md updates: coordinator.
