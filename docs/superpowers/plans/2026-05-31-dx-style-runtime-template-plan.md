# DX Style Runtime Validation Template Plan

## Goal

Emit a review-only runtime validation receipt template from source-apply review packets so the future runtime proof window has a concrete packet shape to satisfy.

## Constraints

- Do not run `just run`.
- Do not run Cargo, broad builds, servers, browser automation, or live WebView validation.
- Keep mutation disabled and report runtime proof as unverified.
- Keep the template connected to source-owned bridge fields and real source review evidence.

## Steps

1. Add `runtime_validation_receipt_template` to the source-owned source-apply and editor write-bridge review field lists.
2. Emit the template from native source-apply review using the native writer commit plan, post-write digest plan, and bridge runtime receipt checklist.
3. Emit a copied Web Preview template packet that honestly reports `not_performed_in_web_preview`.
4. Add readiness blockers when the runtime template field is missing.
5. Update source guards, `DX.md`, `todo.txt`, and `changelog.txt`.
6. Run source-only checks and commit the slice.
