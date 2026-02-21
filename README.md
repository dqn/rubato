# brs — beatoraja Rust Port

Mechanical line-by-line translation of [lr2oraja](https://github.com/exch-bms2/beatoraja) (Java) to Rust.

## Project Structure

```
brs/
  lr2oraja-java/           # Java source (reference implementation, read-only)
  lr2oraja-rust/           # Rust port (Cargo workspace)
    crates/
      bms-model/           # BMS/BMSON/osu! format parser
      bms-table/           # LR2 course table parser
    golden-master/         # Test infrastructure
    test-bms/              # Test BMS files
```

## Crates

| Crate | Description | Status |
|-------|-------------|--------|
| `bms-model` | BMS, BMSON, osu! format parser and decoder | Phase 1-2 complete |
| `bms-table` | LR2 course table parser | Phase 1 complete |

## Implementation Progress

- **Phase 1** (Core Foundation): `bms.model` (15 modules), `bms.table` (11 modules)
- **Phase 2** (Format Variants): `bms.model.bmson` (BMSONDecoder + 16 model types), `bms.model.osu` (OSUDecoder + 9 model types)

See [TODO.md](TODO.md) for the full porting roadmap.

## Building

```sh
cd lr2oraja-rust
cargo check
cargo test
```

## Tech Stack

| Area | Java (original) | Rust (port) |
|------|-----------------|-------------|
| Graphics | LibGDX (LWJGL3) | Bevy |
| Audio | PortAudio / GDX | Kira |
| Skin (Lua) | LuaJ | mlua |
| Database | SQLite (JDBC) | rusqlite |
| GUI | JavaFX / ImGui | egui |
| Discord RPC | JNA IPC | discord-rich-presence |
| OBS | WebSocket | tokio-tungstenite |
