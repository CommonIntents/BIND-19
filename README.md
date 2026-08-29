# BIND-19 — INTENT-7/Transport Binding Protocol [![Org](https://img.shields.io/badge/Org-CommonIntents--144-darkgray.svg)](https://github.com/CommonIntents)

**Flexible, thin, replaceable.** BIND-19 is the adaptation layer between INTENT-7 (intent syntax) and concrete transport implementations. It defines *how* INTENT-7 intents are carried — format negotiation, integrity checks, and version compatibility declaration.

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

## Related
| Protocol | Repository |
|:---|:---|
| INTENT-7 | [CommonIntents/INTENT-7](https://github.com/CommonIntents/INTENT-7) |
| CAPABILITY-13 | [CommonIntents/CAPABILITY-13](https://github.com/CommonIntents/CAPABILITY-13) |
| INTENT-7-SECURE | [CommonIntents/INTENT-7-SECURE](https://github.com/CommonIntents/INTENT-7-SECURE) |

## License
Apache 2.0 — see [LICENSE](LICENSE).
