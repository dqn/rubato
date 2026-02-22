# Porting TODO — Remaining Work

All phases (1–25c, 25d-1, 25d-3) complete. **1730 tests pass, 22 ignored.** See AGENTS.md for full status.

## Phase 24: ランタイム統合（Runtime Integration）— complete

目標: アプリケーション起動→楽曲選択→プレイまでの実行フローを繋ぐ。

### Phase 24a: SQLiteSongDatabaseAccessor + MainLoader — complete

+710 行, +23 テスト。SQLiteSongDatabaseAccessor 6/6 メソッド、updateSongDatas() BMS走査、MainLoader.play()/start()、LauncherUi eframe impl。

### Phase 24b: 入力システム統合（winit → BMSPlayerInputProcessor）— complete

+~300 行, +46 テスト。WinitKeyCode→Java keycode マッピング、SharedKeyState (Arc<Mutex>)、GdxInput/GdxGraphics 実装、MainController input 統合 (render() 内 poll())、マウスイベント連携。

残タスク (deferred):
- [ ] gilrs controller 統合 (BMControllerInputProcessor) → **Phase 28a**
- [ ] KeyCommand のウィンドウシステム統合 (F キーコマンド) → **Phase 28b**
- [ ] チャネルベースの非同期 polling thread → **Phase 29d**

### Phase 24c: オーディオドライバ統合 — complete

+11 テスト。AudioDriver stub 削除、PreviewMusicProcessor → `&dyn AudioDriver`、MainController audio フィールド追加。

### Phase 24d: RenderSnapshot テスト有効化 — complete

