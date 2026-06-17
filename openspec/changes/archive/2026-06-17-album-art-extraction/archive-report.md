# Archive Report: album-art-extraction

**Change**: album-art-extraction
**Archived**: 2026-06-17
**Mode**: hybrid (OpenSpec + Engram)
**Verdict**: PASS WITH WARNINGS

## Artifact Inventory

| Artifact | Engram ID | OpenSpec Path | Status |
|----------|-----------|---------------|--------|
| Proposal | #545 | proposal.md | ✅ |
| Spec | #546 | specs/album-art-cache/spec.md | ✅ |
| Design | #547 | design.md | ✅ |
| Tasks | #548 | tasks.md | ✅ |
| Apply-progress | #549 | apply-progress.md | ✅ |
| Verify-report | #550 | verify-report.md | ✅ |

## Specs Synced

| Domain | Action | Details |
|--------|--------|---------|
| album-art-cache | Created | 6 requirements, 12 scenarios added (new domain) |

No main spec existed for `album-art-cache` — delta spec copied directly as source of truth.

## Archive Location

`openspec/changes/archive/2026-06-17-album-art-extraction/`

## Verification Summary

- 17/17 tasks complete
- 175 Rust tests passed, 26 Vitest tests passed
- 12/12 spec scenarios compliant
- 0 CRITICAL issues
- 2 WARNINGs (integration test fixtures gated behind `integration` feature; Vite mixed static/dynamic import warning)
- 3 SUGGESTIONs (add integration feature to Cargo.toml, dynamic import for assetUrl.ts, pop() loop unit test)

## Source of Truth Updated

`openspec/specs/album-art-cache/spec.md` — now the authoritative spec for album art cache domain.

## Lessons Learned

- Symphonia `Metadata.pop()` loop required (not `skip_to_latest`) — skip_to_latest discards earlier revisions losing tags
- `protocol-asset` feature needed in tauri Cargo.toml for Tauri v2 when `assetProtocol.enable` is set (not in original design)
- sha2 is already a transitive dep via tauri-codegen but must be added explicitly for direct usage
- Vite mixed static/dynamic import warning from `@tauri-apps/api/core` — non-blocking but should be resolved
- Integration tests gated behind `integration` feature flag require fixture files and manual `cargo test --features integration`