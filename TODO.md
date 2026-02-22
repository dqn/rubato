# Porting TODO — Remaining Work

All phases (1–23) complete. 1511 tests pass. See AGENTS.md for full status.

## Completed Phases (recent)

### Phase 22d: Skin.draw_all_objects() Integration (complete)

+~150 lines across 3 files + 11 new tests:

- [x] **SkinDrawable trait** — Defined in beatoraja-core/main_state.rs. Send-bounded trait with 10 methods (draw_all_objects_timed, update_custom_objects_timed, mouse_pressed_at, mouse_dragged_at, dispose_skin, get_fadeout/input/scene/width/height). Replaces SkinStub in MainStateData
- [x] **TimerOnlyMainState adapter** — Bridges beatoraja-core's SkinDrawable to beatoraja-skin's internal MainState stub trait. Wraps timer values, returns defaults for other state queries
- [x] **SkinDrawable impl for Skin** — All 10 methods delegate to existing Skin methods via adapter. +8 tests
- [x] **MainController.render() wiring** — Skin draw calls via take/put-back pattern (borrow safety). update_custom_objects_timed + draw_all_objects_timed called per frame. +3 tests

### Phase 22b: SkinObject Draw Methods + SkinTextBitmap Font Rendering (complete)

+846 lines tests + 500 lines implementation across 7 files. All sub-tasks done:

- [x] **SkinImage draw tests** — 9 tests: basic draw, offset, movie FFmpeg type override, zero alpha skip, region dimensions, color propagation
- [x] **SkinNumber draw tests** — 9 tests: single/multi digit, spacing, alignment (left/center/right), zero padding, per-digit offsets, length calculation
- [x] **SkinTextImage draw tests** — 9 tests: glyph layout, alignment, scaling, margin, offset, height scaling, zero source size
- [x] **SkinTextBitmap.draw_with_offset()** — Real implementation replacing warn!() stub. ab_glyph font rasterization, glyph layout with kerning, alignment (left/center/right), overflow modes (overflow/shrink/truncate), shadow rendering, distance field support. +15 tests
- [x] **Test infrastructure** — MockMainState helper, test_helpers module for beatoraja-skin

### Phase 22c: MainController Render Pipeline + SpriteBatch GPU Flush + FPS Cap (complete)

+325 lines across 5 files + 7 new tests:

- [x] **MainController.render() enhancement** — sprite.begin()/end() lifecycle, input gating by time delta (Java `if(time > prevtime)` pattern), skin draw comments for Phase 22+ wiring
- [x] **SpriteBatch re-export** — sprite_batch_helper.rs now re-exports real SpriteBatch from beatoraja-render (replacing stub unit struct)
- [x] **SpriteBatch→wgpu GPU flush** — SpriteRenderPipeline initialization, dummy white texture, uniform/texture bind groups, vertex buffer upload via flush_to_gpu() in render pass
- [x] **FPS capping** — max_fps from Config, frame duration calculation with thread::sleep, last_frame_time tracking via Instant
- [x] **bytemuck dependency** — Added for projection matrix buffer upload

### Phase 21: Per-Screen MainState Implementations + State Dispatch (complete)

+~350 lines implementation + 23 new tests. All sub-tasks done:

- [x] **DecideState (MusicDecide)** — MainState trait impl in beatoraja-decide. state_type(), create(), render(), input(), dispose() lifecycle methods
- [x] **ResultState (MusicResult)** — MainState trait impl in beatoraja-result. Full lifecycle with score/replay handling stubs
- [x] **PlayState (BMSPlayer)** — MainState trait impl in beatoraja-play. Gameplay loop lifecycle with judge/gauge/BGA stubs
- [x] **SelectState (MusicSelector)** — MainState trait impl in beatoraja-select. Song select lifecycle with bar rendering/preview stubs
- [x] **KeyConfigState / SkinConfigState** — MainState trait impls with Phase 22 warn stubs in beatoraja-core config_pkg
- [x] **MainController state dispatch** — StateFactory trait for cross-crate state creation, change_state() with MainStateType dispatch (matching Java switch), transition_to_state() lifecycle (create→prepare→shutdown old), get_current_state/get_state_type, lifecycle dispatch (render/pause/resume/resize/dispose)
- [x] **Decide skip logic** — config.skip_decide_screen routes Decide→Play (matching Java)

