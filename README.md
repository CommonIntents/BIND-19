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

## Read the Spec
- [BIND-19 v1.0.0-RFC-4](spec/BIND-19.md)
- [中文版](spec/BIND-19.zh-CN.md)

## Related
| Protocol | Repository |
|:---|:---|
| INTENT-7 | [CommonIntents/INTENT-7](https://github.com/CommonIntents/INTENT-7) |
| CAPABILITY-13 | [CommonIntents/CAPABILITY-13](https://github.com/CommonIntents/CAPABILITY-13) |
| INTENT-7-SECURE | [CommonIntents/INTENT-7-SECURE](https://github.com/CommonIntents/INTENT-7-SECURE) |

## License
Apache 2.0 — see [LICENSE](LICENSE).
