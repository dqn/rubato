# Porting TODO — Remaining Work

All phases (1–18g) complete. 1241 tests pass. See AGENTS.md for full status.

## Blocked Tasks

### Phase 16b: Golden Master Test Activation (partially complete)

- [ ] Add missing fixtures for modules not yet covered (modmenu, select bar, stream) — deferred until Rust-side APIs are implemented
- [ ] Reactivate `compare_render_snapshot.rs` — blocked: old crate names, SkinData→Skin pipeline gap, Lua loader stubbed. Requires full API rewrite + loading pipeline

### Phase 18e: Stub replacement (remaining items blocked)

- [ ] Replace `MainState` stubs with real trait impls — blocked: requires per-screen concrete types (PlayState, SelectState, etc.)
- [ ] Remove all `stubs.rs` files — blocked: depends on above + rendering/IR/database implementations
- [ ] beatoraja-external LibGDX stubs (Pixmap/GdxGraphics/BufferUtils/PixmapIO) — blocked on wgpu rendering pipeline

### Phase 18f: Integration verification (remaining items blocked)

- [ ] Activate `compare_render_snapshot.rs` — blocked: SkinData→Skin pipeline, Lua loader
- [ ] E2E gameplay flow test: select → decide → play → result screen transitions — blocked: requires all stubs removed
- [ ] Final verification: all tests pass, zero clippy warnings, clean `cargo fmt` — blocked: final gate

### Known Issues (open)

- [ ] JSONSkinLoader returns `SkinData` (intermediate), not `Skin` — full loading pipeline not connected
- [ ] LuaSkinLoader completely stubbed — `load_header()` and `load_skin()` return None
- [ ] All remaining stubs (16 files, ~2,440 lines) exhaustively audited (4 rounds) — blocked by rendering, IR network, database, per-screen implementations
