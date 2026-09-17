//! CI-4 — the test-vector manifest is actually consumed.
//!
//! Why this exists: the vector file declares 33 vectors "供其他兼容实现验证使用"
//! and nothing loaded it. The suites were green the whole time, so a fully
//! unexercised interoperability contract looked exactly like a covered one
//! (K-013). A test that exists is not the same as a test that runs and judges.
//!
//! Two directions, because both failures are real:
//!
//!   * a vector nothing exercises  -> the contract is not being checked
//!   * a test naming a missing id  -> the test points at a phantom
//!
//! Scope is deliberately the TEST SOURCES, not the whole crate: the generator in
//! `examples/` emits these ids, so counting it would let every vector pass by
//! virtue of having been written rather than read. The whole point is to notice
//! that nothing reads them.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The declared vector file, relative to the crate root.
const VECTORS: &str = "tests/test_vectors/ci-144-v2.0-test-vectors.json";

/// Directories whose `.rs` files count as "exercising" a vector.
/// `examples/` is excluded on purpose — see the module comment.
const EXERCISING_DIRS: [&str; 1] = ["tests"];

/// A vector id that appears in a test source. Ids are matched as whole tokens so
/// that `pfp-all_zero` does not also match a longer id that contains it.
fn declared_ids(json: &serde_json::Value) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let vectors = json
        .get("vectors")
        .and_then(|v| v.as_object())
        .unwrap_or_else(|| panic!("{VECTORS}: no `vectors` object — the manifest shape changed"));
    for (category, list) in vectors {
        let list = list.as_array().unwrap_or_else(|| {
            panic!("{VECTORS}: category {category:?} is not an array")
        });
        for v in list {
            let id = v
                .get("id")
                .and_then(|i| i.as_str())
                .unwrap_or_else(|| panic!("{VECTORS}: a vector in {category:?} has no string `id`"));
            out.insert(id.to_string());
        }
    }
    assert!(
        !out.is_empty(),
        "{VECTORS}: zero vectors parsed — refusing to report a vacuous pass"
    );
    out
}

/// `<id>` -> its manifest category. Waiver keys use this pair, so the mapping
/// has to come from the file rather than from the id's spelling.
fn category_map(json: &serde_json::Value) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    if let Some(vectors) = json.get("vectors").and_then(|v| v.as_object()) {
        for (category, list) in vectors {
            if let Some(list) = list.as_array() {
                for v in list {
                    if let Some(id) = v.get("id").and_then(|i| i.as_str()) {
                        out.insert(id.to_string(), category.clone());
                    }
                }
            }
        }
    }
    out
}

/// Every `.rs` file under the exercising directories, recursively.
fn exercising_sources(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// Vector-shaped ids mentioned anywhere in the exercising sources.
/// Shape: `<category>-<name>`, matched as whole tokens.
fn referenced_ids(sources: &[PathBuf]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for path in sources {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        for token in text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_')) {
            // Both sides of the hyphen must be non-empty: a fragment like
            // `catastrophic-` comes from this file's own source (`c == '-'`
            // inside a character set) and is not an id. Without this the
            // checker accuses itself and reports a phantom nobody can find.
            let (head, tail) = match token.split_once('-') {
                Some((h, t)) => (h, t),
                None => continue,
            };
            if head.is_empty() || tail.is_empty() {
                continue;
            }
            if token.ends_with('-') || token.starts_with('-') {
                continue;
            }
            // A category is a lowercase word run; the id continues after the
            // first hyphen. This keeps prose like `run-abc` out of the set by
            // requiring the id to be quoted-looking (`-` plus a `_` or a word)
            // is NOT enough, so instead we only accept tokens that a vector
            // could plausibly be: lowercase, digits, `-`, `_`.
            if token.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_') {
                out.insert(token.to_string());
            }
        }
    }
    out
}


// ─── the shared waiver file ────────────────────────────────────────────────
//
// Waivers live outside the artefact they protect, carry an owner and a due
// date, and stop working on that date: a waiver that renews itself is a
// permission, and the point is that it is a debt.

const BASELINE: &str = "ci/baseline.toml";

/// Ids waived by `check = "CI-4"`, as `category/id`, EXCLUDING those past due.
/// An overdue waiver is reported separately so its presence is visible rather
/// than silently failing the build with no explanation.
fn waived_and_expired(today: (i32, u32, u32)) -> (BTreeSet<String>, Vec<String>) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let Ok(text) = fs::read_to_string(root.join(BASELINE)) else {
        return (BTreeSet::new(), Vec::new());
    };
    let mut waived = BTreeSet::new();
    let mut expired = Vec::new();
    // Minimal parse of the `[[waiver]]` form: one `key = "value"` per line.
    // A full TOML parser is not worth a dependency for five keys.
    let mut check = String::new();
    let mut target = String::new();
    let mut due = String::new();
    let mut flush = |check: &str, target: &str, due: &str, waived: &mut BTreeSet<String>, expired: &mut Vec<String>| {
        if check != "CI-4" || target.is_empty() {
            return;
        }
        match parse_ymd(due) {
            Some(d) if d >= today => {
                waived.insert(target.to_string());
            }
            Some(_) => expired.push(target.to_string()),
            None => expired.push(format!("{target} (unparseable due: {due:?})")),
        }
    };
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            if line == "[[waiver]]" {
                flush(&check, &target, &due, &mut waived, &mut expired);
                check.clear();
                target.clear();
                due.clear();
            }
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        let v = v.trim().trim_matches('"').to_string();
        match k.trim() {
            "check" => check = v,
            "target" => target = v,
            "due" => due = v,
            _ => {}
        }
    }
    flush(&check, &target, &due, &mut waived, &mut expired);
    (waived, expired)
}

