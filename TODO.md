# Porting TODO — Remaining Work

All phases (1–24e) complete. **1651 tests pass, 22 ignored.** See AGENTS.md for full status.

## Phase 24: ランタイム統合（Runtime Integration）— 進行中

目標: アプリケーション起動→楽曲選択→プレイまでの実行フローを繋ぐ。

### Phase 24a: SQLiteSongDatabaseAccessor + MainLoader — complete

+710 行, +23 テスト。SQLiteSongDatabaseAccessor 6/6 メソッド、updateSongDatas() BMS走査、MainLoader.play()/start()、LauncherUi eframe impl。

### Phase 24b: 入力システム統合（winit → BMSPlayerInputProcessor）— complete

+~300 行, +46 テスト。WinitKeyCode→Java keycode マッピング、SharedKeyState (Arc<Mutex>)、GdxInput/GdxGraphics 実装、MainController input 統合 (render() 内 poll())、マウスイベント連携。

残タスク (Phase 24b スコープ外):
- [ ] gilrs controller 統合 (BMControllerInputProcessor) — Phase 27 で対応
- [ ] チャネルベースの非同期 polling thread — Phase 24f で対応
- [ ] KeyCommand のウィンドウシステム統合 (F キーコマンド) — Phase 24f で対応

### Phase 24c: オーディオドライバ統合 — complete

+11 テスト。AudioDriver stub 削除、PreviewMusicProcessor → `&dyn AudioDriver`、MainController audio フィールド追加。

### Phase 24d: RenderSnapshot テスト有効化 — complete

