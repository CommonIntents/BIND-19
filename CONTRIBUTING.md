# Contributing to BIND-19

BIND-19 (INTENT-7/Transport Binding Protocol) is the ligament of the protocol family.

## Specification Files

- English: [spec/BIND-19.md](spec/BIND-19.md)
- Chinese: [spec/BIND-19.zh-CN.md](spec/BIND-19.zh-CN.md)

## How to Propose Changes

1. Read the [organization-level contribution guide](https://github.com/CommonIntents/.github/blob/main/CONTRIBUTING.md)
2. Open an Issue describing the problem or improvement you've identified
3. If proposing a new format or integrity check, describe the negotiation mechanism
4. Update both the English and Chinese versions in your PR

## Design Constraint

BIND-19 is the thinnest layer in the protocol stack. It defines no new interaction semantics.
Proposals that add semantic content to BIND-19 will be redirected to INTENT-7 or CAPABILITY-13.

---

## v2.0 Protocol Family — Specification Change Process (MUST READ)

BIND-19 v2.0 introduces the **CI-144 Protocol Family** architecture (PFP-xCF14 + SAP-xCF14).
The specification documents are the **Single Source of Truth (SSOT)** for the entire ecosystem.

### Authority Declaration

| Role | Location |
|---|---|
| **Specification Authority (SSOT)** | `docs/spec/pfp-xcf14.md` + `docs/spec/sap-xcf14.md` (this repo) |
| **Reference Implementation** | `src/pfp.rs` + `src/sap.rs` + `src/frame.rs` (this repo) |
| **Publication Window** | [CommonIntents/PFP-xCF14](https://github.com/CommonIntents/PFP-xCF14) + [CommonIntents/SAP-xCF14](https://github.com/CommonIntents/SAP-xCF14) |

**The publication window repos do NOT accept specification change PRs.** All spec changes must be proposed in BIND-19.

### Mandatory PR Requirements for Spec Changes

Any PR that changes the PFP-xCF14 or SAP-xCF14 specification **MUST** update all three of the following in the same PR:

1. **Specification document** (`docs/spec/pfp-xcf14.md` and/or `docs/spec/sap-xcf14.md`)
2. **Implementation code** (`src/pfp.rs` and/or `src/sap.rs` and/or `src/frame.rs`)
3. **Tests** (`tests/` directory, including unit tests and test vectors)

This follows the **OpenSSL model**: code and documentation must be reviewed and merged together, ensuring they never diverge.

### PFP-xCF14 Freeze Constraint

PFP-xCF14 is the **frozen layer**. Once v1.0 is frozen, **no changes are allowed** to the 4-byte structure, field offsets, or enum values. Any proposed change must:

1. Produce a **new version** (e.g., PFP-xCF15) with a new Family-Magic or Protocol-ID
2. Coexist with v1.0 (backward compatibility required)
3. Go through the full ADR process (see `docs/decisions/`)

### SAP-xCF14 Evolution Constraint

SAP-xCF14 is the **evolution layer**. New versions (v2, v3, ...) may be introduced, but:

1. Each version has a distinct `Version` field (4 bits, supports up to 16 versions)
2. Multiple versions may coexist in the same ecosystem
3. Version negotiation must be documented in the spec

### Test Vector Requirement

Any spec change that affects encoding/decoding **MUST** include updated test vectors. Run the test vector generator to regenerate:

```bash
cargo run --example generate_test_vectors
```

Test vectors are published at `tests/test_vectors/ci-144-v2.0-test-vectors.json` and are used by other compatible implementations to verify correctness.

### ADR Requirement

Significant spec changes (new fields, new protocol versions, security model changes) **MUST** include an Architecture Decision Record (ADR) in `docs/decisions/`. See existing ADRs for format reference.

### Version Tagging

When a spec version is finalized, tag the BIND-19 repo with the corresponding version (e.g., `v2.0-rc.1`, `v2.0.0`). The publication window repos reference this tag in their authority declaration.
