# Security Policy
**Version**: v1.0.0-RFC-4  
**Status**: Active  
**Last Updated**: 2026-06-26

---

## 1. Security Philosophy
CI-144 is built on **Zero Trust Architecture** with the following core principles:

| Principle | Implementation |
| :--- | :--- |
| **1. Trust Nothing by Default** | All connections must authenticate via mTLS (public) or SO_PEERCRED (local) |
| **2. Least Privilege** | No permissions are granted by default; all must be explicitly declared and approved |
| **3. Defense in Depth** | Security enforced at 4 layers: Semantic, Capability, Transport, Security |
| **4. No Security by Obscurity** | All cryptographic algorithms are public standards (TLS 1.3, Ed25519) |
| **5. Auditability** | Every transaction traceable via W3C Trace Context standard |

---

## 2. Supported Versions
Only the following versions receive security updates:

| Version | Supported | End of Life |
| :---: | :---: | :--- |
| **1.0.0-RFC-4** | ✅ Yes | Not scheduled |
| < 1.0.0 | ❌ No | Already ended |

---

## 3. Security Architecture
CI-144 implements a 4-layer security model:
```
┌─────────────────────────────────────────────────────────┐
│ Layer 4: Semantic Security (INTENT-7)                   │
│   • JSON Schema validation                              │
│   • Intent verb whitelist                               │
│   • Metadata integrity check                            │
└─────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────┐
│ Layer 3: Capability Security (CAPABILITY-13)            │
│   • Permission mapping verification                     │
│   • Ed25519 signature verification                      │
│   • HITL consensus for high-risk operations             │
└─────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────┐
│ Layer 2: Transport Security (BIND-19)                   │
│   • Frame integrity checksum                            │
│   • Sequence-based anti-replay                          │
│   • Version negotiation downgrade protection            │
└─────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────┐
│ Layer 1: Cryptographic Security (INTENT-7-SECURE)       │
│   • mTLS 1.3 (public networks)                          │
│   • SO_PEERCRED (local UDS)                             │
│   • Certificate pinning (L0 Gene Lock)                  │
│   • Credential isolation (Edge Gateway + KMS)           │
└─────────────────────────────────────────────────────────┘
```

### 3.1 Layer 1: Cryptographic Security

| Mechanism | Standard | Purpose |
| :--- | :--- | :--- |
| **mTLS 1.3** | RFC 8446 | Mutual authentication, confidentiality, integrity |
| **Cipher Suites** | TLS_AES_256_GCM_SHA384, TLS_CHACHA20_POLY1305_SHA256 | Strong encryption, AEAD |
| **Certificate Pinning** | SHA-256 fingerprint in L0 Gene Lock | Prevent MITM from compromised CA |
| **SO_PEERCRED** | POSIX.1-2001 | Local identity verification (0ms overhead) |
| **Key Rotation** | Ed25519, atomic transaction | Forward secrecy, key compromise recovery |

### 3.2 Layer 2: Transport Security

| Mechanism | Description |
| :--- | :--- |
| **Frame Integrity** | Each BIND-19 frame includes checksum |
| **Anti-Replay** | Monotonically increasing Sequence ID per Channel |
| **Version Negotiation** | Only highest common version accepted |

### 3.3 Layer 3: Capability Security

| Mechanism | Description |
| :--- | :--- |
| **Permission Mapping** | `capability_mapping.toml` defines allowed capabilities |
| **Signature Verification** | Ed25519 signature verifies mapping authenticity |
| **HITL Consensus** | Human-in-the-loop approval for high-risk operations |
| **Key Rotation** | GPG-style key rotation with rollback protection |

### 3.4 Layer 4: Semantic Security

| Mechanism | Description |
| :--- | :--- |
| **Schema Validation** | All INTENT-7 payloads validated against JSON Schema |
| **Verb Whitelist** | Only defined ACTIONs are accepted |
| **Trace Context** | W3C `traceparent` bound to TLS session |

---

## 4. Threat Model

### 4.1 Assumed Attacker Capabilities

| Capability | Mitigation |
| :--- | :--- |
| **Network Packet Sniffing** | mTLS encrypts all traffic |
| **Packet Injection** | mTLS provides integrity protection |
| **Replay Attack** | Sequence ID in BIND-19 prevents replay |
| **Man-in-the-Middle (MITM)** | mTLS with certificate pinning |
| **Compromised CA** | Certificate pinning in L0 Gene Lock |
| **Local Privilege Escalation** | SO_PEERCRED verifies UID/GID |
| **Credential Theft** | Credentials isolated in Edge Gateway KMS |

### 4.2 Not in Scope

| Attack Type | Reason |
| :--- | :--- |
| **Physical Compromise** | Out of scope; assume secure hardware |
| **Side-Channel Attacks** | Mitigated by constant-time implementations (if applicable) |
| **DoS Attacks** | Rate limiting at application layer |

---

## 5. Vulnerability Reporting

### 5.1 Reporting Process
**DO NOT** create public GitHub Issues for security vulnerabilities.

Instead:
1. **Email**: Send details to `security@commonintents.org`
2. **Encrypt**: Use our PGP key (fingerprint in L0 Gene Lock)
3. **Include**:
   - Affected protocol and version
   - Vulnerability description
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

### 5.2 Response Timeline

| Milestone | Target | Maximum |
| :--- | :---: | :---: |
| **Initial Acknowledgment** | 24 hours | 48 hours |
| **Severity Assessment** | 72 hours | 7 days |
| **Critical Patch** | 14 days | 30 days |
| **Minor Patch** | 30 days | 60 days |
| **Public Disclosure** | After patch release | 90 days |

