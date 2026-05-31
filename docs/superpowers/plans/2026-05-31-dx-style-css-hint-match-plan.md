# DX Style CSS Hint Match Plan

## Goal

Make `CSS declaration hint provenance match` a real source-owned guard, not only
a named contract requirement. A CSS declaration review must prove the hint packet
matches the active editor context before native review receipts can be trusted.

## Scope

- Add source-owned required CSS declaration hint fields to the DX Style
  source-apply and CSS declaration dry-run contracts.
- Mirror the updated fixtures into Zed's generated fallback artifacts.
- Add Web Preview diagnostics for missing or mismatched hint fields.
- Add native review validation that compares the hint packet against the active
  context.
- Update source guards and handoff docs.

## Non-Goals

- Do not enable source mutation.
- Do not run `just run`, Cargo, builds, servers, or WebView runtime proof.
- Do not broaden visual generator UI scope.

## Checklist

- [x] Update `G:\Dx\style` source contracts and fixtures.
- [x] Sync generated Zed fixture mirrors.
- [x] Enforce hint-field diagnostics in Web Preview.
- [x] Enforce hint-field validation in native source-apply review.
- [x] Update source guards and docs.
- [x] Run allowed lightweight checks.
- [x] Commit only this slice.
