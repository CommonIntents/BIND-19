# BIND-19: INTENT-7/Transport Binding Protocol

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0) [![Version](https://img.shields.io/badge/Version-0.1.0--draft-orange.svg)]() [![Status](https://img.shields.io/badge/Status-RFC%20Draft-yellow.svg)]() [![Org](https://img.shields.io/badge/Org-CommonIntents--144-darkgray.svg)](https://github.com/CommonIntents)

**Version**: 0.1.0-draft  
**Status**: Working Group Internal Draft  
**Date**: 2026-05-22  
**License**: Apache 2.0  

---

## 1. Core Positioning

BIND-19 (INTENT-7 Binding) is the **adaptation layer between INTENT-7 and a concrete transport implementation**.

Its sole responsibility is to define how INTENT-7 intents are carried securely, efficiently, and completely over a specific transport protocol.

BIND-19 is the **ligament** of the protocol stack — flexible, thin, replaceable.

---

## 2. Why BIND-19 Is Needed

INTENT-7 is a transport-agnostic semantic standard. Cryptographic algorithms evolve, transport protocols change, serialization formats update. If such changes required modifying INTENT-7, the lifespan of INTENT-7 would be bound to the transport layer.

BIND-19 isolates all transport-related decisions into a single layer. When cryptographic technology advances, only BIND-19's binding target needs updating. INTENT-7 itself remains completely unaffected.

---

## 3. Core Responsibilities

### 3.1 Transport Format Negotiation

During the handshake phase, the two parties negotiate the transport format via `Content-Type` and `Accept` headers.

```
Client Request:
  Content-Type: application/cic13+msgpack
  Accept: application/cic13+msgpack, application/cic13+json

Server Response:
  Content-Type: application/cic13+msgpack
```

**Binary by default, JSON compatible.** If either party does not support binary, the system automatically falls back to standard JSON. When humans need to review, they can request a JSON endpoint on demand without affecting the production binary fast path.

Binary-to-JSON conversion is an O(n) formatting operation completed in microseconds.

### 3.2 Integrity Check Negotiation

BIND-19 introduces an optional integrity check at the frame layer.

```
BIND-19 Frame = Header (type + length) + Payload + Trailer (CRC32)
```

**Negotiation Mechanism:**

```
Client Request:
  X-BIND-19-Integrity: crc32

Server Response:
  X-BIND-19-Integrity: crc32
```

If both parties agree, it is enabled. If either party does not support or agree, it is skipped. **Activated on demand, disabled by default.**

When the underlying transport uses INTENT-7-SECURE (mTLS), TLS already provides integrity protection at the transport layer. The application-layer integrity check can be negotiated off to avoid redundant computation. In non-encrypted transport scenarios (such as local inter-process communication), it can be negotiated on.

### 3.3 Version Compatibility Declaration

BIND-19 declares its own version and the binding target during the handshake.

```
X-BIND-19-Version: 0.1.0
X-BIND-19-Binding: INTENT-7-SECURE/1.0
```

---

## 4. Current Binding Target

BIND-19 currently binds INTENT-7 to INTENT-7-SECURE (mTLS over HTTPS). Possible future binding targets include:

- **INTENT-7-SECURE-QUIC**: Secure transport based on QUIC
- **INTENT-7-SECURE-PQC**: Secure transport based on post-quantum cryptography
- **INTENT-7-SECURE-Local**: Local inter-process communication (zero network overhead)

All binding targets share the same BIND-19 negotiation mechanism. When the binding target is changed, the upper layers INTENT-7 and CAPABILITY-13 are completely unaffected.

---

## 5. Protocol Boundaries

BIND-19 **is responsible for**:
- Defining the transport format negotiation mechanism
- Defining the integrity check negotiation mechanism
- Defining the version compatibility declaration format
- Defining the BIND-19 frame structure

BIND-19 **is not responsible for**:
- Mandating which transport protocol must be used (chosen by the application)
- Mandating which encryption algorithm must be used (determined by the transport implementation)
- Mandating which binary format must be used (decided by negotiation)
- Providing the transport layer's security guarantees (provided by INTENT-7-SECURE or its alternatives)

---

## 6. Relationship with INTENT-7-SECURE

BIND-19 is the **specification**; INTENT-7-SECURE is the **implementation**.

BIND-19 defines: "Intent data is carried over a secure transport channel in a negotiated format."

INTENT-7-SECURE implements: "That secure channel is mTLS over HTTPS."

A future INTENT-7-SECURE-PQC implements: "That secure channel is mTLS with post-quantum cryptography over HTTPS."

BIND-19 does not change. INTENT-7 does not change. CAPABILITY-13 does not change. Only the transport implementation's version number changes.

---

## 7. Frame Structure Definition

### 7.1 Without Integrity Check

```
Header (1-byte type + 4-byte length) + Payload
```

### 7.2 With Integrity Check (Negotiated On)

```
Header (1-byte type + 4-byte length) + Payload + Trailer (4-byte CRC32)
```

The CRC32 covers all bytes of the header and payload.

Header defines the physical boundary of the frame; the top-level Envelope inside Payload defines frame type discrimination. See Chapter 8 for full Envelope specification.

---

## 8. Frame Envelope

### 8.1 Purpose

BIND-19 frame binary layout (Header + Payload) defines physical boundaries, but does not specify how to distinguish frame types. To eliminate overhead and ambiguity caused by blind try-parse for frame type detection on the receiver side, the top layer of Payload **MUST** be a unified Envelope structure.

### 8.2 Envelope structure

The envelope is a JSON or MessagePack object (per negotiated WireFormat) with the following fixed fields:

| Field | Type | Required | Description |
|------|------|------|------|
| `type` | string | Yes | Frame type: `"request"`, `"response"`, `"event"` |
| `id` | string | Yes | Session identifier for correlating requests and responses. Set to empty string `""` for event frames |
| `body` | object | Yes | Frame payload carrying business data |

### 8.3 Frame Type Definition

| Type Value | Direction | Description |
|----------|------|------|
| `"request"` | Client → Server | Client-initiated request (e.g. CAPABILITY-13 Action) |
| `"response"` | Server → Client | Server reply to a request |
| `"event"` | Server → Client | Server-initiated unsolicited event, no client response expected |

### 8.4 Predefined Events

When `type` is `"event"`, the `body` field **MUST** contain an `event` field to declare the event name.

| Event Name | Carried Data Type | Description |
|--------|-------------|------|
| `snapshot/update` | CAPABILITY-13 `SemanticSnapshot` | Agent state projection updated, client shall re-render UI |
| `manifest/update` | CAPABILITY-13 `CapabilityManifest` | Agent capability declaration changed |
| `heartbeat` | `{ "epoch": u64 }` | Agent liveness signal |

### 8.5 Heartbeat and Silent Period Suspend

- Agent **MUST** send at least one `heartbeat` event every **5 seconds** during an active connection.
- If a client receives no frames for more than **10 seconds**, it **SHOULD** mark the connection as lost and render degraded view.
- **Silent Period Suspend**: When the underlying transport (STDIO / UDS / TCP) disconnects, the Agent **MUST** immediately suspend the heartbeat timer. The heartbeat mechanism can only be re-enabled after a new client handshake is detected and completed. This ensures the Agent enters zero-power sleep state when no client is connected.
- The push interval of `snapshot/update` events **SHOULD** be no less than **150ms**. The Agent shall implement debounce logic to avoid client rendering jitter caused by frequent updates.

---

## 9. Protocol Boundaries Reaffirmed

BIND-19 is the thinnest layer in the protocol stack. Its existence is not to add new functionality, but to isolate change. It defines no new interaction semantics, introduces no new security mechanisms, and binds to no specific transport implementation.

**BIND-19 exists so that INTENT-7 never needs to know what transport layer it is running on.**

---

*This white paper is maintained by the INTENT-7/CAPABILITY-13 Protocol Working Group.*
