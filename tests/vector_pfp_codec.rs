//! CI-4 batch 1 — the `pfp_codec` vectors, actually loaded.
//!
//! vector-category: pfp_codec
//!
//! The vector file has declared these five since 2026-08-29 and nothing read
//! them (K-013). This is the first batch of waivers being paid off rather than
//! extended: `pfp_codec` was scheduled first precisely because `src/pfp.rs`
//! already has the codec and its unit tests, so these five could be proven
//! instead of promised.
//!
//! The vectors are read from the manifest rather than restated here. A test that
//! hard-codes the expected bytes is testing its own copy: it would keep passing
//! after the manifest changed, which is the failure this whole check exists to
//! name.

use std::collections::BTreeMap;
use std::path::Path;

use bind19::pfp::{
    BodyStance, Modality, OutputDest, OverrideFlag, PfpHeader, ProximityEdge, RiskLevel,
};

/// One vector as the manifest spells it. `expected` keeps the fields this batch
/// asserts on; anything else in the object is ignored rather than guessed at.
#[derive(serde::Deserialize)]
struct Vector {
    id: String,
    input: Input,
    expected: Expected,
}

#[derive(serde::Deserialize)]
struct Input {
    modality: String,
    risk_level: String,
    body_stance: String,
    proximity_edge: String,
    output_dest: String,
    override_flag: String,
    replay_enable: bool,
}

#[derive(serde::Deserialize)]
struct Expected {
    encoded_hex: String,
    encoded_bytes: Vec<u8>,
    size: usize,
    family_magic: String,
}

fn pfp_codec_vectors() -> Vec<Vector> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("tests/test_vectors/ci-144-v2.0-test-vectors.json");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let json: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("invalid JSON: {e}"));
    let list = json
        .get("vectors")
        .and_then(|v| v.get("pfp_codec"))
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| panic!("manifest has no `pfp_codec` array"));
    let out: Vec<Vector> = list
        .iter()
        .map(|v| serde_json::from_value(v.clone()).unwrap_or_else(|e| panic!("bad vector: {e}")))
        .collect();
    assert_eq!(
        out.len(),
        5,
        "expected the five declared pfp_codec vectors, got {}",
        out.len()
    );
    out
}

fn modality(s: &str) -> Modality {
    match s {
        "Cognitive" => Modality::Cognitive,
        "Render" => Modality::Render,
        "Executive" => Modality::Executive,
        "SensorFeed" => Modality::SensorFeed,
        other => panic!("vector uses an unknown modality {other:?}"),
    }
}

fn risk(s: &str) -> RiskLevel {
    match s {
        "Low" => RiskLevel::Low,
        "Medium" => RiskLevel::Medium,
        "Critical" => RiskLevel::Critical,
        "Catastrophic" => RiskLevel::Catastrophic,
        other => panic!("vector uses an unknown risk level {other:?}"),
    }
}

fn stance(s: &str) -> BodyStance {
    match s {
        "Seated" => BodyStance::Seated,
        "Standing" => BodyStance::Standing,
        "Moving" => BodyStance::Moving,
        "Unknown" => BodyStance::Unknown,
        other => panic!("vector uses an unknown body stance {other:?}"),
    }
}

fn edge(s: &str) -> ProximityEdge {
    match s {
        "Safe" => ProximityEdge::Safe,
        "Warning" => ProximityEdge::Warning,
        "Danger" => ProximityEdge::Danger,
        "CriticalEdge" => ProximityEdge::CriticalEdge,
        other => panic!("vector uses an unknown proximity edge {other:?}"),
    }
}

fn dest(s: &str) -> OutputDest {
    match s {
        "Internal" => OutputDest::Internal,
        "External" => OutputDest::External,
        other => panic!("vector uses an unknown output destination {other:?}"),
    }
}

