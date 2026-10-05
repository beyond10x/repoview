//! story:repository-page: the served binary hands the project to the Repository routes, so
//! `/api/vcs` and `/api/docs` answer for `--root` (they are 500 without the `Env` extension).

mod common;

use std::fs;

use common::{Server, canonical, git, git_repo_with_commit};

#[test]
fn served_binary_answers_the_repository_routes_for_its_root() {
    let project = git_repo_with_commit();
    fs::write(project.path().join("README.md"), "# served\n").unwrap();
    let head = git(project.path(), &["rev-parse", "HEAD"]);
    let root = canonical(project.path());
    let server = Server::start(&["open", "--no-browser", "--root", &root]);
    let token = [("X-Repoview-Token", server.token.as_str())];

    let vcs = server.get("/api/vcs", &token);
    assert_eq!(vcs.status, 200, "{}", vcs.text());
    let vcs: serde_json::Value = serde_json::from_slice(&vcs.body).unwrap();
    assert_eq!(vcs["head"], head);

    let doc = server.get("/api/docs/README.md", &token);
    assert_eq!(doc.status, 200, "{}", doc.text());
    let doc: serde_json::Value = serde_json::from_slice(&doc.body).unwrap();
    assert_eq!(doc["markdown"], "# served\n");
}
