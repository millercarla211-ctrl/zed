# DX Style CSS Hint Schema Plan

## Goal

Make the expected CSS declaration hint packet schema source-owned by DX Style
contracts instead of only hardcoded in Zed/Web Preview.

## Scope

- Add `css_declaration_hint_schema` to the DX Style source-apply and CSS
  declaration dry-run contracts and fixtures.
- Sync Zed generated fixture mirrors.
- Make Web Preview compare the source-apply and dry-run schema values before IPC.
- Make native source-apply review validate hint packets against the contract
  schema value, with a fail-closed fallback.
- Update source guards and docs.

## Non-Goals

- Do not enable mutation.
- Do not run `just run`, Cargo, builds, servers, or WebView runtime proof.

## Checklist

- [x] Update `G:\Dx\style` contracts and fixtures.
- [x] Sync generated Zed fixture mirrors.
- [x] Validate schema source ownership in Web Preview.
- [x] Validate schema source ownership in native review.
- [x] Update guards and docs.
- [x] Run allowed lightweight checks.
- [x] Commit only this slice.
