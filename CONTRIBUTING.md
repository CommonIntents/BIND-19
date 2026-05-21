# Contributing to CIB

CIB (CIS/Transport Binding Protocol) is the ligament of the protocol family.

## Specification Files

- English: [spec/CIB.md](spec/CIB.md)
- Chinese: [spec/CIB.zh-CN.md](spec/CIB.zh-CN.md)

## How to Propose Changes

1. Read the [organization-level contribution guide](https://github.com/CommonIntents/.github/blob/main/CONTRIBUTING.md)
2. Open an Issue describing the problem or improvement you've identified
3. If proposing a new format or integrity check, describe the negotiation mechanism
4. Update both the English and Chinese versions in your PR

## Design Constraint

CIB is the thinnest layer in the protocol stack. It defines no new interaction semantics.
Proposals that add semantic content to CIB will be redirected to CIS or CAP.
