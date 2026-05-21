# CIB: CIS/Transport Binding Protocol

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0) [![Version](https://img.shields.io/badge/Version-0.1.0--draft-orange.svg)]() [![Status](https://img.shields.io/badge/Status-RFC%20Draft-yellow.svg)]() [![Org](https://img.shields.io/badge/Org-CommonIntents-darkgray.svg)](https://github.com/CommonIntents)

**Version**: 0.1.0-draft  
**Status**: Working Group Internal Draft  
**Date**: 2026-05-22  
**License**: Apache 2.0  

---

## 1. Core Positioning

CIB (CIS Binding) is the **adaptation layer between CIS and a concrete transport implementation**.

Its sole responsibility is to define how CIS intents are carried securely, efficiently, and completely over a specific transport protocol.

CIB is the **ligament** of the protocol stack — flexible, thin, replaceable.

---

## 2. Why CIB Is Needed

CIS is a transport-agnostic semantic standard. Cryptographic algorithms evolve, transport protocols change, serialization formats update. If such changes required modifying CIS, the lifespan of CIS would be bound to the transport layer.

CIB isolates all transport-related decisions into a single layer. When cryptographic technology advances, only CIB's binding target needs updating. CIS itself remains completely unaffected.

---

## 3. Core Responsibilities

### 3.1 Transport Format Negotiation

During the handshake phase, the two parties negotiate the transport format via `Content-Type` and `Accept` headers.

```
Client Request:
  Content-Type: application/cap+msgpack
  Accept: application/cap+msgpack, application/cap+json

Server Response:
  Content-Type: application/cap+msgpack
```

**Binary by default, JSON compatible.** If either party does not support binary, the system automatically falls back to standard JSON. When humans need to review, they can request a JSON endpoint on demand without affecting the production binary fast path.

Binary-to-JSON conversion is an O(n) formatting operation completed in microseconds.

### 3.2 Integrity Check Negotiation

CIB introduces an optional integrity check at the frame layer.

```
CIB Frame = Header (type + length) + Payload + Trailer (CRC32)
```

**Negotiation Mechanism:**

```
Client Request:
  X-CIB-Integrity: crc32

Server Response:
  X-CIB-Integrity: crc32
```

If both parties agree, it is enabled. If either party does not support or agree, it is skipped. **Activated on demand, disabled by default.**

When the underlying transport uses CISS (mTLS), TLS already provides integrity protection at the transport layer. The application-layer integrity check can be negotiated off to avoid redundant computation. In non-encrypted transport scenarios (such as local inter-process communication), it can be negotiated on.

### 3.3 Version Compatibility Declaration

CIB declares its own version and the binding target during the handshake.

```
X-CIB-Version: 0.1.0
X-CIB-Binding: CISS/1.0
```

---

## 4. Current Binding Target

CIB currently binds CIS to CISS (mTLS over HTTPS). Possible future binding targets include:

- **CISS-QUIC**: Secure transport based on QUIC
- **CISS-PQC**: Secure transport based on post-quantum cryptography
- **CISS-Local**: Local inter-process communication (zero network overhead)

All binding targets share the same CIB negotiation mechanism. When the binding target is changed, the upper layers CIS and CAP are completely unaffected.

---

## 5. Protocol Boundaries

CIB **is responsible for**:
- Defining the transport format negotiation mechanism
- Defining the integrity check negotiation mechanism
- Defining the version compatibility declaration format
- Defining the CIB frame structure

CIB **is not responsible for**:
- Mandating which transport protocol must be used (chosen by the application)
- Mandating which encryption algorithm must be used (determined by the transport implementation)
- Mandating which binary format must be used (decided by negotiation)
- Providing the transport layer's security guarantees (provided by CISS or its alternatives)

---

## 6. Relationship with CISS

CIB is the **specification**; CISS is the **implementation**.

CIB defines: "Intent data is carried over a secure transport channel in a negotiated format."

CISS implements: "That secure channel is mTLS over HTTPS."

A future CISS-PQC implements: "That secure channel is mTLS with post-quantum cryptography over HTTPS."

CIB does not change. CIS does not change. CAP does not change. Only the transport implementation's version number changes.

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

---

## 8. Protocol Boundaries Reaffirmed

CIB is the thinnest layer in the protocol stack. Its existence is not to add new functionality, but to isolate change. It defines no new interaction semantics, introduces no new security mechanisms, and binds to no specific transport implementation.

**CIB exists so that CIS never needs to know what transport layer it is running on.**

---

*This white paper is maintained by the CIS/CAP Protocol Working Group.*