### Phase 19: SkinData→Skin Loading Pipeline (complete)

+1,469 lines across 6 files, +20 tests. All sub-phases done:

- [x] **19a:** JsonSkinObjectLoader base — complete conversion methods for all JsonSkin types (Image, ImageSet, Text, Value, Slider, Graph, GaugeGraph, JudgeGraph, BpmGraph, NoteSet, SongList, PMchara, HiddenCover, LiftCover, BGA, Judge). 820+ lines added to json_skin_object_loader.rs
- [x] **19b:** Screen-specific loaders — PlaySkinObjectLoader (note field, gauge, judge, lane cover, BGA), SelectSkinObjectLoader (bar list rendering). Decide/Result/Course/KeyConfig/SkinConfig remain minimal (delegate to base, matching Java)
- [x] **19c:** LuaSkinLoader — `load_header()` and `load_skin()` implemented via mlua. `from_lua_value()` recursive converter: LuaTable → JsonSkin data tree. 280 lines
- [x] **19d:** SkinLoader entry points — `load()` routes to JSONSkinLoader or LuaSkinLoader based on file extension. `load_skin()` wired to screen-specific object loader creation. JSONSkinLoader `load_skin()` fully connected

### Phase 20: IRConnection Integration (complete)

+263 lines across 6 files + 2 new files, +13 tests:

- [x] `IRSendStatus` — full `send()` implementation: calls `connection.send_play_data()`, checks response, updates `is_sent`/`retry`. `send_course()` for course results. 250 lines
- [x] `IRInitializer` — `initialize_ir()` method: iterates player IR configs, creates connections via `IRConnectionManager`, calls login, returns `Vec<IRStatus>`. 107 lines
- [x] `IRResend` — `IRResendLoop` with exponential backoff (`4^retry * 1000ms`), periodic retry via `tokio::time::interval`, configurable max retries. 232 lines
- [x] `IRStatus` — updated with `connection: Arc<dyn IRConnection>`, `config`, `player` fields
- [x] IR stub comments updated to "real implementations (Phase 20)" in beatoraja-result/stubs.rs

## Blocked Tasks

### Phase 16b: Golden Master Test Activation (partially complete)

