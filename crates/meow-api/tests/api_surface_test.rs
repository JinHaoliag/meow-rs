//! Freezes the external-controller surface.
//!
//! RClash removed the embedded dashboard and the CLI manager, and the rule
//! that keeps them out is simple: the kernel serves the HTTP/WS control API
//! and *nothing else*. This test is the guard — it parses every
//! `.route("<path>", …)` literal out of `src/routes.rs` and asserts the exact
//! set matches [`EXPECTED`] (`kernel/docs/kernel-api-contract.md` is the prose
//! copy of the same table), then asserts the panel never comes back.
//!
//! Adding, renaming or removing a route therefore fails here first: update
//! both the table and the contract doc in the same commit.

use std::path::{Path, PathBuf};

/// The frozen 0.29.0-minus-panel surface: every `METHOD /path` pair the
/// controller answers, sorted.
const EXPECTED: &[&str] = &[
    "DELETE /api/proxy-groups/{name}",
    "DELETE /api/subscriptions/{name}",
    "DELETE /connections",
    "DELETE /connections/{id}",
    "DELETE /logbook",
    "DELETE /proxies/{name}",
    "DELETE /rules/{index}",
    "DELETE /traffic/ledger",
    "GET /",
    "GET /api/proxy-groups",
    "GET /api/subscriptions",
    "GET /configs",
    "GET /connections",
    "GET /connections/{id}",
    "GET /dns/query",
    "GET /dns/results",
    "GET /group",
    "GET /group/{name}",
    "GET /group/{name}/delay",
    "GET /listeners",
    "GET /logbook",
    "GET /logs",
    "GET /memory",
    "GET /metrics",
    "GET /proxies",
    "GET /proxies/{name}",
    "GET /proxies/{name}/chain-health",
    "GET /proxies/{name}/delay",
    "GET /providers/proxies",
    "GET /providers/proxies/{name}",
    "GET /providers/proxies/{name}/healthcheck",
    "GET /providers/proxies/{provider_name}/{proxy_name}",
    "GET /providers/proxies/{provider_name}/{proxy_name}/healthcheck",
    "GET /providers/rules",
    "GET /providers/rules/{name}",
    "GET /rules",
    "GET /statistics",
    "GET /traffic",
    "GET /traffic/ledger",
    "GET /version",
    "PATCH /configs",
    "POST /api/config/save",
    "POST /api/proxy-groups",
    "POST /api/subscriptions",
    "POST /api/subscriptions/{name}/refresh",
    "POST /cache/dns/flush",
    "POST /cache/fakeip/flush",
    "POST /dns/query",
    "POST /rules",
    "POST /rules/reorder",
    "PUT /api/proxy-groups/{name}",
    "PUT /api/proxy-groups/{name}/select",
    "PUT /configs",
    "PUT /proxies/{name}",
    "PUT /providers/proxies/{name}",
    "PUT /providers/rules/{name}",
    "PUT /rules",
];

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn routes_source() -> String {
    let path = manifest_dir().join("src/routes.rs");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Index of the `)` matching the `(` at `open`, or the end of the input.
fn matching_paren(text: &str, open: usize) -> usize {
    let mut depth = 0usize;
    for (offset, ch) in text[open..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return open + offset;
                }
            }
            _ => {}
        }
    }
    text.len()
}

/// The first double-quoted literal inside `segment`.
fn first_string_literal(segment: &str) -> Option<&str> {
    let start = segment.find('"')? + 1;
    let rest = &segment[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// Every `METHOD /path` pair declared by a `.route("…", …)` call in `source`.
fn route_pairs(source: &str) -> Vec<String> {
    let mut pairs = Vec::new();
    let mut cursor = 0usize;
    while let Some(found) = source[cursor..].find(".route(") {
        let call_start = cursor + found;
        let open = call_start + ".route".len();
        let end = matching_paren(source, open);
        let argument = &source[open + 1..end];
        cursor = end;

        let Some(path) = first_string_literal(argument) else {
            continue;
        };

        for method in argument.match_indices('(') {
            let before = &argument[..method.0];
            let name = before
                .chars()
                .rev()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<String>();
            if matches!(name.as_str(), "get" | "post" | "put" | "patch" | "delete") {
                pairs.push(format!("{} {path}", name.to_uppercase()));
            }
        }
    }
    pairs.sort();
    pairs.dedup();
    pairs
}

#[test]
fn the_controller_surface_is_exactly_the_frozen_table() {
    let actual = route_pairs(&routes_source());

    let expected: Vec<String> = {
        let mut expected: Vec<String> = EXPECTED.iter().map(|s| (*s).to_string()).collect();
        expected.sort();
        expected.dedup();
        expected
    };

    let missing: Vec<&String> = expected.iter().filter(|e| !actual.contains(e)).collect();
    let unexpected: Vec<&String> = actual.iter().filter(|a| !expected.contains(a)).collect();

    assert!(
        missing.is_empty() && unexpected.is_empty(),
        "the external-controller surface drifted from docs/kernel-api-contract.md\n\
         missing (in the doc, not in routes.rs): {missing:?}\n\
         unexpected (in routes.rs, not in the doc): {unexpected:?}\n\
         if the change is intended, update both EXPECTED here and the doc",
    );
}

#[test]
fn the_controller_counts_match_the_table() {
    let actual = route_pairs(&routes_source());
    assert_eq!(
        actual.len(),
        EXPECTED.len(),
        "expected {} method/path pairs, found {}",
        EXPECTED.len(),
        actual.len()
    );
    assert_eq!(
        EXPECTED.len(),
        57,
        "the frozen table is 57 method/route pairs over 42 route entries"
    );
}

#[test]
fn the_panel_and_static_hosting_are_gone() {
    let dir = manifest_dir();

    assert!(
        !dir.join("src/ui.rs").exists(),
        "src/ui.rs is the removed built-in panel; it must not come back"
    );
    assert!(
        !dir.join("static").exists(),
        "crates/meow-api/static/ held the panel assets; the kernel hosts no files"
    );

    // No route, module path or import may reference a UI module again.
    for entry in ["src/routes.rs", "src/lib.rs"] {
        let path: PathBuf = dir.join(entry);
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        assert!(
            !source.contains("ui::"),
            "{entry} still references a `ui::` module"
        );
        assert!(
            !source.contains("ServeDir") && !source.contains("nest_service"),
            "{entry} still mounts a static-file service"
        );
        assert!(
            !source.contains("external_ui"),
            "{entry} still carries external-UI state"
        );
    }
}

#[test]
fn the_contract_doc_exists_and_matches_the_table() {
    // The doc lives in the repository, not the crate: a published `meow-api`
    // has no `docs/`. Skip the cross-check when the crate is built standalone.
    let doc = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/kernel-api-contract.md");
    let Ok(doc) = std::fs::read_to_string(&doc) else {
        return;
    };

    let mut missing = Vec::new();
    for pair in EXPECTED {
        let (method, path) = pair.split_once(' ').expect("METHOD /path");
        let row = format!("| `{method}` | `{path}` |");
        if !doc.contains(&row) {
            missing.push(row);
        }
    }
    assert!(
        missing.is_empty(),
        "docs/kernel-api-contract.md is missing rows (it must mirror EXPECTED): {missing:?}"
    );
}
