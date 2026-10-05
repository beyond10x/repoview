//! The CI and release workflows, read as data (story:ci-binaries).
//!
//! GitHub runs them; these cases catch the drift a run would only show later: an action that is
//! not pinned, a token that can write, a target or a smoke check that went missing, a toolchain
//! that no longer matches `rust-toolchain.toml` or `web/package.json`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_yaml_ng::Value;

const CI: &str = "ci.yml";
const RELEASE: &str = "release-build.yml";

const MAIN_PUSH: &str = "github.event_name == 'push' && github.ref == 'refs/heads/main'";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn workflows_dir() -> PathBuf {
    repo_root().join(".github/workflows")
}

fn parse(path: &Path) -> Value {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_yaml_ng::from_str(&text)
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}

fn workflow(name: &str) -> Value {
    parse(&workflows_dir().join(name))
}

/// Every workflow file in `.github/workflows`, by file name.
fn all_workflows() -> Vec<(String, Value)> {
    let mut files: Vec<_> = std::fs::read_dir(workflows_dir())
        .expect("read .github/workflows")
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| {
            matches!(
                path.extension().and_then(|ext| ext.to_str()),
                Some("yml" | "yaml")
            )
        })
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            (name, parse(&path))
        })
        .collect()
}

fn text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        Value::Bool(flag) => flag.to_string(),
        other => panic!("expected a scalar, got {other:?}"),
    }
}

fn jobs(workflow: &Value) -> Vec<(String, &Value)> {
    workflow["jobs"]
        .as_mapping()
        .expect("jobs is a mapping")
        .iter()
        .map(|(id, job)| (text(id), job))
        .collect()
}

fn job<'a>(workflow: &'a Value, id: &str) -> &'a Value {
    let job = &workflow["jobs"][id];
    assert!(job.is_mapping(), "job `{id}` is missing");
    job
}

fn steps(job: &Value) -> &[Value] {
    job["steps"].as_sequence().map_or(&[], Vec::as_slice)
}

fn all_steps(workflow: &Value) -> Vec<&Value> {
    jobs(workflow)
        .into_iter()
        .flat_map(|(_, job)| steps(job))
        .collect()
}

fn run(step: &Value) -> &str {
    step["run"].as_str().unwrap_or("")
}

fn uses(step: &Value) -> &str {
    step["uses"].as_str().unwrap_or("")
}

/// The index of the first step in `job` whose `run` contains `needle`.
fn step_running(job: &Value, needle: &str) -> usize {
    steps(job)
        .iter()
        .position(|step| run(step).contains(needle))
        .unwrap_or_else(|| panic!("no step runs `{needle}`"))
}

