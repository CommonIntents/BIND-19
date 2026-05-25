# CIB — CIS/Transport Binding Protocol [![Org](https://img.shields.io/badge/Org-CommonIntents-darkgray.svg)](https://github.com/CommonIntents)

**Flexible, thin, replaceable.** CIB is the adaptation layer between CIS (intent syntax) and concrete transport implementations. It defines *how* CIS intents are carried — format negotiation, integrity checks, and version compatibility declaration.

## What CIB Negotiates
- **Transport format** — binary (MessagePack/CBOR) by default, JSON fallback
- **Integrity check** — optional CRC32 at frame level, disabled under mTLS
- **Version binding** — declares CIB version and current binding target

## Protocol Stack
```
CIS (intent syntax — SIDL)
  ↑
CIB ← You are here (transport binding)
  ↑
CISS (optional mTLS reference implementation)
  ↑
CAP (consensus confirmation)
```

## Read the Spec
- [CIB v0.1.0-draft](spec/CIB.md)
- [中文版](spec/CIB.zh-CN.md)

## Related
| Protocol | Repository |
|:---|:---|
| CIS | [CommonIntents/CIS](https://github.com/CommonIntents/CIS) |
| CAP | [CommonIntents/CAP](https://github.com/CommonIntents/CAP) |
| CISS | [CommonIntents/CISS](https://github.com/CommonIntents/CISS) |

## License
Apache 2.0 — see [LICENSE](LICENSE).
