# DX Style Post-Write Digest Verification Plan

## Goal

Make the DX Style source write bridge require a review-only post-write digest verification plan before any future mutation-capable path can be considered ready.

## Constraints

- Do not run `just run`.
- Do not run Cargo or broad build/check/lint commands.
- Keep source mutation disabled until the authorized runtime proof exists.
- Preserve existing review behavior while adding stricter readiness evidence.

## Steps

1. Add `post-write source digest verification plan` to DX Style source-owned editor guards.
2. Add `post_write_digest_verification_plan` to DX Style source-owned review receipt fields.
3. Sync the generated Zed source-apply contract mirror.
4. Wire native source-apply receipts to emit a review-only post-write digest plan derived from the native dry-run commit plan.
5. Wire Web Preview readiness to report that the plan cannot be performed in-preview.
6. Update source guard tests, `DX.md`, `todo.txt`, and `changelog.txt`.
7. Run only lightweight source checks and commit the slice.
