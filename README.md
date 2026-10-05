# repoview

Run `repoview` in a project directory and a browser tab opens on that project: its Git state, its
ESS specifications, its AEP plan as a board and a tree of visions, designs, epics and stories, and
its Codegate quality rating. One command, no configuration, read-only.

repoview does not interpret those records itself. It asks the tools that own them (`git`, `aep`,
`ess`, `codegate`) for their JSON output and shows on every panel which tool and version answered.
A source that is absent, or whose tool is missing or fails, says so; it never shows as empty.

> Status: design. Nothing is implemented yet. The design is
> [`.engineering/planning/architecture-design/repoview.md`](.engineering/planning/architecture-design/repoview.md),
> the vision is [`.engineering/planning/vision/repoview.md`](.engineering/planning/vision/repoview.md).

## Use

```console
cd ~/src/some-project
repoview                       # opens the browser
repoview open --no-browser     # prints the URL instead
repoview snapshot --format json
repoview doctor                # which sources were found, which tools answered
```

The server listens on `127.0.0.1` only. The URL it prints carries a token for that run.

## Install

Download the archive for your platform from the
[releases](https://github.com/beyond10x/repoview/releases), check it against `SHA256SUMS`, and put
`repoview` on your `PATH`. The web app is compiled into the binary; there is nothing else to
install. The panels use whichever of `git`, `aep`, `ess` and `codegate` are on your `PATH`.

## Build from source

Requires Rust, Node 22 with pnpm, and [Task](https://taskfile.dev).

```console
task build     # web app, then the release binary with it embedded
task check     # the repository gate
task dev       # Vite dev server against a running repoview
```

## For agents

Operating rules, conventions and the gate are in [`AGENTS.md`](AGENTS.md).