### 5.3 Severity Rating
We use [CVSS v3.1](https://www.first.org/cvss/) for severity assessment:

| Severity | CVSS Range | Example |
| :---: | :--- | :--- |
| **Critical** | 9.0 - 10.0 | Remote code execution without authentication |
| **High** | 7.0 - 8.9 | Authentication bypass, private key leak |
| **Medium** | 4.0 - 6.9 | Permission boundary violation |
| **Low** | 0.1 - 3.9 | Information disclosure, DoS |

---

## 6. Cryptographic Key Management

### 6.1 Key Types

| Key Type | Algorithm | Purpose | Rotation |
| :--- | :--- | :--- | :--- |
| **L0 Creator Key** | Ed25519 | Sign capability mappings | 90 days recommended |
| **TLS Certificate** | X.509 v3 with Ed25519/ECDSA | mTLS authentication | 1 year recommended |
| **Trace ID** | 128-bit random | Transaction tracing | Per transaction |

### 6.2 Key Rotation Procedure
Key rotation follows the **CAPABILITY-13 Key Rotation Protocol**:
1. **Generate New Key**: Create new Ed25519 key pair
2. **Sign Transaction**: Create `KeyRotationTransaction` signed by old key
3. **Multi-Signature**: Requires 3-of-3 signature from Seed instances
4. **Atomic Replacement**: All mappings re-signed with new key
5. **Announcement**: Publish rotation event via BIND-19 Control Frame

### 6.3 Key Compromise Response
If a private key is compromised:
1. **Immediate**: Revoke compromised key via emergency transaction
2. **Short-Term**: Use backup key for signing
3. **Long-Term**: Complete key rotation with new key pair
4. **Notification**: Announce via all channels (GitHub, email, etc.)

---

## 7. Security Best Practices for Implementers

### 7.1 Transport Layer
```rust
// Example: BIND-19 frame validation
fn validate_frame(frame: &Frame) -> Result<(), SecurityError> {
    // 1. Check version compatibility
    if frame.version != BIND_19_VERSION {
        return Err(SecurityError::VersionMismatch);
    }
    
    // 2. Verify checksum
    if !frame.verify_checksum() {
        return Err(SecurityError::IntegrityCheckFailed);
    }
    
    // 3. Check sequence ID (anti-replay)
    if frame.sequence_id <= self.last_sequence_id[frame.channel_id] {
        return Err(SecurityError::ReplayDetected);
    }
    
    Ok(())
}
```

### 7.2 Security Layer
```rust
// Example: mTLS configuration
fn configure_mtls(config: &mut ServerConfig) {
    // 1. Set minimum TLS version to 1.3
    config.set_min_proto_version(ProtocolVersion::TLSv1_3);
    
    // 2. Restrict cipher suites
    config.set_cipher_suites(&[
        CipherSuite::TLS_AES_256_GCM_SHA384,
        CipherSuite::TLS_CHACHA20_POLY1305_SHA256,
    ]);
    
    // 3. Configure client certificate verification
    config.set_client_certificate_verifier(
        ClientCertificateVerifier::new()
            .require_client_cert(true)
            .add_trusted_fingerprint(gene_lock_fingerprint),
    );
}
```

### 7.3 Credential Isolation
```rust
// Example: Edge Gateway credential injection
async fn inject_credentials(
    request: &mut Request,
    identity_label: &str,
) -> Result<(), SecurityError> {
    // 1. Parse identity label (namespace/identifier@instance-id)
    let label = IdentityLabel::parse(identity_label)?;
    
    // 2. Retrieve decrypted credential from KMS
    let credential = kms.retrieve_credential(&label).await?;
    
    // 3. Inject into HTTP header
    request.headers_mut().insert(
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Bearer {}", credential.token))?,
    );
    
    Ok(())
}
```

---

## 8. Security Audit Trail
Every transaction in CI-144 generates an audit trail:

| Field | Source | Purpose |
| :--- | :--- | :--- |
| `traceparent` | INTENT-7 metadata | Global transaction ID |
| `TLS Session ID` | INTENT-7-SECURE | Cryptographic session binding |
| `Certificate Fingerprint` | mTLS | Client/server identity |
| `UID/GID` | SO_PEERCRED | Local process identity |
| `Capability Mapping Hash` | CAPABILITY-13 | Permission configuration state |
| `HXR Record` | INTENT-7 | Execution history |

---

## 9. Security Updates Policy

### 9.1 Patch Release Criteria
A security patch will be released when:
- A vulnerability is reported and confirmed
- The CVSS score is ≥ 4.0 (Medium)
- A working exploit exists in the wild
- The vulnerability affects a supported version

### 9.2 Backward Compatibility
Security patches will **NOT** break backward compatibility unless absolutely necessary.

If a security fix requires breaking changes:
- It will be released as a new major version
- Migration guide will be provided
- 6-month overlap support for previous major version

---

## 10. Security Contact Information
- **Security Email**: `security@commonintents.org`
- **PGP Key**: `pgp_keys.asc` in repository root
- **Fingerprint**: Published in L0 Gene Lock
- **Response Time**: As per Section 5.2

---

## 11. Acknowledgments
We welcome responsible disclosure of security vulnerabilities.  
Contributors will be credited in security advisories unless they wish to remain anonymous.

---

**"Trust must be proven, not assumed."**
