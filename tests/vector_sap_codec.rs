//! CI-4 batch 1 (second half) — the `sap_codec` vectors, actually loaded.
//!
//! vector-category: sap_codec
//!
//! Correctness is asserted against the golden in `tests/test_vectors/golden/`,
//! not against this file's own expectations.
//!
//! Feasibility was checked before this batch was attempted, because the first
//! batch proved nothing about the others: `pfp_codec` was easy only because
//! `src/pfp.rs` already had a codec. Here the check found a real split —
//! `sap_codec` is pure encode/decode and the implementation exists, so it can be
//! delivered; `pah_signature`, scheduled for the same date, is NOT, because
//! `src/sap.rs` carries a signature field but implements no truncation and no
//! verification (spec `sap:3.7`). That batch is moved rather than promised.

use std::collections::BTreeMap;
use std::path::Path;

use bind19::sap::{SapHeader, PAH_SIZE, SIG_SIZE};

#[derive(serde::Deserialize)]
struct Vector {
    id: String,
    input: Input,
    expected: Expected,
}

#[derive(serde::Deserialize)]
struct Input {
    seq_counter: u16,
    pah_hash_hex: String,
    pah_signature_hex: String,
}

#[derive(serde::Deserialize)]
struct Expected {
    encoded_hex: String,
    encoded_bytes: Vec<u8>,
    size: usize,
}

fn vectors() -> Vec<Vector> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("tests/test_vectors/ci-144-v2.0-test-vectors.json");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let json: serde_json::Value = serde_json::from_str(&raw).expect("manifest is not JSON");
    let list = json
        .get("vectors")
        .and_then(|v| v.get("sap_codec"))
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| panic!("manifest has no `sap_codec` array"));
    let out: Vec<Vector> = list
        .iter()
        .map(|v| serde_json::from_value(v.clone()).unwrap_or_else(|e| panic!("bad vector: {e}")))
        .collect();
    assert_eq!(out.len(), 5, "expected five declared sap_codec vectors");
    out
}

fn unhex<const N: usize>(s: &str, what: &str) -> [u8; N] {
    assert_eq!(s.len(), N * 2, "{what}: wrong hex length in the manifest");
    let mut out = [0u8; N];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
            .unwrap_or_else(|e| panic!("{what}: bad hex at byte {i}: {e}"));
    }
    out
}

fn encode(v: &Vector) -> Vec<u8> {
    SapHeader::new(
        v.input.seq_counter,
        unhex::<PAH_SIZE>(&v.input.pah_hash_hex, "pah_hash_hex"),
        unhex::<SIG_SIZE>(&v.input.pah_signature_hex, "pah_signature_hex"),
    )
    .encode()
    .to_vec()
}

#[test]
fn sap_codec_vectors_encode_as_declared() {
    for v in vectors() {
        let got = encode(&v);
        assert_eq!(got.len(), v.expected.size, "{}: declared size", v.id);
        assert_eq!(got, v.expected.encoded_bytes, "{}: encoded bytes", v.id);
        let hex: String = got.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, v.expected.encoded_hex, "{}: encoded hex", v.id);
    }
}

#[test]
fn sap_codec_vectors_round_trip() {
    for v in vectors() {
        let buf: [u8; 28] = encode(&v).try_into().expect("SAP is 28 bytes");
        let decoded = SapHeader::decode(&buf);
        assert_eq!(decoded.encode(), buf, "{}: decode/encode not identity", v.id);
        assert_eq!(
            decoded.seq_counter, v.input.seq_counter,
            "{}: seq_counter differs after a round trip",
            v.id
        );
    }
}

#[test]
fn sap_codec_matches_the_reference_golden() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("tests/test_vectors/golden/sap_codec.json");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read golden {}: {e}", path.display()));
    let json: serde_json::Value = serde_json::from_str(&raw).expect("golden is not JSON");
    assert_eq!(
        json.get("category").and_then(|c| c.as_str()),
        Some("sap_codec"),
        "golden file is for a different category"
    );
    let golden: BTreeMap<String, String> = json
        .get("encoded_hex")
        .and_then(|m| m.as_object())
        .expect("golden has no `encoded_hex` object")
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
        .collect();

    let declared: Vec<Vector> = vectors();
    for v in &declared {
        let hex: String = encode(v).iter().map(|b| format!("{b:02x}")).collect();
        let want = golden
            .get(&v.id)
            .unwrap_or_else(|| panic!("{} has no golden entry", v.id));
        assert_eq!(&hex, want, "{}: differs from the reference golden", v.id);
    }
    let names: std::collections::BTreeSet<&str> =
        declared.iter().map(|v| v.id.as_str()).collect();
    let recorded: std::collections::BTreeSet<&str> = golden.keys().map(|k| k.as_str()).collect();
    assert_eq!(names, recorded, "golden and manifest disagree on which vectors exist");
}