22 テスト有効化 (#[ignore] — SkinData→Skin パイプライン待ち)。

残タスク:
- [ ] Java fixture 生成環境の整備 (`just golden-master-render-snapshot-gen`)
- [ ] `SkinData→Skin` loading pipeline — テスト `#[ignore]` 解除の前提条件

### Phase 24e: BarManager + 楽曲選択画面統合 — complete

+~1600 行, +40 テスト (beatoraja-select 87 テスト全合格)。BarManager.init() (テーブル/コース/お気に入り/コマンド/ランダムフォルダ)、update_bar()/update_bar_with_context() (モードフィルタ、非表示フィルタ、BarSorter、カーソル復元)、BarContentsLoaderThread.run() (スコア読み込み、ライバルスコア)、BarManager.close()。UpdateBarContext/LoaderContext/CourseTableAccessor 追加。

残タスク:
- [ ] バナー/ステージファイル実画像読み込み — PixmapResourcePool 実装待ち
- [ ] リプレイ存在チェック — ReplayData API 統合待ち

### Phase 24f: MainController 残スタブ解消

**優先度: 高** — Phase 24b/24c 完了によりブロック解除

依存: Phase 24b (入力) ✅, Phase 24c (オーディオ) ✅

- [ ] `MainController.updateStateReferences()` — スキン/オーディオ/IR ステートの更新
- [ ] `MainController` polling thread — 入力ポーリングスレッド起動
- [ ] `MainController` audio driver 初期化 — AudioConfig に基づくドライバ選択・初期化
- [ ] modmenu 関連スタブ (SongManagerMenu 完全版、MusicSelector modmenu 連携)
- [ ] ScreenType / AbstractResult — result 画面の外部依存スタブ

**見積り:** ~400 行実装 + ~10 テスト

## Phase 25: スタブ棚卸し + E2E 統合テスト + 品質保証

依存: Phase 24 全完了

### 25a: スタブ棚卸しと分類

現在 16 stubs.rs ファイル / 3,004 行 (テスト 244 行含む)。実態は3種類:

1. **re-export のみ (削除不要、整理のみ):** beatoraja-core (1行), beatoraja-audio (1行), beatoraja-play (9行), beatoraja-ir (10行), beatoraja-stream (4行), beatoraja-obs (9行), md-processor (12行) — 計 46 行
2. **実装済みコード (stubs.rs にあるが実はスタブではない):** beatoraja-external の Pixmap/GdxGraphics/BufferUtils/PixmapIO (image crate ベースの実装 ~170行 + テスト ~244行), ClipboardHelper (arboard 実装 ~25行), beatoraja-launcher の CoreGraphics FFI/cpal/rfd 実装 (~200行), beatoraja-input の GdxInput/GdxGraphics/Keys (~150行 SharedKeyState ベース)
3. **真のスタブ (実装が必要):**
   - beatoraja-external: ScoreDatabaseAccessor, MainState struct, ScreenType, AbstractResult, Property traits/factories, Twitter4j — ~290行
   - beatoraja-result: MainController (10メソッド), PlayerResource wrapper (35メソッド), RankingDataCache, AudioProcessorStub, SkinObjectData — ~290行
   - beatoraja-select: MainState trait, EventType enum, SkinText/SkinNumber/SkinImage/SkinObject/SkinObjectRenderer rendering stubs, SongManagerMenu, DownloadTask — ~185行
   - beatoraja-skin: MainState/MainController/Timer/Resolution/SkinOffset stubs, BMSPlayer/JudgeManager/MusicResult/PlayerResource stubs, PlaySkinStub — ~280行
   - beatoraja-types: JudgeAlgorithm/BMSPlayerRule, BarSorter, modifier stubs, IRConnectionManager, KeyInputLog/PatternModifyLog/bms_player_input_device — ~205行
   - beatoraja-modmenu: MainController (3メソッド), MainState trait, Skin/SkinObject stubs, MusicSelector/Bar/SongBar, Rectangle — ~110行
   - beatoraja-decide: MainControllerRef (3メソッド), AudioProcessorStub, SkinStub, load_skin, play_sound — ~85行
   - beatoraja-launcher: MainLoader (display stubs), VersionChecker, SongDatabaseUpdateListener, TwitterAuth — ~75行

サブタスク:
- [ ] 実装済みコードを stubs.rs から適切なモジュールに移動 (~590行)
- [ ] re-export のみの stubs.rs を整理 (mod.rs の `pub use` に統合、7ファイル)
- [ ] 真のスタブの一覧と解消ロードマップを文書化
- [ ] Twitter4j スタブ → `bail!()` のまま永久保持 (130行, 対応不要)

**見積り:** ~200 行変更 (移動中心) + ~5 テスト

### 25b: E2E 統合テスト

- [ ] select → decide → play → result のフル E2E テスト
  - LauncherStateFactory + 実 SQLiteSongDatabaseAccessor を使用
  - テスト用 BMS ファイルは test-bms/ を使用
  - 各画面遷移で MainState lifecycle (create/prepare/render/shutdown) が正しく呼ばれることを検証
- [ ] RenderSnapshot パリティテスト有効化
  - SkinData→Skin loading pipeline の完成が前提
  - 22 テストの `#[ignore]` 解除
- [ ] BarManager 統合テスト
  - SQLiteSongDatabaseAccessor + テスト DB で楽曲読み込み→バー表示
  - BarSorter でソート検証

**見積り:** ~400 行テスト + ~100 行インフラ

### 25c: 品質保証

- [ ] cargo clippy 警告ゼロ (--workspace -- -D warnings)
- [ ] cargo fmt クリーン
- [ ] 全テスト pass 確認 (ignored テストの理由を文書化)
- [ ] 未使用 import / dead code 警告の解消

**見積り:** ~50 行修正

## Phase 26: 楽曲データベース拡張 + 楽曲検索

依存: Phase 24e (BarManager), Phase 25b (E2E テスト基盤)

### 26a: updateSongDatas() の並列走査 (rayon)

- [ ] `rayon::par_iter()` で BMS ファイル走査を並列化
  - 現在: 逐次ファイル走査 → BMSDecoder → DB 書き込み
  - 目標: ファイル発見 → 並列デコード → バッチ DB 書き込み
- [ ] スレッドセーフな進捗コールバック (`AtomicUsize` カウンタ)
- [ ] 並列走査の golden master テスト (逐次と同一結果を保証)

**見積り:** ~150 行実装 + ~10 テスト

### 26b: getSongDatasByText() — SQLite FTS5 全文検索

- [ ] FTS5 仮想テーブル作成 (`song_fts` テーブル、title/subtitle/artist/genre カラム)
- [ ] `updateSongDatas()` で FTS5 テーブルも更新
- [ ] `get_song_datas_by_text()` — FTS5 `MATCH` クエリで検索
- [ ] BarManager の検索フォルダで FTS5 検索を呼び出し

**見積り:** ~120 行実装 + ~8 テスト

### 26c: SongInformationAccessor — 楽曲情報データベース連携

- [ ] `SongInformationAccessor` trait の実装 (beatoraja-song に定義済み)
- [ ] SQLite テーブル作成 + CRUD
- [ ] BarContentsLoaderThread からの呼び出し統合

**見積り:** ~100 行実装 + ~6 テスト

## Phase 27: プラットフォーム固有機能

依存: Phase 24b (入力), Phase 24f (MainController)

### 27a: Windows named pipe (LR2 互換)

- [ ] `\\.\pipe\lr2oraja` named pipe サーバ実装
  - tokio の `named_pipe::ServerOptions` を使用
  - LR2 クライアントからの接続受付 + コマンドパース
- [ ] LR2 プロトコル互換メッセージ処理
- [ ] `#[cfg(target_os = "windows")]` で条件コンパイル

**見積り:** ~200 行実装 + ~8 テスト (Windows CI でのみ検証)

### 27b: macOS CoreGraphics モニター列挙 (winit 連携)

- [ ] beatoraja-launcher の `get_monitors_macos()` と winit イベントループの連携
  - 現在: CoreGraphics FFI で直接列挙 (実装済み)
  - 目標: winit の `available_monitors()` からも補完 (name が "Display N" だけなので CG で情報補強)
- [ ] ランチャー UI でモニター選択 → Config に保存

**見積り:** ~80 行実装 + ~4 テスト

### 27c: Discord Rich Presence

- [ ] `discord-rich-presence` クレート統合
  - discord_rpc crate はすでに存在 (Phase 17 で作成)
  - `DiscordRpcClient` の接続/切断ライフサイクル
- [ ] 画面状態に応じたプレゼンス更新
  - Select: "Selecting a song"
  - Play: "Playing [title] [artist]"
  - Result: "Viewing results"
- [ ] MainController から RPC クライアントへの状態通知

**見積り:** ~150 行実装 + ~6 テスト

## Phase 28: パフォーマンス最適化 + リファクタリング

依存: Phase 25c (品質保証ベースライン)

### 28a: 間接参照の削減

- [ ] PlayerResource: trait 呼び出し → 直接フィールドアクセス
  - 各 crate の PlayerResource wrapper struct を統合
  - `Box<dyn PlayerResourceAccess>` → 具象型 (全 crate で同一型を使用)
- [ ] MainController スタブ → 実体参照への置換
  - beatoraja-result/modmenu/decide の MainController stubs を削除
  - `&MainController` (beatoraja-core) を直接渡す

**見積り:** ~300 行変更 + ~10 テスト

### 28b: PlayerResource trait の最適化

- [ ] 32 メソッド → 必要最小限に絞り込み
  - 使用頻度分析 (grep で各メソッドの呼び出し回数を集計)
  - 未使用メソッドの削除
  - 類似メソッドの統合 (get_gauge/get_groove_gauge 等)

**見積り:** ~150 行変更 + ~5 テスト

### 28c: メモリプロファイリング + テクスチャキャッシュ

- [ ] `dhat` / `jemalloc_ctl` でメモリプロファイリング
- [ ] テクスチャキャッシュ戦略の実装
  - LRU キャッシュ (wgpu Texture の再利用)
  - 未使用テクスチャの解放タイミング
- [ ] SpriteBatch のバッチ効率測定 + 最適化

**見積り:** ~200 行実装 + ~8 テスト

## Blocked Tasks

### Phase 16b: Golden Master Test Activation (partially complete)

- [ ] 未カバーモジュールの fixture 追加 (modmenu, select bar, stream) — Rust 側 API 完成待ち
- [x] `compare_render_snapshot.rs` 再有効化 — **DONE (Phase 24d)**

### Phase 18e: Stub replacement (部分的に残存)

- [x] `MainState` スタブ → real trait — **DONE (Phase 21)**
- [ ] 全 stubs.rs 削除 → **Phase 25a** で対応
- [ ] beatoraja-external LibGDX stubs — wgpu 代替可能だが **Phase 25a** で整理

### Phase 18f: Integration verification

- [x] `compare_render_snapshot.rs` 有効化 — **DONE (Phase 24d)**
- [x] E2E gameplay flow — **PARTIALLY DONE (Phase 21/23)**
- [ ] Final verification — **Phase 25c** で対応

### Known Issues (open)

- [ ] Remaining stubs: 3,004 行 across 16 files — 実質的な真のスタブは ~1,520 行。残りは re-export (~140行), 実装済みコード (~1,100行), テスト (~244行)
- [ ] MainController: 5 stub methods in main_controller.rs (application exit, SongUpdateThread, updateTable, downloadIpfs) — Phase 24f で対応
- [ ] 22 ignored tests (RenderSnapshot) — SkinData→Skin pipeline 完成待ち
- **Intentional:** Twitter4j → `bail!()` (永久、~130行)

## Completed Phases (summary)

| Phase | Summary | Tests |
|-------|---------|-------|
| 1–17 | Core translation (17 crates, 300+ modules), real impls (wgpu, Kira, mlua, ffmpeg, midir, cpal, egui) | 868 |
| 18a–d | Core judge loop, rendering state, audio decode, BGA/skin test APIs | — |
| 18e (1–12) | Stub replacement (12 sub-phases), PlayerResource wrapper, 4 rounds audit | — |
| 18f | E2E test activation (138 tests across 9 files) | 138 |
| 18g | BRD replay codec | — |
| 19 | SkinData→Skin Loading Pipeline (JsonSkinObjectLoader, LuaSkinLoader, SkinLoader) | +20 |
| 20 | IRConnection Integration (IRSendStatus, IRInitializer, IRResendLoop) | +13 |
| 21 | Per-Screen MainState + State Dispatch (6 states, StateFactory, change_state) | +23 |
| 22a | WGSL Sprite Shader + Render Pipeline (6 shaders, 30 pipeline variants) | +43 |
| 22b | SkinObject Draw + SkinTextBitmap (ab_glyph font rendering) | +42 |
| 22c | MainController Render Pipeline + FPS Cap | +7 |
| 22d | Skin.draw_all_objects() (SkinDrawable trait, take/put-back pattern) | +11 |
| 23a–d | LauncherStateFactory + DB wiring (7 state types, songdb field, CourseResult) | +10 |
| 24a | SQLiteSongDatabaseAccessor + MainLoader (6/6 methods, updateSongDatas, LauncherUi) | +23 |
| 24c | Audio driver wiring (AudioDriver stub deleted, MainController audio field) | +11 |
| 24b | Input system integration (winit→Java keycode, SharedKeyState, MainController input) | +46 |
| 24d | RenderSnapshot test activation (22 tests compiled, #[ignore]) | +22 |
| 24e | BarManager + music selection (init/update_bar/close, BarContentsLoaderThread) | +40 |
