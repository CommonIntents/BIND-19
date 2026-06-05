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