fn override_flag(s: &str) -> OverrideFlag {
    match s {
        "Normal" => OverrideFlag::Normal,
        "HardOverride" => OverrideFlag::HardOverride,
        other => panic!("vector uses an unknown override flag {other:?}"),
    }
}

fn encode_vector(v: &Vector) -> [u8; 4] {
    PfpHeader::new(
        modality(&v.input.modality),
        risk(&v.input.risk_level),
        stance(&v.input.body_stance),
        edge(&v.input.proximity_edge),
        dest(&v.input.output_dest),
        override_flag(&v.input.override_flag),
        v.input.replay_enable,
    )
    .encode()
}

/// The encoder matches the manifest, in all three spellings the manifest gives:
/// length, decimal bytes, and hex.
#[test]
fn pfp_codec_vectors_encode_as_declared() {
    for v in pfp_codec_vectors() {
        let got = encode_vector(&v);
        assert_eq!(
            got.len(),
            v.expected.size,
            "{}: declared size {}",
            v.id,
            v.expected.size
        );
        assert_eq!(
            got.to_vec(),
            v.expected.encoded_bytes,
            "{}: encoded bytes differ from the manifest",
            v.id
        );
        let hex: String = got.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, v.expected.encoded_hex, "{}: encoded hex differs", v.id);
        // Compare the digits, not the spelling: the manifest writes `0xcf14`
        // and the test must not fail over the case of `x`.
        let magic = format!("{:02x}{:02x}", got[0], got[1]);
        let declared = v
            .expected
            .family_magic
            .trim_start_matches("0x")
            .trim_start_matches("0X");
        assert_eq!(
            magic,
            declared.to_lowercase(),
            "{}: family magic differs",
            v.id
        );
    }
}

/// Decoding is the inverse, and it is checked against the ENCODED bytes rather
/// than against a second hand-written table. Re-encoding the decode is what
/// catches a decoder that reads the right fields out of the wrong offsets.
#[test]
fn pfp_codec_vectors_round_trip_through_decode() {
    for v in pfp_codec_vectors() {
        let encoded = encode_vector(&v);
        let decoded = PfpHeader::decode(&encoded);
        assert_eq!(
            decoded.encode(),
            encoded,
            "{}: decode then encode is not the identity",
            v.id
        );
        // Field-level agreement with the vector's own input, so a decoder that
        // round-trips by preserving raw bytes cannot pass by accident.
        assert_eq!(decoded.modality, modality(&v.input.modality), "{}", v.id);
        assert_eq!(decoded.risk_level, risk(&v.input.risk_level), "{}", v.id);
        assert_eq!(decoded.body_stance, stance(&v.input.body_stance), "{}", v.id);
        assert_eq!(
            decoded.proximity_edge,
            edge(&v.input.proximity_edge),
            "{}",
            v.id
        );
        assert_eq!(decoded.output_dest, dest(&v.input.output_dest), "{}", v.id);
        assert_eq!(
            decoded.override_flag,
            override_flag(&v.input.override_flag),
            "{}",
            v.id
        );
        assert_eq!(decoded.replay_enable, v.input.replay_enable, "{}", v.id);
    }
}

/// Every vector id in this category is distinguishable from the others, so a
/// load that silently returned one row five times would not pass.
#[test]
fn the_category_has_five_distinct_vectors() {
    let vectors = pfp_codec_vectors();
    let by_id: BTreeMap<&str, usize> = vectors
        .iter()
        .fold(BTreeMap::new(), |mut acc, v| {
            *acc.entry(v.id.as_str()).or_insert(0) += 1;
            acc
        });
    assert_eq!(by_id.len(), vectors.len(), "duplicate vector ids: {by_id:?}");
    let encodings: BTreeMap<Vec<u8>, &str> = vectors
        .iter()
        .map(|v| (encode_vector(v).to_vec(), v.id.as_str()))
        .collect();
    assert_eq!(
        encodings.len(),
        vectors.len(),
        "two vectors encode to the same bytes, so one of them proves nothing: {encodings:?}"
    );
}
