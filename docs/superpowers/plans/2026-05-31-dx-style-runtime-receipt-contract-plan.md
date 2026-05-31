# DX Style Runtime Receipt Contract Plan

## Goal

Make the remaining runtime validation proof boundary source-owned and concrete by naming the runtime validation receipt schema plus the fields future authorized runtime proof must provide.

## Constraints

- Do not run `just run`.
- Do not run Cargo, broad builds, servers, browser automation, or live WebView validation.
- Keep `can_mutate_source` and source mutation disabled.
- Treat this as contract/readiness hardening, not runtime proof.

## Steps

1. Add `runtime_validation_receipt_schema` and `required_runtime_validation_receipt_fields` to the DX Style editor write-bridge preflight contract and fixture.
2. Expose those fields through the Zed Style editor write-bridge snapshot and Web Preview bridge packet.
3. Make native and Web Preview source-write readiness report named blockers for missing/unsupported runtime receipt schema or fields.
4. Extend source guards so drift in the runtime receipt checklist fails lightweight verification.
5. Update `DX.md`, `todo.txt`, and `changelog.txt`.
6. Run only source-only checks and commit the slice.