22 テスト有効化 (#[ignore] — SkinData→Skin パイプライン待ち)。

残タスク (deferred):
- [ ] Java fixture 生成環境の整備 → **Phase 26c**
- [ ] `SkinData→Skin` loading pipeline → **Phase 26b**

### Phase 24e: BarManager + 楽曲選択画面統合 — complete

+~1600 行, +40 テスト (beatoraja-select 87 テスト全合格)。BarManager.init() (テーブル/コース/お気に入り/コマンド/ランダムフォルダ)、update_bar()/update_bar_with_context() (モードフィルタ、非表示フィルタ、BarSorter、カーソル復元)、BarContentsLoaderThread.run() (スコア読み込み、ライバルスコア)、BarManager.close()。UpdateBarContext/LoaderContext/CourseTableAccessor 追加。

残タスク (deferred):
- [ ] バナー/ステージファイル実画像読み込み → **Phase 26d**
- [ ] リプレイ存在チェック → **Phase 26d**

### Phase 24f: MainController 残スタブ解消 — complete

+~200 行, +10 テスト。update_main_state_listener() 実ディスパッチ、update_state_references() StateReferencesCallback trait、periodic_config_save() Java 準拠 (120s/BMSPlayer スキップ)、create() 完全配線、add_state_listener()。

残タスク (deferred):
- [ ] Polling thread のチャネルベース分離 → **Phase 29d** (nice-to-have)
- Audio driver 生成 — ランチャー層で set_audio_driver() 経由で注入済み ✅

## Phase 25: スタブ棚卸し + E2E 統合テスト + 品質保証

依存: Phase 24 全完了

### 25a: スタブ棚卸しと分類 — complete

6 re-export-only stubs.rs 削除 (beatoraja-audio, play, ir, obs, stream, md-processor)。3 大 stubs.rs 再編成: beatoraja-input (→gdx_compat.rs, keys.rs)、beatoraja-external (→pixmap_io.rs, clipboard_helper.rs)、beatoraja-launcher (→platform.rs)。残存 stubs.rs は 10 ファイル、真のスタブ ~1,520 行のみ。

残存する真のスタブ (解消ロードマップ):
- beatoraja-external (~290行): ScoreDatabaseAccessor, MainState, ScreenType, AbstractResult, Property traits, Twitter4j (永久)
- beatoraja-result (~290行): MainController (10), PlayerResource wrapper (35), RankingDataCache
- beatoraja-select (~185行): EventType, SkinObject rendering, SongManagerMenu, DownloadTask
- beatoraja-skin (~280行): MainState/MainController/Timer stubs, BMSPlayer/JudgeManager stubs
- beatoraja-types (~205行): JudgeAlgorithm, BarSorter, modifier stubs, IRConnectionManager
- beatoraja-modmenu (~110行): MainController, Skin/SkinObject, MusicSelector/Bar stubs
- beatoraja-decide (~85行): MainControllerRef, AudioProcessor, SkinStub
- beatoraja-launcher (~75行): MainLoader display, VersionChecker, TwitterAuth (永久)

### 25b: E2E 統合テスト — complete

+32 テスト。画面遷移 E2E 12テスト (Select→Decide→Play→Result チェイン、lifecycle 検証、skip_decide、ストレステスト)。BarManager 統合 20テスト (init/update_bar/close、ソート、ナビゲーション、検索)。RenderSnapshot は Phase 26c で対応。

### 25c: 品質保証 — complete

clippy 警告ゼロ (24ファイル修正)、cargo fmt クリーン、#[allow(clippy::field_reassign_with_default)] を 14 テストモジュールに追加 (Java 機械翻訳パターン保持)。22 ignored テスト文書化済み。

### 25d: 残存スタブ解消

依存: Phase 25a ✅, Phase 25c

真のスタブ ~1,520 行を3カテゴリに分けて段階的に解消する。

#### 25d-1: Cross-crate forwarding スタブの削除 — partially complete

4 新規 trait ファイルを beatoraja-types に追加 (ScreenType, ScoreDatabaseAccess, MainStateAccess, AbstractResultAccess)。beatoraja-external (~80行削除)、beatoraja-modmenu (~15行削除) のスタブを trait 参照に置換。

残存 (deeper type hierarchy 変更が必要):
- [ ] beatoraja-result MainController/PlayerResource wrapper — crate 固有型 (BMSPlayerInputProcessor, IRStatus 等) に依存
- [ ] beatoraja-decide MainControllerRef/SkinStub — BMSPlayerInputProcessor, AudioProcessorStub に依存
- [ ] beatoraja-modmenu Skin/SkinObject/MusicSelector/Bar stubs — レンダリング依存 → **Phase 25d-2** で対応
- [ ] beatoraja-external Property traits/factories — &MainState 型互換性 → **Phase 29a** で対応

#### 25d-2: レンダリング連携スタブの解消 (~465行)

スキン描画パイプラインの cross-crate 連携を完成させてスタブを置換。

- [ ] **beatoraja-skin の internal stubs** (~280行)
  - MainState/MainController/InputProcessor → beatoraja-types の trait を参照
  - Timer/Resolution/SkinOffset → beatoraja-types に移動
  - BMSPlayer/JudgeManager stubs → beatoraja-types に最小 trait を定義
  - MusicResult/PlayerResource stubs → beatoraja-types の trait 参照に置換
  - PlaySkinStub/SkinLoaderStub → 実装に置換 or 削除
- [ ] **beatoraja-select のレンダリングスタブ** (~185行)
  - EventType enum → beatoraja-types に定義
  - SkinText/SkinNumber/SkinImage/SkinObject → beatoraja-skin の実型を直接参照
  - SkinObjectRenderer → beatoraja-skin の実型を使用
  - SongManagerMenu → beatoraja-modmenu の実型を参照 (select→modmenu 依存追加)
  - DownloadTask → beatoraja-external の実型 or md-processor の実型を参照

**見積り:** ~350 行変更 + ~10 テスト

#### 25d-3: 型定義スタブの beatoraja-types 集約 — complete

beatoraja-types: JudgeAlgorithm (Score variant 追加)、BMSPlayerRule (7 variants)、BarSorter (12 variant enum)、modifier Mode 型 (Java 全バリアント)、PatternModifyLog (section/modify 構造)。beatoraja-launcher: SongDatabaseUpdateListener (AtomicI32)、VersionChecker (reqwest GitHub API)、DisplayMode (デフォルト値)。+29 テスト。

#### 永久保持 (対応不要)
- Twitter4j → `bail!()` (~155行, beatoraja-external + beatoraja-launcher) — サービス終了のため永久保持

**Phase 25d 合計見積り:** ~1,000 行変更 + ~35 テスト

## Phase 26: リソースローディング + スキンパイプライン完成

依存: Phase 22 (レンダリング) ✅, Phase 25d-2 (レンダリング連携スタブ解消)

目標: SkinData→Skin 変換パイプラインを完成させ、22 ignored テストを解除する。

### 26a: PixmapResourcePool (テクスチャリソース管理)

- [ ] `PixmapResourcePool` 実装 — wgpu テクスチャのロード/キャッシュ/解放
  - Java: `PixmapResourcePool.loadPixmap()` → Rust: image crate で読み込み → wgpu Texture 生成
  - テクスチャキャッシュ (パス→Texture のマップ)
  - 参照カウント or LRU で未使用テクスチャ解放
- [ ] `SkinSourceImage` — スキン画像リソースの実ロード
  - 現在: Phase 22 で SkinImage.draw() は実装済みだが、テクスチャロードがスタブ
  - JSON/Lua スキンの image パスを解決してテクスチャを生成

**見積り:** ~250 行実装 + ~10 テスト

### 26b: SkinData→Skin 変換パイプライン

- [ ] `SkinLoader.load_skin()` の完成
  - Phase 19 で JSONSkinLoader/LuaSkinLoader が `SkinData` を返す部分は実装済み
  - 未実装: `SkinData` → `Skin` (テクスチャロード + SkinObject インスタンス化)
  - `JsonSkinObjectLoader` の各メソッドでテクスチャを `PixmapResourcePool` 経由でロード
- [ ] スキンオブジェクトのテクスチャバインド
  - SkinImage/SkinNumber/SkinTextImage → TextureRegion の実テクスチャ参照

**見積り:** ~300 行実装 + ~12 テスト

### 26c: RenderSnapshot テスト解除 + Java fixture 生成

依存: Phase 26b

- [ ] Java fixture 生成環境の整備 (`just golden-master-render-snapshot-gen`)
  - Java 側でスキンを読み込み、レンダリングスナップショットを JSON に出力
  - 既存テスト用 BMS + スキンファイルから fixture 生成
- [ ] 22 テストの `#[ignore]` 解除
  - `load_lua_skin`/`load_json_skin` ヘルパーを実装に置換 (現在 stub)
  - golden master 比較テスト有効化
- [ ] 未カバーモジュール fixture 追加 (← Phase 16b deferred)
  - modmenu, select bar, stream の fixture

**見積り:** ~200 行 + ~22 テスト解除

### 26d: バナー/ステージファイル画像 + リプレイ API (← Phase 24e deferred)

依存: Phase 26a (PixmapResourcePool)

- [ ] バナー/ステージファイル実画像読み込み (BarContentsLoaderThread)
  - `PixmapResourcePool` 経由で画像ロード
  - サムネイル生成 (固定サイズへのリサイズ)
- [ ] リプレイ存在チェック
  - `ReplayData::exists()` API — ファイルパスベースの存在確認
  - BarContentsLoaderThread からの呼び出し統合

**見積り:** ~150 行実装 + ~8 テスト

## Phase 27: 楽曲データベース拡張 + 楽曲検索

依存: Phase 24e (BarManager) ✅

### 27a: updateSongDatas() の並列走査 (rayon)

- [ ] `rayon::par_iter()` で BMS ファイル走査を並列化
  - 現在: 逐次ファイル走査 → BMSDecoder → DB 書き込み
  - 目標: ファイル発見 → 並列デコード → バッチ DB 書き込み
- [ ] スレッドセーフな進捗コールバック (`AtomicUsize` カウンタ)
- [ ] 並列走査の golden master テスト (逐次と同一結果を保証)

**見積り:** ~150 行実装 + ~10 テスト

### 27b: getSongDatasByText() — SQLite FTS5 全文検索

- [ ] FTS5 仮想テーブル作成 (`song_fts` テーブル、title/subtitle/artist/genre カラム)
- [ ] `updateSongDatas()` で FTS5 テーブルも更新
- [ ] `get_song_datas_by_text()` — FTS5 `MATCH` クエリで検索
- [ ] BarManager の検索フォルダで FTS5 検索を呼び出し

**見積り:** ~120 行実装 + ~8 テスト

### 27c: SongInformationAccessor — 楽曲情報データベース連携

- [ ] `SongInformationAccessor` trait の実装 (beatoraja-song に定義済み)
- [ ] SQLite テーブル作成 + CRUD
- [ ] BarContentsLoaderThread からの呼び出し統合

**見積り:** ~100 行実装 + ~6 テスト

## Phase 28: プラットフォーム固有機能 + 入力拡張

依存: Phase 24b (入力) ✅, Phase 24f (MainController) ✅

### 28a: gilrs コントローラ統合 (← Phase 24b deferred)

- [ ] `gilrs` クレート依存追加
- [ ] `BMControllerInputProcessor` の `Controller` 列挙を gilrs で実装
- [ ] アナログスティック → `computeAnalogDiff()` はすでに実装済み、gilrs 入力と接続
- [ ] コントローラ hotplug 検出

**見積り:** ~200 行実装 + ~8 テスト

### 28b: KeyCommand ウィンドウシステム統合 (← Phase 24b deferred)

- [ ] F キーコマンド (F1-F12) のマッピング
- [ ] スクリーンモード切替 (Alt+Enter → winit fullscreen toggle)
- [ ] その他ウィンドウ操作 (ESC → 終了確認、etc.)

**見積り:** ~100 行実装 + ~5 テスト

### 28c: Windows named pipe (LR2 互換)

- [ ] `\\.\pipe\lr2oraja` named pipe サーバ実装
  - tokio の `named_pipe::ServerOptions` を使用
  - LR2 クライアントからの接続受付 + コマンドパース
- [ ] LR2 プロトコル互換メッセージ処理
- [ ] `#[cfg(target_os = "windows")]` で条件コンパイル

**見積り:** ~200 行実装 + ~8 テスト (Windows CI でのみ検証)

### 28d: macOS CoreGraphics モニター列挙 (winit 連携)

- [ ] beatoraja-launcher の `get_monitors_macos()` と winit イベントループの連携
  - 現在: CoreGraphics FFI で直接列挙 (実装済み)
  - 目標: winit の `available_monitors()` からも補完
- [ ] ランチャー UI でモニター選択 → Config に保存

**見積り:** ~80 行実装 + ~4 テスト

### 28e: Discord Rich Presence

- [ ] `discord-rich-presence` クレート統合
  - discord_rpc crate はすでに存在 (Phase 17 で作成)
  - `DiscordRpcClient` の接続/切断ライフサイクル
- [ ] 画面状態に応じたプレゼンス更新
- [ ] MainController から RPC クライアントへの状態通知

**見積り:** ~150 行実装 + ~6 テスト

## Phase 29: パフォーマンス最適化 + リファクタリング

依存: Phase 25d (スタブ解消), Phase 25c (品質ベースライン)

### 29a: 間接参照の削減

Phase 25d でスタブ→trait 化が完了した後、trait 間接参照をさらに削減:

- [ ] PlayerResource: `Box<dyn PlayerResourceAccess>` → 具象型 (ジェネリクス or 直接型)
- [ ] MainControllerAccess trait → 具象型の直接参照

**見積り:** ~200 行変更 + ~10 テスト

### 29b: PlayerResource trait の最適化

- [ ] 32 メソッド → 必要最小限に絞り込み
  - 使用頻度分析 (grep で各メソッドの呼び出し回数を集計)
  - 未使用メソッドの削除
  - 類似メソッドの統合 (get_gauge/get_groove_gauge 等)

**見積り:** ~150 行変更 + ~5 テスト

### 29c: メモリプロファイリング + テクスチャキャッシュ

- [ ] `dhat` / `jemalloc_ctl` でメモリプロファイリング
- [ ] テクスチャキャッシュ戦略の最適化 (Phase 26a の PixmapResourcePool をベースに)
- [ ] SpriteBatch のバッチ効率測定 + 最適化

**見積り:** ~200 行実装 + ~8 テスト

### 29d: 入力ポーリングの非同期化 (← Phase 24f deferred, nice-to-have)

- [ ] チャネルベースの非同期 polling thread
  - 現在: render() 内で同期 poll() (十分機能している)
  - 目標: 専用スレッド + crossbeam チャネルで winit イベント転送
  - 低レイテンシが必要な場合のみ実施

**見積り:** ~100 行実装 + ~5 テスト

## Known Issues (open)

- [ ] Remaining stubs: 10 stubs.rs files, ~1,520 行 → **Phase 25d** で解消
- [ ] 22 ignored tests (RenderSnapshot) → **Phase 26c** で解除
- **Intentional:** Twitter4j → `bail!()` (永久、~155行)

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
| 24f | MainController stubs resolved (state refs, listener dispatch, config save, create wiring) | +10 |
| 25a | Stub audit: 6 stubs.rs deleted, 3 reorganized (→gdx_compat, keys, pixmap_io, clipboard_helper, platform) | — |
| 25b | E2E integration tests (screen transitions + BarManager integration) | +32 |
| 25c | Quality assurance (zero clippy warnings, fmt clean, 22 ignored documented) | — |