fn parse_ymd(s: &str) -> Option<(i32, u32, u32)> {
    let mut it = s.split('-');
    let y = it.next()?.parse().ok()?;
    let m = it.next()?.parse().ok()?;
    let d = it.next()?.parse().ok()?;
    Some((y, m, d))
}

/// Today, from the environment so the check is deterministic in CI and can be
/// pinned in a test. `CI_TODAY=2026-10-19` forces an expiry.
fn today() -> (i32, u32, u32) {
    if let Ok(v) = std::env::var("CI_TODAY") {
        if let Some(d) = parse_ymd(&v) {
            return d;
        }
    }
    // No clock dependency in the checker: the date is an input.
    (2026, 9, 18)
}

#[test]
fn every_declared_vector_is_exercised_by_a_test() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let raw = fs::read_to_string(root.join(VECTORS))
        .unwrap_or_else(|e| panic!("cannot read {VECTORS}: {e}"));
    let json: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("{VECTORS}: invalid JSON: {e}"));

    let declared = declared_ids(&json);
    let category_of = category_map(&json);
    let sources = exercising_sources(&root.join(EXERCISING_DIRS[0]));
    assert!(
        !sources.is_empty(),
        "no test sources found under {} — the check would pass vacuously",
        EXERCISING_DIRS[0]
    );
    let referenced = referenced_ids(&sources);

    let (waived, expired) = waived_and_expired(today());
    assert!(
        expired.is_empty(),
        "{} CI-4 waiver(s) are past due and no longer exempt anything:\n  {}\n\
         An exemption is a debt with a date. Fix the target or re-justify it in \
         {BASELINE} — do not extend it silently.",
        expired.len(),
        expired.join("\n  ")
    );

    // The waiver key is the manifest's OWN `<category>/<id>`, not a prefix
    // guessed off the id: real categories are `catastrophic_detection`, while
    // the id starts `catastrophic-`. Guessing produced keys that matched
    // nothing and made every waiver silently inert.
    let unexercised: Vec<String> = declared
        .iter()
        .map(|id| {
            let category = category_of.get(id).map(String::as_str).unwrap_or("");
            format!("{category}/{id}")
        })
        .filter(|key| {
            let id = key.rsplit_once('/').map(|(_, id)| id).unwrap_or("");
            !referenced.contains(id)
        })
        .filter(|key| !waived.contains(key))
        .collect();
    assert!(
        unexercised.is_empty(),
        "{} of {} declared vectors are never referenced by a test:\n  {}\n\
         Each one is an interoperability claim nothing checks. Either exercise it \
         or delete it — do not leave it declared and unread.",
        unexercised.len(),
        declared.len(),
        unexercised.join("\n  ")
    );
}

#[test]
fn a_test_does_not_reference_a_vector_that_does_not_exist() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let raw = fs::read_to_string(root.join(VECTORS))
        .unwrap_or_else(|e| panic!("cannot read {VECTORS}: {e}"));
    let json: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("{VECTORS}: invalid JSON: {e}"));

    let declared = declared_ids(&json);
    let sources = exercising_sources(&root.join(EXERCISING_DIRS[0]));
    let referenced = referenced_ids(&sources);

    // Only ids whose category is one the manifest actually uses: a test source
    // legitimately mentions unrelated hyphenated names (`run-abc`, `turn/start`),
    // and accusing those would make the check unusable and therefore ignored.
    // The ID PREFIX used by every declared vector ("catastrophic", "frame",
    // "sap", ...). This is what makes a hyphenated token in a test source look
    // like one of ours rather than like `run-abc` or `turn-start`.
    //
    // Using the manifest's parent keys here instead would disable the check
    // silently: those are `catastrophic_detection` / `frame_codec`, which no id
    // starts with, so the filter would match nothing and never fire. That is
    // how this check would have shipped looking green while checking nothing.
    let id_prefixes: BTreeSet<&str> = declared
        .iter()
        .filter_map(|id| id.split_once('-').map(|(c, _)| c))
        .collect();

    let phantom: Vec<&String> = referenced
        .iter()
        .filter(|t| match t.split_once('-') {
            Some((c, _)) => id_prefixes.contains(c),
            None => false,
        })
        .filter(|t| !declared.contains(*t))
        .collect();

    assert!(
        phantom.is_empty(),
        "tests reference vector ids the manifest does not declare:\n  {}",
        phantom
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}
