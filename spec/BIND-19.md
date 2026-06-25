# BIND-19: Transport Binding and Layered Semantic Framing Specification (v1.0.0-RFC-4)

## 1. Introduction and Objectives

This specification defines **BIND-19**, an independent, transport-level semantic framing and capability negotiation protocol within the **CommonIntents-144 (CI-144)** suite. 

To prevent architectural tight coupling (the "silo" anti-pattern), BIND-19 is strictly designed to be **completely orthogonal to the INTENT-7 semantic layer**. BIND-19 remains entirely oblivious to the specific business contents of its payload. It is solely responsible for:
- Standardized binary frame boundaries (preventing TCP half-packet/sticky-packet fragmentation issues).
- 256-channel logical multi-task multiplexing over a single physical connection.
- TLS 1.3-style zero-RTT and negotiation-based transport capability handshakes.
- Hard security boundaries, frame-level integrity verification, and anti-replay defense.

---

## 2. Fixed Header and Frame Layout

Every BIND-19 packet MUST begin with an 8-byte fixed-size header, followed by a variable-length payload. All multi-byte integers MUST be encoded in Big-Endian (Network Byte Order).

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|  Frame Type   |  Channel ID   |     Flags     |  Sequence ID  |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                        Payload Length                         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                     Payload Data (Variable)                   |
|                             ...                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### 2.1 Header Field Definitions

- **Frame Type (1 Byte)**: Defines the mathematical and routing semantics of the frame (see Section 3).
- **Channel ID (1 Byte)**: Logical channel identifier supporting up to 256 parallel concurrent streams (multiplexing) over a single physical transport without Head-of-Line (HOL) blocking.
- **Flags (1 Byte)**: Bitmask defining operational characteristics of the frame:
  - `0x01` (**FIN**): Set if this is the final fragment of a split/partitioned payload.
  - `0x02` (**CON**): Set if asynchronous Human-In-The-Loop (HITL) consensus via CAPABILITY-13 is required before execution.
  - `0x04` (**SEC**): Set if the payload is encrypted at the frame layer (complementary to TLS).
  - *Unassigned bits MUST be set to 0 by the sender and ignored by the receiver to ensure forward compatibility.*
- **Sequence ID (1 Byte)**: Monotonically increasing sequence number within each `Channel ID` for packet reordering, deduplication, and sliding-window flow control.
- **Payload Length (4 Bytes)**: Unsigned 32-bit integer indicating the size of the payload in bytes. Hard-capped at `0x04000000` (64MB) to mitigate out-of-memory (OOM) denial-of-service attacks.

---

## 3. Frame Type Allocation and Governance (IANA-Style)

To prevent collision between official standard updates and local experimental extensions, BIND-19 enforces a strict partition of the 1-byte `FrameType` address space:

```
+------------------+-------------------------------------------------+
| FrameType Range  | Allocation & Governance Policy (IANA-Style)     |
+------------------+-------------------------------------------------+
| 0x00             | Reserved / Invalid (Hard Exception)             |
| 0x01 - 0x0E      | Standard Core (Immutable, locked in v1.0.0)     |
| 0x0F - 0xEF      | Standard Extensions (Allocated ONLY via RFC PR) |
| 0xF0 - 0xFF      | Private / Experimental Use (Zero Governance)   |
+------------------+-------------------------------------------------+
```

### 3.1 Standard Core Frame Types

```rust
#[repr(u8)]
pub enum FrameType {
    /// 0x01: Data Frame. Carries standard INTENT-7 JSON payload.
    Data = 0x01,
    
    /// 0x02: Heartbeat Frame. Used for keep-alive. Payload MUST be 0.
    Heartbeat = 0x02,
    
    /// 0x03: Control Frame. Carries system-level flow signals (CANCEL, SUSPEND, RESUME).
    Control = 0x03,
    
    /// 0x04: Vector Frame. Carries high-frequency incremental mental energy arrays (e.g., SA-Core activation vectors).
    Vector = 0x04,
    
    /// 0x05: Handshake Frame. Carries transport negotiation options during session initialization.
    Handshake = 0x05,
    
    /// 0x06: Error Frame. Carries transport-level framing or negotiation failures.
    Error = 0x06,
}
```

### 3.2 Standard Extension (0x0F - 0xEF) RFC Governance Process
Any future addition to the standard extension range MUST follow the PR review lifecycle:
1. **RFC Submission**: The applicant submits an RFC draft under `BIND-19/spec/drafts/` detailing the byte layout and performance impact.
2. **Deterministic Check**: The CI pipeline automatically verifies that the frame format does not break the 8-byte fixed header boundaries or exceed the 64MB buffer limit.
3. **Consensus Merge**: The PR must accumulate Ed25519 digital signatures from at least 3 active, independent seed instances to be merged into `main`.
4. **Registry Generation**: Upon merging, the CI pipeline automatically compiles the metadata and updates the official `registry.json` list.

