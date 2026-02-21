# Porting TODO — Mechanical Line-by-Line Translation

Dependency graph order. Each module is ported only after its dependencies are complete.

## Phase 1: Core Foundation (~11,178 lines)

Zero internal dependencies. Port these first.

- [x] `bms.model` (19 files, 8,070 lines) — BMS format parser
- [x] `bms.table` (10 files, 3,108 lines) — LR2 course table parser

## Phase 2: Format Variants (~1,802 lines)

Depends on: bms.model

- [x] `bms.model.bmson` (16 files, 526 lines) — BMSON parser
- [x] `bms.model.osu` (9 files, 1,276 lines) — osu! format converter

## Phase 3: Low-level Subsystems (~12,050 lines)

Isolated subsystems with minimal internal deps.

- [ ] `beatoraja.exceptions` (1 file, 7 lines) — Exception definitions
- [ ] `beatoraja.system` (1 file, 139 lines) — File utilities
- [ ] `tool.util` (1 file, 121 lines) — Generic utilities
- [ ] `beatoraja.controller` (3 files, 762 lines) — Gamepad management
- [ ] `beatoraja.external.DiscordRPC` (4 files, 634 lines) — Discord RPC
- [ ] `beatoraja.input` (8 files, 4,188 lines) — Keyboard/MIDI input
- [ ] `beatoraja.audio` (14 files, 7,086 lines) — Audio playback (Kira)
- [ ] `tool.mdprocessor` (9 files, 2,264 lines) — Download/song processing

## Phase 4: Configuration & Central State (~26,290 lines)

Hub module — most others depend on this.

- [ ] `beatoraja.config` (4 files, 2,582 lines) — Config definitions
- [ ] `beatoraja` root (44 files, 23,708 lines) — Central state/data classes

## Phase 5: Pattern & Gameplay (~17,692 lines)

Core gameplay logic. Depends on: config, model, input, audio.

- [ ] `beatoraja.pattern` (14 files, 4,108 lines) — Lane/note shuffle
- [ ] `beatoraja.play` (23 files, 13,584 lines) — Judge, gauge, game loop
  - [ ] `beatoraja.play.bga` (5 files, 1,802 lines) — BGA playback

## Phase 6: Skin System (~15,594 lines)

Multi-format skin rendering. Depends on: config, model, play.

- [ ] `beatoraja.skin` base (34 files, 15,594 lines) — Skin rendering engine
  - [ ] `beatoraja.skin.json` (11 files, 5,456 lines) — JSON skin loader
  - [ ] `beatoraja.skin.lr2` (10 files, 6,482 lines) — LR2 skin loader
  - [ ] `beatoraja.skin.lua` (5 files, 2,480 lines) — Lua skin loader
  - [ ] `beatoraja.skin.property` (13 files, 8,216 lines) — Property binding

## Phase 7: Screen Implementations (~11,900 lines)

UI screens. Depends on: skin, config, play.

- [ ] `beatoraja.select` (13 files, 8,386 lines) — Song select screen
  - [ ] `beatoraja.select.bar` (17 files, 3,514 lines) — Bar rendering
- [ ] `beatoraja.result` (7 files, 3,122 lines) — Result screen
- [ ] `beatoraja.decide` (2 files, 172 lines) — Decide screen

## Phase 8: Advanced Features (~15,946 lines)

Optional/peripheral features.

- [ ] `beatoraja.ir` (14 files, 3,572 lines) — Internet ranking
- [ ] `beatoraja.external` (7 files, 2,076 lines) — OBS, webhooks
- [ ] `beatoraja.obs` (2 files, 1,502 lines) — OBS WebSocket
- [ ] `beatoraja.modmenu` (15 files, 8,468 lines) — In-game mod menu
- [ ] `beatoraja.stream` (3 files, 402 lines) — Stream commands

## Phase 9: Launcher (~9,210 lines)

Standalone GUI. Can be deferred.

- [ ] `beatoraja.launcher` (21 files, 9,210 lines) — Settings GUI (egui)

---

## Testing Checkpoints

| After Phase | What you can test |
|-------------|-------------------|
| 1 | BMS parsing independently (Golden Master) |
| 2 | All format variants (BMS, BMSON, osu!) |
| 3 | Input/audio subsystems |
| 5 | Full gameplay logic with judge calculations |
| 6 | Skin rendering with actual skins |
| 7 | Full game flow (select → play → result) |
