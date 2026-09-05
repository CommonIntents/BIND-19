# BIND-19 — INTENT-7/Transport Binding Protocol [![Org](https://img.shields.io/badge/Org-CommonIntents--144-darkgray.svg)](https://github.com/CommonIntents)

[![Version](https://img.shields.io/badge/v2.0--rc.1-Released-brightgreen.svg)](https://github.com/CommonIntents/BIND-19/releases/tag/v2.0-rc.1)
[![Tests](https://img.shields.io/badge/tests-140%20passed-brightgreen.svg)]()
[![Vectors](https://img.shields.io/badge/test%20vectors-33%20sets-blue.svg)]()
[![Examples](https://img.shields.io/badge/examples-4-orange.svg)]()
[![Benchmarks](https://img.shields.io/badge/benchmarks-14-purple.svg)]()

> **仓库双身份**：本分支（v2.0-alpha）= **Rust 参考实现**（142 tests，含 PFP/SAP 解析器）；
> 协议规范正文在 `main` 分支（v1.0.0-RFC-4，spec-only，tag `v1.0.0-RFC-4`）。规范与实现分离，各自独立演进。

**Flexible, thin, replaceable.** BIND-19 is the adaptation layer between INTENT-7 (intent syntax) and concrete transport implementations. It defines *how* INTENT-7 intents are carried — format negotiation, integrity checks, and version compatibility declaration.

## 🏆 v2.0-rc.1 Battle Results

| Metric | Result | Note |
|---|---|---|
| **Test Vectors** | **33 sets** (8 categories) | Exceeds ≥20 requirement, fixed-seed reproducible |
| **Usage Examples** | **4 examples** | All compile & run verified |
| **Multi-Tenant Tests** | **9 scenarios** | Cross-tenant isolation + concurrency no deadlock |
| **Total Tests** | **140 passed** | lib 122 + integration 9 + multi-tenant 9 |
| **Rust Modules** | **9 modules** | pfp/sap/frame/crypto/config/rotation/catastrophic/replay_cache/processor |
| **Zero Warnings** | ✅ | `cargo clippy --all-targets -- -D warnings` |

### ⚡ Key Benchmarks (2013 MacBook Pro 2.3GHz i7)

| Operation | Latency | Throughput |
|---|---|---|
| Replay cache check (hit) | **39 ns** | ~25.6M ops/s |
| Frame encode+decode roundtrip | **253 ns** | ~3.95M fps |
| PAH 64-bit truncated verify | **684 ns** | ~1.46M ops/s |
| CATASTROPHIC bit-check | **0.3 ps** | ~3 CPU cycles |
| **Tuck hard-real-time path** | **< 100 ns** | **sub-microsecond decision** |

### 📦 Quick Start

```bash
# Run all tests
cargo test --all-targets

# Run examples
cargo run --example basic_frame
cargo run --example replay_protection
cargo run --example tuck_integration

# Generate test vectors
cargo run --example generate_test_vectors

# Run benchmarks
cargo bench --bench replay_bench
```

---

## What BIND-19 Negotiates
- **Transport format** — binary (MessagePack/CBOR) by default, JSON fallback
- **Integrity check** — optional CRC32 at frame level, disabled under mTLS
- **Version binding** — declares BIND-19 version and current binding target

## Protocol Stack
```
INTENT-7 (intent syntax — SIDL)
  ↑
BIND-19 ← You are here (transport binding)
  ↑
INTENT-7-SECURE (optional mTLS reference implementation)
  ↑
CAPABILITY-13 (consensus confirmation)
```

## v2.0 — CI-144 Protocol Family Architecture (Active Development)

BIND-19 v2.0 introduces the **CI-144 Protocol Family** architecture, decoupling physical features from security attestation:

```
[ 8-byte BIND-19 Header ] + [ PFP 4 bytes (optional) ] + [ SAP 28 bytes (optional) ] + [ Payload ]
```

### PFP-xCF14 — Physical Feature Protocol (Frozen Layer, 4 bytes)

Tuck hard-real-time decision layer. Read-only, 4 bytes, 83.3% more energy-efficient than the original 24-byte PAL design.

| Byte | Bits | Field |
|---|---|---|
| 0-1 | 16 | Family-Magic = `0xCF14` |
| 2 | 2 | Modality (COGNITIVE/RENDER/EXECUTIVE/SENSOR_FEED) |
| 2 | 2 | Risk-Level (LOW/MEDIUM/CRITICAL/CATASTROPHIC) |
| 2 | 2 | Body-Stance (SEATED/STANDING/MOVING/UNKNOWN) |
| 2 | 2 | Proximity-Edge (SAFE/WARNING/DANGER/CRITICAL_EDGE) |
| 3 | 1 | Output-Dest (INTERNAL/EXTERNAL) |
| 3 | 1 | Override-Flag (NORMAL/HARD_OVERRIDE) |
| 3 | 1 | Replay-Enable |
| 3 | 5 | Reserved (zero) |

### SAP-xCF14 — Security Attestation Protocol (Evolution Layer, 28 bytes)

Optional security layer. Loaded on-demand; low-security scenarios can skip SAP entirely (minimum frame overhead = 11 bytes).

| Byte | Field |
|---|---|
| 0-1 | Family-Magic = `0xCF14` |
| 2 | Protocol-ID = `0x01` |
| 3 | SAP-Version (4 bits) + Reserved (4 bits) |
| 4-5 | Seq-Counter (16-bit, rotation threshold = 65534) |
| 6-19 | PAH-Hash (112 bits = SHA-256 truncated) |
| 20-27 | PAH-Signature (64 bits = first-layer fast verification) |

### Key Rotation (Rule 7)

- Trigger: Seq-Counter ≥ 65534
- Frame type: `0x07` (KEY_ROTATION, confirmed unallocated by ADR-0008)
- ACK frame type: `0x08` (KEY_ROTATION_ACK)
- ACK timeout: 100ms, max 3 retries
- Fail-closed: after 3 failed retries, stop sending data frames, wait for manual reset

### Debug Mode

Environment variable `CI144_DEBUG=1`:
- Rule 6 (Replay-Enable=0 forced downgrade to MEDIUM) can be skipped
- Rules 1-3 (CATASTROPHIC hard override) always enforced, cannot be skipped
- Read only at startup, cannot be toggled at runtime
- Warning banner printed to stderr on startup

## Rust Implementation (v2.0-alpha)

| Module | Description |
|---|---|
| `pfp` | PFP-xCF14 4-byte header encode/decode |
| `sap` | SAP-xCF14 28-byte header encode/decode |
| `frame` | BIND-19 8-byte header + full frame with optional PFP/SAP |
| `crypto` | Ed25519 signing + SHA-256 64-bit truncation (first-layer verification) |
| `config` | Runtime config (CI144_DEBUG env var) |
| `rotation` | Key rotation state machine (KEY_ROTATION + ACK timeout fail-closed) |

## Performance Benchmarks (v2.0-beta)

Measured on 2013 MacBook Pro (2.3GHz i7, 16GB). Criterion 0.5, 100 samples each.

### Replay Cache (DashMap + AtomicU16)

| Operation | Latency | Throughput |
|---|---|---|
| `check_and_update` (cache hit) | **39 ns** | ~25.6M ops/s |
| `check_and_update` (replay reject) | **38 ns** | ~26.3M ops/s |
| `check_and_update` (miss/new source) | **195 ns** | ~5.1M ops/s |

### Frame Encode/Decode (PFP 4B + SAP 28B + 64B payload)

| Operation | Latency |
|---|---|
| `frame.encode()` | **88 ns** |
| `BindFrame::decode()` | **105 ns** |
| encode + decode roundtrip | **253 ns** |

### PAH Signature (Ed25519 + SHA-256 truncation)

| Operation | Latency |
|---|---|
| Ed25519 full sign (64B) | **25.2 µs** |
| Ed25519 full verify (64B) | **46.2 µs** |
| PAH 64-bit truncated sign | **26.3 µs** |
| PAH 64-bit truncated verify (match) | **684 ns** |

### CATASTROPHIC Detection

| Operation | Latency |
|---|---|
| Pure bit-check (`is_catastrophic_override`) | **0.3 ps** (compiler-optimized, ~3 CPU cycles) |
| Normal frame check (no trigger) | **3.2 ns** |
| Full handle (trigger event + audit log) | **60.8 µs** |

### Key Insights

- **Tuck hard-real-time path** = PFP read (4B) + CATASTROPHIC bit-check (~3 cycles) + replay cache check (~40ns) = **sub-microsecond** decision
- **PAH first-layer verification** = 684ns (truncated match), full Ed25519 verify deferred to async layer
- **Frame processing** = ~250ns roundtrip, suitable for 10Gbps+ line-rate processing
- **Energy efficiency** = fixed-offset parsing, no allocation on hot path, no branching in decision logic

Run benchmarks locally:
```bash
cargo bench --bench replay_bench
```

## Read the Spec
- [BIND-19 v1.0.0-RFC-4](spec/BIND-19.md)
- [中文版](spec/BIND-19.zh-CN.md)
- [CI-144 v2.0 Upgrade Plan](docs/v2.0-upgrade-plan.md)
- [Development Plan (PLAN)](PLAN.md)
- [Growth Log (GROWTH)](GROWTH.md)
- [Architecture Decision Records](docs/decisions/)

## Test Vectors & Examples

### Test Vectors (33 sets, 8 categories)

Machine-readable JSON test vectors for other compatible implementations:

- **[`tests/test_vectors/ci-144-v2.0-test-vectors.json`](tests/test_vectors/ci-144-v2.0-test-vectors.json)** — 33 sets, 21KB
- **[`tests/test_vectors/README.md`](tests/test_vectors/README.md)** — Human-readable documentation with usage examples

| Category | Count | Content |
|---|---|---|
| `pfp_codec` | 5 | PFP 4-byte encode/decode (Modality/Risk/Stance/Edge/flags) |
| `sap_codec` | 5 | SAP 28-byte encode/decode (Seq-Counter boundary values) |
| `frame_codec` | 5 | Full frame encode/decode (v1.0 compat / PFP-only / PFP+SAP / payload) |
| `replay_protection` | 5 | Replay protection (normal increment / exact replay / old seq / new source) |
| `rule6_downgrade` | 3 | Rule 6 downgrade (Replay-Enable=0 → MEDIUM) |
| `key_rotation` | 4 | Key rotation state machine (threshold / start / ACK / complete) |
| `catastrophic_detection` | 3 | CATASTROPHIC detection (Risk+Override combinations) |
| `pah_signature` | 3 | PAH signature (full Ed25519 / 64-bit truncation / wrong signature) |

Generate test vectors locally:
```bash
cargo run --example generate_test_vectors
```

### Examples (4 key usage examples)

| Example | Description | Run |
|---|---|---|
| [`basic_frame.rs`](examples/basic_frame.rs) | Basic frame create/encode/decode | `cargo run --example basic_frame` |
| [`replay_protection.rs`](examples/replay_protection.rs) | ReplayCache + FrameProcessor usage | `cargo run --example replay_protection` |
| [`tuck_integration.rs`](examples/tuck_integration.rs) | **Tuck hard-real-time decision path** (most important ecosystem example) | `cargo run --example tuck_integration` |
| [`generate_test_vectors.rs`](examples/generate_test_vectors.rs) | Test vector generator | `cargo run --example generate_test_vectors` |

### Multi-Tenant Tests (9 scenarios)

Comprehensive multi-tenant isolation verification in [`tests/multi_tenant_test.rs`](tests/multi_tenant_test.rs):

- Cross-tenant cache isolation
- Cross-tenant counter isolation
- Cross-tenant key rotation state machine isolation
- Multi-tenant concurrent access (10 tenants × 100 seq)
- Tenant ID boundary cases (0 / u64::MAX)
- Same tenant different source isolation
- FrameProcessor multi-tenant processing
- Multi-tenant cache capacity (1000 combinations)
- Multi-tenant rule6 downgrade isolation

Run all tests:
```bash
cargo test --all-targets
```

## Specification Repos (Publication Windows)

BIND-19 is the **Single Source of Truth (SSOT)** for the CI-144 Protocol Family. The following repos are **publication windows** (spec mirrors), not independent authorities:

| Protocol | Repo | Spec Authority (in BIND-19) |
|---|---|---|
| **PFP-xCF14** (Physical Feature, 4 bytes, Frozen) | [CommonIntents/PFP-xCF14](https://github.com/CommonIntents/PFP-xCF14) | [`docs/spec/pfp-xcf14.md`](docs/spec/pfp-xcf14.md) |
| **SAP-xCF14** (Security Attestation, 28 bytes, Evolving) | [CommonIntents/SAP-xCF14](https://github.com/CommonIntents/SAP-xCF14) | [`docs/spec/sap-xcf14.md`](docs/spec/sap-xcf14.md) |

> **All spec change PRs must be filed in BIND-19**, updating `docs/spec/` + `src/` + `tests/` together (OpenSSL model). The publication window repos do not accept spec change PRs.

## Related
| Protocol | Repository |
|:---|:---|
| INTENT-7 | [CommonIntents/INTENT-7](https://github.com/CommonIntents/INTENT-7) |
| CAPABILITY-13 | [CommonIntents/CAPABILITY-13](https://github.com/CommonIntents/CAPABILITY-13) |
| INTENT-7-SECURE | [CommonIntents/INTENT-7-SECURE](https://github.com/CommonIntents/INTENT-7-SECURE) |

## Appendix: Design Inspiration & Conceptual Isomorphism (Inspired by)

CI-144 Protocol Family is an **independent implementation**, but its design philosophy is **inspired by** mature infrastructure projects that have been validated over decades. We stand on the shoulders of giants.

> **"Inspired by" means: we learned the design ideas, implemented independently. Ideas are not copyrightable; this is open-source etiquette, not a legal obligation.**

### Conceptual Isomorphism Table

| Mature Infrastructure | Years Validated | CI-144 Design Point Inspired | Isomorphism |
|---|---|---|---|
| **EtherType** (IEEE 802.3) | 40+ | PFP `Family-Magic` (0xCF14) | 2-byte fixed-offset protocol identifier, hardware-friendly |
| **IP Protocol Number** (RFC 790) | 40+ | Sub-protocol ID (1 byte, fixed allocation table) | 1-byte upper-layer protocol identifier, fixed assignment |
| **PROFIsafe** (IEC 61784-3) | 20+ | Rule 6: forced downgrade to safe state (Replay-Enable=0 → MEDIUM) | Fail-safe communication: when security mechanism is disabled, force downgrade to safe state |
| **CAN Bus** (ISO 11898) | 30+ | `Risk-Level` priority field (fixed-offset, high-priority frames processed first) | Fixed-offset priority field, hardware-level scheduling |
| **TLS 1.3** (RFC 8446) | 5+ | Fixed encryption path (AES-GCM, no "encrypt or not" branch) | Fixed cipher suite, no branching, hardware acceleration pipeline-friendly |
| **OpenSSL Documentation Policy** | 20+ | "One authority, multiple presentations" (spec + code + tests in same PR) | Code and documentation reviewed and merged together, never diverge |

### Why This Matters

| Benefit | Explanation |
|---|---|
| **Trust Transfer** | Users trust infrastructure validated over decades; CI-144's conceptual isomorphism transfers that trust |
| **Lower Criticism Threshold** | When design is questioned, we can say "this is conceptually isomorphic to EtherType" instead of "we thought of it ourselves" |
| **Open-Source Etiquette** | Acknowledge the giants whose shoulders we stand on |
| **Reduced Explanation** | Readers who see this appendix understand the design philosophy without further explanation |

### Key Principle

> **Acknowledgment is credit endorsement, not a disclaimer.**
>
> When we say "inspired by mature infrastructure," we are not making excuses ("so don't blame me if it's wrong"). We are building credibility ("these paths have been validated, so our choices have basis").

---

## License
Apache 2.0 — see [LICENSE](LICENSE).
