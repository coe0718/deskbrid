# Deskbrid Docs Archive

Reference material, design specs, and historical records. Not the primary documentation
— see [deskbrid.patchhive.dev](https://deskbrid.patchhive.dev) for the live docs.

## External Reviews & Audits

| File | Author | Description |
|------|--------|-------------|
| [`CODE_REVIEW_VEX.md`](CODE_REVIEW_VEX.md) | Vex | Security audit — 40 findings across 4 criticals, 26 warnings, 7 suggestions, 2 informational. All resolved in v0.13.0→v1.0.0. |
| [`CHANGELOGv1.0.0.md`](CHANGELOGv1.0.0.md) | Scout | v1.0.0 release changelog — full scope, breaking changes, migration notes. |

## Design Specs

| File | Author | Feature |
|------|--------|---------|
| [`design/029-keyring.md`](design/029-keyring.md) | Scout | #29 Secret Service / Keyring — architecture, security model, zeroize, confirmation gating. |
| [`design/083-rules-engine.md`](design/083-rules-engine.md) | Scout | #83 Rules Engine — triggers, conditions, time ranges, persistence. |

## Configuration

| File | Description |
|------|-------------|
| [`permissions.example.toml`](permissions.example.toml) | Complete permissions reference — all 230+ actions, wildcard syntax, high-risk marking, presets for read-only and full-control. |

## Wiki

The [`wiki/`](wiki/) directory contains the source for [docs.deskbrid.patchhive.dev](https://docs.deskbrid.patchhive.dev) — architecture, protocol, features, integrations, and installation guides.