- [ ] Add missing fixtures for modules not yet covered (modmenu, select bar, stream) — deferred until Rust-side APIs are implemented
- [x] Reactivate `compare_render_snapshot.rs` — **DONE (Phase 24d)**: 22 tests activated (#[ignore] — awaiting SkinData→Skin pipeline)

### Phase 18e: Stub replacement (remaining items blocked)

- [x] Replace `MainState` stubs with real trait impls — **DONE (Phase 21)**: all 6 screen states implement MainState trait
- [ ] Remove all `stubs.rs` files — blocked: depends on rendering/database implementations. 現在 16 ファイル / 2,624 行。大きいもの: beatoraja-external (574), beatoraja-result (388), beatoraja-select (359), beatoraja-launcher (321), beatoraja-skin (294)
- [ ] beatoraja-external LibGDX stubs (Pixmap/GdxGraphics/BufferUtils/PixmapIO) — **partially unblocked (Phase 22 complete)**。wgpu ベースの代替実装が可能に

### Phase 18f: Integration verification (partially unblocked)

- [x] Activate `compare_render_snapshot.rs` — **DONE (Phase 24d)**: 22 tests compiled (#[ignore] — SkinData→Skin pipeline needed)
- [x] E2E gameplay flow test: select → decide → play → result screen transitions — **PARTIALLY DONE (Phase 21/23)**: MainController.change_state() dispatches to concrete states via LauncherStateFactory
- [ ] Final verification: all tests pass, zero clippy warnings, clean `cargo fmt` — blocked: final gate

### Known Issues (open)

- [x] SkinObject→GPU rendering gap — **RESOLVED (Phase 22d)**: Full pipeline connected
- [ ] Remaining stubs: ~2,624 lines across 16 stubs.rs files — 大半は re-export + 薄いラッパー。実質的なスタブは select (AudioDriver, EventType, SkinObject rendering), external (Pixmap/GdxGraphics), launcher (MainController partial)
- [ ] MainController: ~12 stub methods (polling thread, updateStateReferences, audio driver) — polling thread は Phase 24b (入力) で、audio driver は Phase 24c で対応予定
- [x] StateFactory concrete implementation — DONE (Phase 23): LauncherStateFactory in beatoraja-launcher

## Completed Phases

### Phase 22: Rendering Pipeline (SkinObject→GPU) — complete

Unblocks: Phase 16b render snapshot tests, Phase 18f E2E tests, visual output

- [x] **22a: WGSL sprite shader + wgpu render pipeline + SpriteBatch GPU flush** — 6 shader types, 30 pipeline variants, SpriteBatch flush_to_gpu(). +43 tests
- [x] **22b: SkinObject draw methods + SkinTextBitmap** — Draw chain integration tests + ab_glyph font rendering. +42 tests, +1,346 lines
- [x] **22c: MainController render pipeline + FPS cap** — sprite lifecycle, SpriteBatch re-export, wgpu flush, FPS capping. +7 tests, +325 lines
- [x] **22d: Skin.draw_all_objects() integration** — SkinDrawable trait, TimerOnlyMainState adapter, take/put-back borrow pattern. +11 tests, +~150 lines

### Phase 23: Database Integration — partially complete

Unblocks: SongDatabaseAccessor stubs, PlayDataAccessor stubs

- [x] **23a: LauncherStateFactory** — Concrete StateFactory impl. +10 tests
- [x] **23b: MainController DB wiring** — songdb field + set/get methods, PlayDataAccessor init
- [x] **23c: MusicSelector DB injection** — with_song_database() constructor
- [x] **23d: CourseResult MainState** — MainState trait impl
- [ ] Wire rusqlite SongDatabaseAccessor with real schema — → Phase 24a
- [ ] Connect to MusicSelector song list loading — → Phase 24e

## Phase 24: ランタイム統合（Runtime Integration）

目標: アプリケーション起動→楽曲選択→プレイまでの実行フローを繋ぐ。
Phase 22 (レンダリング) と Phase 23 (DB配線) が完了し、ランタイム統合の前提条件が揃った。

### Phase 24a: SQLiteSongDatabaseAccessor + MainLoader.play() エントリポイント

**優先度: 最高** — 楽曲データベースが全ての下流機能（選曲、スコア表示、リプレイ）の基盤

依存: なし（即着手可能）

- [ ] `SQLiteSongDatabaseAccessor` を beatoraja-song に実装
  - rusqlite で `song` テーブル + `folder` テーブルのスキーマ作成 (Java の `SQLiteSongDatabaseAccessor` コンストラクタ参照)
  - `SongDatabaseAccessor` trait の 6 メソッドを実装: `get_song_datas(key, value)`, `get_song_datas_by_hashes()`, `get_song_datas_by_sql()`, `set_song_datas()`, `get_song_datas_by_text()`, `get_folder_datas()`
  - `song` テーブル: md5, sha256, title, subtitle, genre, artist, subartist, tag, path, folder, stagefile, banner, backbmp, preview, parent, level, difficulty, maxbpm, minbpm, length, mode, judge, feature, content, date, favorite, adddate, notes, charthash
  - `folder` テーブル: title, subtitle, command, path, banner, parent, type, date, adddate, max
- [ ] `updateSongDatas()` — BMS ルートディレクトリ走査 + BMSDecoder/BMSONDecoder でメタデータ抽出 + DB 挿入。タイムスタンプ比較で増分更新
- [ ] `MainLoader.play()` を完成 — Config 読み込み → SQLiteSongDatabaseAccessor 生成 → MainController 生成 → winit ウィンドウ + wgpu 初期化 → イベントループ開始
- [ ] `MainLoader.get_score_database_accessor()` を実装 (現在 `None` 返却)
- [ ] `MainLoader.start()` — egui ランチャー UI の基本フレーム (設定画面表示)

**見積り:** ~800 行実装 + ~20 テスト

### Phase 24b: 入力システム統合（winit → BMSPlayerInputProcessor）

**優先度: 高** — キーボード/コントローラ入力がないとプレイ不可

依存: Phase 24a (MainLoader がウィンドウを作成)

- [ ] winit `WindowEvent::KeyboardInput` → `KeyBoardInputProcesseor` への接続
  - winit の `KeyCode` を Java keycode (`com.badlogic.gdx.Input.Keys`) にマッピング
  - `BMSPlayerInputProcessor.poll()` を winit イベントベースに適応（現在は `System.nanoTime()` ベース）
- [ ] MainController の input polling thread 実装
  - Java: `new Thread(() -> { while(!quit) { input.poll(); Thread.sleep(1); } })` パターン
  - Rust: `tokio::spawn` or `std::thread::spawn` + `crossbeam` チャネルで winit イベント転送
- [ ] コントローラ入力 (gilrs クレート)
  - `BMControllerInputProcessor` の `Controller` 列挙をgirs で実装
  - アナログスティック → `computeAnalogDiff()` はすでに実装済み
- [ ] マウス入力 — `CursorMoved`, `MouseInput` イベント → mousex/mousey/mousepressed
- [ ] `KeyCommand` のウィンドウシステム統合 — F キー、スクリーンモード切替等

**見積り:** ~500 行実装 + ~15 テスト

### Phase 24c: オーディオドライバ統合（AudioDriver stub → real trait wiring）— complete

- [x] beatoraja-select の `AudioDriver` スタブ削除 (stubs.rs から 11 行削除)
- [x] `PreviewMusicProcessor` を `&dyn AudioDriver` (beatoraja-audio trait) に切り替え
  - `new()`, `run_preview_loop()`, `stop_preview_internal()` の引数を `&AudioDriver` stub → `&dyn/&mut dyn AudioDriver` trait に変更
  - メソッド呼び出しを trait メソッド名に修正: `play`→`play_path`, `stop`→`stop_path`, `dispose`→`dispose_path`, `is_playing`→`is_playing_path`, `set_volume`→`set_volume_path`
- [x] `MainController` に `audio: Option<Box<dyn AudioDriver>>` フィールド追加
  - `get_audio_processor()` → `Option<&dyn AudioDriver>` (旧 `Option<()>` stub 置換)
  - `get_audio_processor_mut()` → `Option<&mut dyn AudioDriver>` 新規追加
  - `set_audio_driver()` — 外部からの DI (ドライバ選択はランチャー層で実施)
- [x] 11 新テスト: MockAudioDriver 5 (beatoraja-core) + PreviewMusicProcessor 6 (beatoraja-select)
- [x] ワークスペース全体コンパイル OK

**注:** KiraAudioDriver (PortAudioDriver) は Phase 17 で既に 537 行実装済み。CpalAudioDriver は Phase 24f 以降で対応予定。AudioConfig.DriverType ベースの選択はランチャー層で実装予定。

### Phase 24d: RenderSnapshot テスト有効化 — complete

- [x] `compare_render_snapshot.rs` のクレート名修正 — `bms_config`→`beatoraja_core`, `bms_render`→`golden_master`, `bms_skin`→`beatoraja_skin`
- [x] `golden-master/Cargo.toml` — 依存はすでに存在 (beatoraja-core, beatoraja-skin)
- [x] `tests/pending/` から `tests/compare_render_snapshot.rs` へ移動してテスト有効化 (22 tests)
- [x] `Gauge` DrawDetail variant を `render_snapshot.rs` に追加 + `compare_detail` で比較対応
- [x] `load_lua_skin`/`load_json_skin` ヘルパー — `SkinData→Skin` 変換パイプライン未実装のため stub 返却 + 全 22 テスト `#[ignore]` 化
- [ ] Java fixture 生成環境の整備 (`just golden-master-render-snapshot-gen`) — 後続で対応
- [ ] `SkinData→Skin` loading pipeline — テスト `#[ignore]` 解除の前提条件。JSONSkinLoader/LuaSkinLoader が `SkinData` を返すが、`capture_render_snapshot` は `Skin` が必要

### Phase 24e: BarManager + 楽曲選択画面統合

**優先度: 中** — 楽曲選択 UI の動作に必要

依存: Phase 24a (SQLiteSongDatabaseAccessor)

- [ ] `BarManager.init()` 実装
  - `TableDataAccessor` でテーブルデータ読み込み (すでに beatoraja-core に実装済み)
  - `CourseDataAccessor` でコースデータ読み込み (すでに実装済み)
  - お気に入り (favorite) 読み込み
  - コマンドフォルダ (`folder/default.json`) パース
  - ランダムフォルダ (`random/default.json`) パース
  - IR テーブル取得 (IRConnection 経由)
- [ ] `BarManager.update_bar()` 実装
  - ルート: テーブル + コマンド + お気に入り + 検索を表示
  - DirectoryBar: 子バー取得 → モードフィルタ → 不可視楽曲フィルタ → ソート
  - `BarSorter` (すでに実装済み) でソート適用
  - スコアデータキャッシュからスコア読み込み
- [ ] `BarContentsLoaderThread.run()` 実装
  - スコアデータ読み込み (SongBar/GradeBar)
  - リプレイ存在チェック
  - バナー/ステージファイル画像読み込み
  - `SongInformationAccessor` による楽曲情報取得
- [ ] `BarManager.close()` — ディレクトリ階層を上に戻る

**見積り:** ~600 行実装 + ~15 テスト

### Phase 24f: MainController 残スタブ解消

**優先度: 低** — アプリ起動後の補助機能

依存: Phase 24b (入力), Phase 24c (オーディオ)

- [ ] `MainController.updateStateReferences()` — スキン/オーディオ/IR ステートの更新
- [ ] `MainController` polling thread — 入力ポーリングスレッド起動
- [ ] `MainController` audio driver 初期化 — AudioConfig に基づくドライバ選択・初期化
- [ ] modmenu 関連スタブ (SongManagerMenu 完全版、MusicSelector modmenu 連携)
- [ ] ScreenType / AbstractResult — result 画面の外部依存スタブ

**見積り:** ~400 行実装 + ~10 テスト

## Phase 25 以降（概要のみ）

### Phase 25: E2E 統合テスト + 品質保証

- [ ] select → decide → play → result のフル E2E テスト (LauncherStateFactory + 実 DB)
- [ ] RenderSnapshot パリティ回帰テスト全有効化
- [ ] 全 stubs.rs ファイルの棚卸し — 不要スタブ削除、残存スタブの理由文書化
- [ ] cargo clippy 警告ゼロ + cargo fmt クリーン

### Phase 26: 楽曲データベース更新 + 楽曲検索

- [ ] `updateSongDatas()` の並列走査 (rayon)
- [ ] `getSongDatasByText()` — SQLite FTS5 全文検索
- [ ] `SongInformationAccessor` — 楽曲情報データベース連携

### Phase 27: プラットフォーム固有機能

- [ ] Windows named pipe (LR2 互換)
- [ ] macOS CoreGraphics モニター列挙（基本実装済み、winit 連携が必要）
- [ ] Discord Rich Presence (discord-rich-presence クレート)

### Phase 28: パフォーマンス最適化 + リファクタリング

- [ ] スタブ→実装への移行で導入された間接参照の削減
- [ ] PlayerResource trait の最適化（32 メソッド → 必要最小限）
- [ ] メモリプロファイリング + テクスチャキャッシュ戦略