### 3.3 Private Range (0xF0 - 0xFF) Exception Policy
The private range is completely exempt from standard registration. Local implementations (such as experimental game-reflex streams) can use these bytes freely. Standard-compliant parsers encountering unknown private frames MUST skip them or delegate them to custom local plugins, rather than triggering a connection-level panic.

---

## 4. Connection Handshake and TLS 1.3-Style Version Negotiation

To prevent version desynchronization and lock-ins on public networks, BIND-19 establishes a dual-negotiation handshake mimicking TLS 1.3.

### 4.1 Handshake Initialization (Frame Type 0x05)
Upon connection, the client sends a `Handshake` frame containing its supported versions and capability bitmasks in a lightweight, flat JSON format:

```json
{
  "client_supported_bind_versions": ["1.0", "1.1"],
  "client_supported_intent_versions": ["1.0"],
  "client_supported_capabilities": ["tlv_frame", "channel_multiplex", "flow_control"]
}
```

#### Fields Description
- `client_supported_bind_versions`: Array of strings. Lists the BIND-19 semantic frame protocol versions supported by the sender.
- `client_supported_intent_versions`: Array of strings. Lists the INTENT-7 payload schema versions supported by the sender.
- `client_supported_capabilities`: Array of strings. Declares optional BIND-19 features requested.

### 4.2 Intersection Selection Algorithm
1. The receiving server reads the client's version lists and compares them against its own local support list.
2. The server selects the **highest mutually supported version** (e.g., if local supports `["1.0", "1.1"]` and client supports `["1.1", "1.2"]`, the negotiated version is `1.1`).
3. The negotiated version and selected features are returned in the Handshake Response.
4. **Mute Mismatch Drop**: If the intersection of supported versions is empty, the server MUST return an `Error (0x06)` frame with the code `0x01CC` (`460 Version Mismatch`) and immediately drop the physical TCP/Unix socket connection. No further backwards-compatible degradation is permitted.

---

## 5. Local Transport Binding (UDS & 0-RTT Implicit Negotiation)

When the underlying physical layer is a **Unix Domain Socket (UDS, `unix://`)**, BIND-19 enforces a high-performance, zero-overhead **0-RTT Implicit Handshake**:

- **No physical handshakes are transmitted**. Both client and server implicitly assume:
  - `tlv_frame = true`
  - `crc_check = false` (Zero risk of hardware-level packet corruption over local IPC loopback).
  - `flow_control = false` (Relying purely on the OS kernel's socket buffer backpressure).
- This eliminates the negotiation round-trip entirely, reducing local latency to **sub-millisecond (<0.5ms)** levels and saving massive amounts of ARM CPU cycles.

---

## 6. Error Codes

When a framing or negotiation failure occurs, a `FrameType::Error (0x06)` is dispatched containing a 2-byte error code in its payload:

| Error Code | Hexadecimal | Name | Description |
|:---|:---|:---|:---|
| `1024` | `0x0400` | `INVALID_FRAME_HEADER` | Fixed 8-byte header is corrupted or unparseable. |
| `1120` | `0x0460` | `VERSION_MISMATCH` | Client and server share no overlapping SemVer matrix. |
| `1200` | `0x04B0` | `BUFFER_OVERFLOW` | Frame payload size exceeds the 64MB hard limit. |
| `1300` | `0x0514` | `UNALLOCATED_STANDARD_FRAME` | Standard extension frame received but not registered in BIND-19. |
| `1400` | `0x0578` | `INTEGRITY_CHECKSUM_FAILED` | CRC or payload verification hash mismatch. |
| `1500` | `0x05DC` | `CONSENSUS_UNAVAILABLE` | External CAPABILITY-13 confirmation engine is unreachable. |

---

## 7. Zero-Copy Rust Parsing Implementation Guidelines

Conforming implementations of BIND-19 in Rust MUST utilize the `bytes` crate to ensure zero-allocation slicing of incoming frame streams:

```rust
// Implementation template using the `bytes` crate
use bytes::{Bytes, BytesMut, Buf};

pub struct FrameParser {
    buffer: BytesMut,
}

impl FrameParser {
    pub fn parse_frame(&mut self) -> Option<Result<(Header, Bytes), ErrorCode>> {
        if self.buffer.len() < 8 {
            return None; // Wait for more data (no allocations)
        }
        
        // Peek at payload length without copying
        let payload_len = u32::from_be_bytes([
            self.buffer[4], self.buffer[5], self.buffer[6], self.buffer[7]
        ]) as usize;
        
        if payload_len > 0x04000000 {
            return Some(Err(ErrorCode::BufferOverflow));
        }
        
        if self.buffer.len() < 8 + payload_len {
            return None; // Wait for full payload to arrive
        }
        
        // Advance buffer pointer and slice zero-copy payload
        let header_bytes = self.buffer.split_to(8);
        let payload_bytes = self.buffer.split_to(payload_len).freeze();
        
        let header = Header::from_bytes(header_bytes);
        Some(Ok((header, payload_bytes)))
    }
}
```

This guarantees optimal performance on resource-constrained ARM matrices while preventing memory fragmentation.