/// The steps in `job` that use the action `name` (`owner/repo`).
fn steps_using<'a>(job: &'a Value, name: &str) -> Vec<(usize, &'a Value)> {
    let prefix = format!("{name}@");
    steps(job)
        .iter()
        .enumerate()
        .filter(|(_, step)| uses(step).starts_with(&prefix))
        .collect()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn toolchain_channel() -> String {
    let text = std::fs::read_to_string(repo_root().join("rust-toolchain.toml"))
        .expect("read rust-toolchain.toml");
    text.lines()
        .find_map(|line| {
            let (key, value) = line.split_once('=')?;
            (key.trim() == "channel").then(|| value.trim().trim_matches('"').to_owned())
        })
        .expect("rust-toolchain.toml names a channel")
}

fn pnpm_version() -> String {
    let text = std::fs::read_to_string(repo_root().join("web/package.json"))
        .expect("read web/package.json");
    let package: serde_json::Value = serde_json::from_str(&text).expect("parse web/package.json");
    let manager = package["packageManager"]
        .as_str()
        .expect("web/package.json names its packageManager");
    manager
        .strip_prefix("pnpm@")
        .unwrap_or_else(|| panic!("packageManager is not pnpm: {manager}"))
        .split('+')
        .next()
        .unwrap()
        .to_owned()
}

/// `owner/repo[/path]@<40 hex>`: a full commit SHA, never a tag or a branch.
fn pinned(reference: &str) -> bool {
    let Some((action, sha)) = reference.split_once('@') else {
        return false;
    };
    action.split('/').count() >= 2
        && !action.starts_with('.')
        && sha.len() == 40
        && sha
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

#[test]
fn every_action_and_reusable_workflow_is_pinned_to_a_full_commit_sha() {
    let mut seen = 0;
    for (file, workflow) in all_workflows() {
        for (id, job) in jobs(&workflow) {
            if let Some(reference) = job["uses"].as_str() {
                seen += 1;
                assert!(pinned(reference), "{file} job `{id}` uses `{reference}`");
            }
            for step in steps(job) {
                if step["uses"].is_null() {
                    continue;
                }
                seen += 1;
                let reference = uses(step);
                assert!(pinned(reference), "{file} job `{id}` uses `{reference}`");
            }
        }
    }
    assert!(seen > 0, "no `uses` found: the walk reads nothing");
    for name in [CI, RELEASE] {
        assert!(
            all_steps(&workflow(name))
                .iter()
                .any(|step| !step["uses"].is_null()),
            "{name} uses no action"
        );
    }
}

#[test]
fn every_checkout_keeps_no_credentials() {
    let mut checkouts = 0;
    for (file, workflow) in all_workflows() {
        for (id, job) in jobs(&workflow) {
            for (_, step) in steps_using(job, "actions/checkout") {
                checkouts += 1;
                assert_eq!(
                    step["with"]["persist-credentials"].as_bool(),
                    Some(false),
                    "{file} job `{id}`: checkout without `persist-credentials: false`"
                );
            }
        }
    }
    assert!(
        checkouts >= 2,
        "ci.yml and release-build.yml each check out"
    );
}

#[test]
fn permissions_are_contents_read_and_no_job_widens_them() {
    for name in [CI, RELEASE] {
        let workflow = workflow(name);
        let permissions = workflow["permissions"]
            .as_mapping()
            .unwrap_or_else(|| panic!("{name} sets no top-level permissions mapping"));
        let granted: Vec<(String, String)> = permissions
            .iter()
            .map(|(scope, level)| (text(scope), text(level)))
            .collect();
        assert_eq!(
            granted,
            [("contents".to_owned(), "read".to_owned())],
            "{name} permissions"
        );
        for (id, job) in jobs(&workflow) {
            assert!(
                job["permissions"].is_null(),
                "{name} job `{id}` sets its own permissions"
            );
        }
    }
}

#[test]
fn no_workflow_publishes_a_release() {
    for (file, workflow) in all_workflows() {
        for step in all_steps(&workflow) {
            let action = uses(step).to_ascii_lowercase();
            assert!(
                !action.contains("release"),
                "{file} uses a release action: {action}"
            );
            // Downloading a tool from a release is reading; anything else naming releases is not.
            let script = run(step).replace("/releases/download/", "");
            for needle in ["gh release", "/releases", "gh api", "api.github.com"] {
                assert!(
                    !script.contains(needle),
                    "{file} step writes a release (`{needle}`): {script}"
                );
            }
        }
    }
}

/// Context values reach scripts through `env`, never through `${{ }}` inside `run` (a tag name
/// is attacker-chosen text).
#[test]
fn run_scripts_read_context_through_env() {
    for name in [CI, RELEASE] {
        for step in all_steps(&workflow(name)) {
            assert!(
                !run(step).contains("${{"),
                "{name}: expression inside a run script: {}",
                run(step)
            );
        }
    }
}

#[test]
fn ci_runs_on_pull_requests_and_pushes_to_main() {
    let ci = workflow(CI);
    let on = ci["on"].as_mapping().expect("ci.yml `on` is a mapping");
    let events: BTreeSet<String> = on.keys().map(text).collect();
    assert!(events.contains("pull_request"), "events: {events:?}");
    assert!(events.contains("push"), "events: {events:?}");
    let branches: Vec<String> = ci["on"]["push"]["branches"]
        .as_sequence()
        .expect("push.branches")
        .iter()
        .map(text)
        .collect();
    assert_eq!(branches, ["main"]);
    assert!(
        ci["on"]["push"]["tags"].is_null(),
        "ci.yml does not run on tags"
    );
    // Only a pull request's superseded run may be cancelled; every main push builds.
    assert_eq!(
        ci["concurrency"]["cancel-in-progress"].as_str(),
        Some("${{ github.event_name == 'pull_request' }}")
    );
}

/// The smoke step's cleanup: the one line allowed to swallow a failure (the server may already
/// have exited).
const SMOKE_TRAP: &str = r#"trap 'kill "$server" 2>/dev/null || true' EXIT"#;

/// The byte offset just past `word` at the start of `text`, if `text` starts with `word` and the
/// next character cannot continue a shell word.
fn word_at(text: &str, word: &str) -> Option<usize> {
    let rest = text.strip_prefix(word)?;
    let continues = rest
        .chars()
        .next()
        .is_some_and(|next| next.is_ascii_alphanumeric() || next == '_' || next == '-');
    (!continues).then_some(word.len())
}

/// How many `|| true`, `|| :` and `|| exit 0` (any spacing) `line` holds.
fn swallows(line: &str) -> usize {
    line.match_indices("||")
        .filter(|(at, _)| {
            let rest = line[at + 2..].trim_start();
            if word_at(rest, "true").is_some() || word_at(rest, ":").is_some() {
                return true;
            }
            let Some(after) = word_at(rest, "exit") else {
                return false;
            };
            let code = &rest[after..];
            code.starts_with(char::is_whitespace) && word_at(code.trim_start(), "0").is_some()
        })
        .count()
}

/// Whether `line` turns off `errexit` or `pipefail`: `set +e`, `set +eu`, `set +o errexit`,
/// `set +o pipefail`, any spacing.
fn disables_errexit(line: &str) -> bool {
    let words: Vec<&str> = line
        .split(|c: char| c.is_whitespace() || c == ';')
        .filter(|word| !word.is_empty())
        .collect();
    words.iter().enumerate().any(|(at, word)| {
        *word == "set"
            && words[at + 1..]
                .iter()
                .take_while(|option| option.starts_with(['+', '-']) || is_set_option(option))
                .enumerate()
                .any(|(offset, option)| {
                    let flags = option.strip_prefix('+').unwrap_or("");
                    (flags != "o" && flags.contains('e'))
                        || (*option == "+o"
                            && words
                                .get(at + 2 + offset)
                                .is_some_and(|name| matches!(*name, "errexit" | "pipefail")))
                })
    })
}

fn is_set_option(word: &str) -> bool {
    word.bytes().all(|byte| byte.is_ascii_lowercase()) && !word.is_empty()
}

/// Whether `script` holds a pipeline: a `|` that is not half of `||`.
fn has_pipe(script: &str) -> bool {
    let bytes = script.as_bytes();
    bytes.iter().enumerate().any(|(at, byte)| {
        *byte == b'|' && bytes.get(at + 1) != Some(&b'|') && (at == 0 || bytes[at - 1] != b'|')
    })
}

#[test]
fn the_swallow_detectors_catch_every_form() {
    for line in [
        "task check || true",
        "task check ||true",
        "task check ||   true;",
        "task check || :",
        "task check ||:",
        "task check || exit 0",
        "task check ||exit   0",
        r#"trap 'kill "$server" 2>/dev/null || true' EXIT; false || true"#,
    ] {
        assert!(swallows(line) >= 1, "missed: {line}");
    }
    assert_eq!(
        swallows(r#"trap 'kill "$server" 2>/dev/null || true' EXIT; false || true"#),
        2
    );
    assert_eq!(swallows(SMOKE_TRAP), 1);
    for line in [
        "task check || exit 1",
        "task check || exit",
        "task check || true_fallback",
        "task check || exit 0x",
        "a | b",
        "test -n \"$x\" || echo missing >&2",
    ] {
        assert_eq!(swallows(line), 0, "false alarm: {line}");
    }
    for line in [
        "set +e",
        "set  +e",
        "set +eu",
        "set -u +e",
        "set +o errexit",
        "set   +o   errexit",
        "set +o pipefail",
        "true; set +e",
    ] {
        assert!(disables_errexit(line), "missed: {line}");
    }
    for line in [
        "set -e",
        "set -o pipefail",
        "set -euo pipefail",
        "set +x",
        "set +o xtrace",
    ] {
        assert!(!disables_errexit(line), "false alarm: {line}");
    }
    assert!(has_pipe("echo x | sha256sum --check"));
    assert!(!has_pipe("a || b"));
}

/// A failed check fails its run. Nothing in either workflow may swallow a failure: no
/// `continue-on-error` on a job or step, no job-level `if`, no step-level `if` except the
/// main-push build, package and upload in ci.yml, no `shell` override (GitHub's default for a
/// `run` step is `bash -e {0}`; an override can drop `-e`), nothing that turns off `errexit` or
/// `pipefail`, and no `|| true`, `|| :` or `|| exit 0` except [`SMOKE_TRAP`]. A script with a
/// pipeline starts with `set -o pipefail`, which the default shell does not set.
#[test]
fn no_step_or_job_can_pass_over_a_failed_check() {
    for name in [CI, RELEASE] {
        let workflow = workflow(name);
        let mut conditional = 0;
        let mut traps = 0;
        for (id, job) in jobs(&workflow) {
            assert!(
                job["continue-on-error"].is_null(),
                "{name} job `{id}` has continue-on-error"
            );
            assert!(job["if"].is_null(), "{name} job `{id}` has an `if`");
            assert!(
                job["defaults"].is_null(),
                "{name} job `{id}` sets defaults (a shell override)"
            );
            for step in steps(job) {
                let label = step["name"]
                    .as_str()
                    .map_or_else(|| uses(step).to_owned(), str::to_owned);
                assert!(
                    step["continue-on-error"].is_null(),
                    "{name} step `{label}` has continue-on-error"
                );
                assert!(
                    step["shell"].is_null(),
                    "{name} step `{label}` overrides the shell"
                );
                if !step["if"].is_null() {
                    conditional += 1;
                    assert_eq!(name, CI, "{name} step `{label}` has an `if`");
                    assert_eq!(
                        step["if"].as_str(),
                        Some(MAIN_PUSH),
                        "{name} step `{label}` has a condition other than a main push"
                    );
                }
                let script = run(step);
                for line in script.lines() {
                    assert!(
                        !disables_errexit(line),
                        "{name} step `{label}` turns off errexit or pipefail: {line}"
                    );
                    if line.trim() == SMOKE_TRAP {
                        traps += 1;
                        continue;
                    }
                    assert_eq!(
                        swallows(line),
                        0,
                        "{name} step `{label}` swallows a failure: {line}"
                    );
                }
                if has_pipe(script) {
                    assert_eq!(
                        script.lines().find(|line| !line.trim().is_empty()),
                        Some("set -o pipefail"),
                        "{name} step `{label}` pipes without `set -o pipefail` first"
                    );
                }
            }
        }
        assert!(
            workflow["defaults"].is_null(),
            "{name} sets defaults (a shell override)"
        );
        let expected = if name == CI { 3 } else { 0 };
        assert_eq!(conditional, expected, "{name}: conditional steps");
        let expected = if name == RELEASE { 1 } else { 0 };
        assert_eq!(traps, expected, "{name}: the smoke step's cleanup trap");
    }
}

#[test]
fn ci_installs_the_pinned_tools_and_runs_task_check() {
    let ci = workflow(CI);
    let gate = job(&ci, "check");

    let toolchain = steps_using(gate, "dtolnay/rust-toolchain");
    assert_eq!(toolchain.len(), 1, "one Rust toolchain step");
    assert_eq!(
        text(&toolchain[0].1["with"]["toolchain"]),
        toolchain_channel()
    );
    assert_eq!(toolchain_channel(), "1.98.1");

    let node = steps_using(gate, "actions/setup-node");
    assert_eq!(node.len(), 1, "one Node step");
    assert_eq!(text(&node[0].1["with"]["node-version"]), "22");

    let pnpm = &steps(gate)[step_running(gate, "npm install --global \"pnpm@$PNPM_VERSION\"")];
    assert_eq!(text(&pnpm["env"]["PNPM_VERSION"]), pnpm_version());

    let task = &steps(gate)[step_running(gate, "github.com/go-task/task/releases/download/")];
    assert!(run(task).contains("task_linux_amd64.tar.gz"));
    assert!(is_sha256(&text(&task["env"]["TASK_SHA256"])));
    assert!(run(task).contains("$TASK_SHA256") && run(task).contains("--check"));

    for (tool, var) in [("aep", "AEP"), ("ess", "ESS")] {
        let at = step_running(
            gate,
            &format!("github.com/beyond10x/{tool}/releases/download/"),
        );
        let step = &steps(gate)[at];
        let version = text(&step["env"][&format!("{var}_VERSION")]);
        assert!(
            version.split('.').count() == 3,
            "{tool} version `{version}`"
        );
        assert!(
            is_sha256(&text(&step["env"][&format!("{var}_SHA256")])),
            "{tool} has no SHA-256"
        );
        assert!(
            run(step).contains(&format!("${var}_SHA256"))
                && run(step).contains("sha256sum --check"),
            "{tool} download is not checksum-verified"
        );
        assert!(
            run(step).contains(&format!("{tool}-${var}_VERSION-x86_64-unknown-linux-gnu"))
                && run(step).contains(".tar.gz"),
            "{tool} archive"
        );
    }

    let check = step_running(gate, "task check");
    for install in [
        "npm install --global",
        "github.com/go-task/task/releases/download/",
        "github.com/beyond10x/aep/releases/download/",
        "github.com/beyond10x/ess/releases/download/",
    ] {
        assert!(
            step_running(gate, install) < check,
            "`{install}` runs after `task check`"
        );
    }
}

#[test]
fn ci_uploads_a_linux_build_of_each_main_push_for_30_days() {
    let ci = workflow(CI);
    let gate = job(&ci, "check");
    let check = step_running(gate, "task check");

    let build = step_running(gate, "task build");
    assert!(build > check, "the build runs after the gate");
    assert_eq!(steps(gate)[build]["if"].as_str(), Some(MAIN_PUSH));

    let package = step_running(gate, "repoview-${GITHUB_SHA::7}-x86_64-unknown-linux-gnu");
    let script = run(&steps(gate)[package]);
    assert!(package > build);
    assert_eq!(steps(gate)[package]["if"].as_str(), Some(MAIN_PUSH));
    assert_eq!(steps(gate)[package]["id"].as_str(), Some("package"));
    for needle in [
        "target/release/repoview",
        "README.md",
        "[ -f LICENSE ]",
        "tar -czf",
        ".tar.gz",
        "$GITHUB_OUTPUT",
    ] {
        assert!(script.contains(needle), "package step lacks `{needle}`");
    }

    let uploads = steps_using(gate, "actions/upload-artifact");
    assert_eq!(uploads.len(), 1, "one upload");
    let (at, upload) = uploads[0];
    assert!(at > package);
    assert_eq!(upload["if"].as_str(), Some(MAIN_PUSH));
    assert_eq!(
        upload["with"]["name"].as_str(),
        Some("${{ steps.package.outputs.name }}")
    );
    assert_eq!(
        upload["with"]["path"].as_str(),
        Some("dist/${{ steps.package.outputs.name }}.tar.gz")
    );
    assert_eq!(text(&upload["with"]["retention-days"]), "30");
    assert_eq!(upload["with"]["if-no-files-found"].as_str(), Some("error"));
}

/// Acceptance 1: *every* `main` push leaves a Linux build. GitHub keeps one running and one pending
/// run per concurrency group and cancels the pending one when another queues, whatever
/// `cancel-in-progress` says; a group shared by consecutive pushes to `main` therefore drops the
/// build of every push that waited behind a running one and was overtaken by a third.
#[test]
fn adversary_ci_concurrency_never_drops_a_main_push_build() {
    let ci = workflow(CI);
    let concurrency = &ci["concurrency"];
    if concurrency.is_null() {
        return;
    }
    let group = if concurrency.is_string() {
        text(concurrency)
    } else {
        text(&concurrency["group"])
    };
    assert!(
        group.contains("github.sha") || group.contains("github.run_id"),
        "ci.yml concurrency group `{group}` is the same for consecutive pushes to main: a third \
         push cancels the second push's pending run, and that push never uploads its build"
    );
}

#[test]
fn release_build_runs_on_0_tags_only() {
    let release = workflow(RELEASE);
    let on = release["on"]
        .as_mapping()
        .expect("release-build.yml `on` is a mapping");
    let events: Vec<String> = on.keys().map(text).collect();
    assert_eq!(events, ["push"]);
    let tags: Vec<String> = release["on"]["push"]["tags"]
        .as_sequence()
        .expect("push.tags")
        .iter()
        .map(text)
        .collect();
    assert_eq!(tags, ["0.*"]);
    assert!(release["on"]["push"]["branches"].is_null());
}

#[test]
fn release_build_targets_three_platforms_on_their_runners() {
    let release = workflow(RELEASE);
    let build = job(&release, "build");
    assert_eq!(build["runs-on"].as_str(), Some("${{ matrix.runner }}"));
    let include = build["strategy"]["matrix"]["include"]
        .as_sequence()
        .expect("strategy.matrix.include");
    let targets: BTreeSet<(String, String)> = include
        .iter()
        .map(|entry| (text(&entry["target"]), text(&entry["runner"])))
        .collect();
    let expected: BTreeSet<(String, String)> = [
        ("x86_64-unknown-linux-gnu", "ubuntu-22.04"),
        ("aarch64-unknown-linux-gnu", "ubuntu-22.04-arm"),
        ("aarch64-apple-darwin", "macos-14"),
    ]
    .into_iter()
    .map(|(target, runner)| (target.to_owned(), runner.to_owned()))
    .collect();
    assert_eq!(targets, expected);
    assert_eq!(include.len(), 3);
    for entry in include {
        assert!(
            is_sha256(&text(&entry["task-sha256"])),
            "{entry:?} has no go-task SHA-256"
        );
        assert!(text(&entry["task-asset"]).starts_with("task_"));
    }

    let toolchain = steps_using(build, "dtolnay/rust-toolchain");
    assert_eq!(toolchain.len(), 1);
    assert_eq!(
        text(&toolchain[0].1["with"]["toolchain"]),
        toolchain_channel()
    );
    let node = steps_using(build, "actions/setup-node");
    assert_eq!(node.len(), 1);
    assert_eq!(text(&node[0].1["with"]["node-version"]), "22");
    let pnpm = &steps(build)[step_running(build, "npm install --global \"pnpm@$PNPM_VERSION\"")];
    assert_eq!(text(&pnpm["env"]["PNPM_VERSION"]), pnpm_version());
    let task = &steps(build)[step_running(build, "github.com/go-task/task/releases/download/")];
    assert_eq!(
        text(&task["env"]["TASK_SHA256"]),
        "${{ matrix.task-sha256 }}"
    );
    assert!(run(task).contains("--check"));

    // `task build` builds web/dist, then the release binary that embeds it.
    step_running(build, "task build");
}

#[test]
fn release_build_smoke_tests_each_binary_before_packaging_it() {
    let release = workflow(RELEASE);
    let build = job(&release, "build");
    let compile = step_running(build, "task build");
    let smoke = step_running(build, "--version");
    let script = run(&steps(build)[smoke]);
    for needle in [
        "\"repoview $GITHUB_REF_NAME\"",
        "snapshot --format json --root .",
        "select(.source_id == \"vcs\" and .availability == \"Present\")",
        "open --no-browser --port 0",
        "200",
        "content-type: text/html",
        "<meta name=\"repoview-mode\" content=\"server\"",
    ] {
        assert!(script.contains(needle), "smoke step lacks `{needle}`");
    }
    // Without `--exit-status`, jq exits 0 when it prints `false`.
    let jq_lines: Vec<&str> = script
        .lines()
        .filter(|line| line.trim_start().starts_with("jq "))
        .collect();
    assert!(!jq_lines.is_empty(), "smoke step runs no jq");
    for line in jq_lines {
        assert!(
            line.contains("--exit-status") || line.contains(" -e "),
            "jq without --exit-status: {line}"
        );
    }
    let package = step_running(build, "repoview-$GITHUB_REF_NAME-$TARGET");
    assert!(compile < smoke && smoke < package, "build, smoke, package");
}

#[test]
fn release_build_uploads_three_archives_and_sha256sums() {
    let release = workflow(RELEASE);
    let build = job(&release, "build");
    let package = step_running(build, "repoview-$GITHUB_REF_NAME-$TARGET");
    let step = &steps(build)[package];
    assert_eq!(text(&step["env"]["TARGET"]), "${{ matrix.target }}");
    for needle in [
        "target/release/repoview",
        "README.md",
        "[ -f LICENSE ]",
        "tar -czf",
    ] {
        assert!(run(step).contains(needle), "package step lacks `{needle}`");
    }
    let uploads = steps_using(build, "actions/upload-artifact");
    assert_eq!(uploads.len(), 1);
    let (at, upload) = uploads[0];
    assert!(at > package);
    assert_eq!(
        upload["with"]["name"].as_str(),
        Some("repoview-${{ github.ref_name }}-${{ matrix.target }}")
    );
    assert_eq!(upload["with"]["if-no-files-found"].as_str(), Some("error"));

    let sums = job(&release, "checksums");
    assert_eq!(text(&sums["needs"]), "build");
    let downloads = steps_using(sums, "actions/download-artifact");
    assert_eq!(downloads.len(), 1);
    assert_eq!(
        downloads[0].1["with"]["pattern"].as_str(),
        Some("repoview-${{ github.ref_name }}-*")
    );
    let write = step_running(sums, "sha256sum *.tar.gz > SHA256SUMS");
    assert!(
        run(&steps(sums)[write]).contains("-eq 3"),
        "exactly three archives"
    );
    let uploads = steps_using(sums, "actions/upload-artifact");
    assert_eq!(uploads.len(), 1);
    let (at, upload) = uploads[0];
    assert!(at > write);
    assert_eq!(
        upload["with"]["name"].as_str(),
        Some("repoview-${{ github.ref_name }}-release")
    );
    let path = upload["with"]["path"].as_str().unwrap_or("");
    assert!(path.contains("dist/*.tar.gz") && path.contains("dist/SHA256SUMS"));
    assert_eq!(upload["with"]["if-no-files-found"].as_str(), Some("error"));
}
