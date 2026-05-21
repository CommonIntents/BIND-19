# CIB — CIS/Transport Binding Protocol

[![Org](https://img.shields.io/badge/Org-CommonIntents-darkgray.svg)](https://github.com/CommonIntents)

**Flexible, thin, replaceable.**

CIB is the adaptation layer between CIS and concrete transport implementations. It defines *how* CIS intents are carried — format negotiation, integrity checks, and version compatibility declaration.

CIB is the **ligament** of the CIS/CAP protocol family — it exists so CIS never needs to know what transport layer it runs on.

## What CIB Negotiates

- **Transport format** — binary (MessagePack/CBOR) by default, JSON fallback
- **Integrity check** — optional CRC32 at frame level, disabled under mTLS
- **Version binding** — declares CIB version and current binding target (e.g., CISS/1.0)

## Protocol Stack

```
CIS  (intent semantics)
 ↑
CIB  ← You are here
 ↑
CISS (mTLS security)
 ↑
CAP  (capability auth & HITL)
```

## Read the Spec

- [CIB v0.1.0-draft](spec/CIB.md)
- [中文版](spec/CIB.zh-CN.md)

## Related

| Protocol | Repository |
|----------|------------|
| CIS | [CommonIntents/CIS](https://github.com/CommonIntents/CIS) |
| CAP | [CommonIntents/CAP](https://github.com/CommonIntents/CAP) |
| CISS | [CommonIntents/CISS](https://github.com/CommonIntents/CISS) |

## License

Apache 2.0 — see [LICENSE](LICENSE).
