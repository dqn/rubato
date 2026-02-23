# Porting TODO — Remaining Work

Phases 1–33 complete. **1816 tests, 22 ignored (9 explicit #[ignore] + 13 fixture-absent runtime skips).** 27 crates, 122k lines. See AGENTS.md.

## Phase 26: スキンパイプライン完成 → 22 ignored テスト解除

Resolves: `beatoraja-skin/stubs.rs` (287 lines), `beatoraja-launcher/stubs.rs` (partial)

- [x] **26a:** `PixmapResourcePool` (wgpu テクスチャ ロード/キャッシュ/解放) + `SkinSourceImage`
- [x] **26b:** `SkinLoader.load_skin()` (SkinData→Skin 変換 + テクスチャバインド)
- [x] **26c:** Lua→JSON 型変換 + 13 テスト `#[ignore]` 解除 (残り 9 ignored: main_state API / SkinNote/SkinBar)
- [x] **26d:** バナー/ステージファイル画像 + `ReplayData::exists()` (依存: 26a)

## Phase 27: 楽曲 DB 拡張 + 検索

- [x] **27a:** `rayon::par_iter()` による BMS 並列走査
- [x] **27b:** SQLite FTS5 全文検索 (`get_song_datas_by_text()`)
- [x] **27c:** `SongInformationAccessor` trait + SQLite CRUD

## Phase 28: プラットフォーム固有 + 入力

Resolves: `beatoraja-input/stubs.rs` (44 lines)

- [x] **28a:** gilrs コントローラ + hotplug
- [x] **28b:** KeyCommand (F キー, Alt+Enter, ESC)
- [x] **28c:** Windows named pipe (`#[cfg(windows)]`)
- [x] **28d:** winit モニター列挙
- [x] **28e:** Discord Rich Presence (discord-rpc crate + DiscordListener 完全実装済み、MainController 接続配線 → **Phase 29a**)

## Phase 29: リファクタリング + スタブ解消

Resolves: rendering stubs (result/decide/select/modmenu ~972 lines), `beatoraja-types/stubs.rs` (549 lines), `beatoraja-external/stubs.rs` (partial)

- [x] **29a-1:** MainStateListener trait 統合 + Discord/OBS 接続配線 (StateAccessAdapter パターン)
- **29a-2:** rendering stubs 削減 (result/decide/select/modmenu ~972 lines) — → **Phase 33**
- **29a-3:** Property traits 統合 (beatoraja-skin vs beatoraja-external) — → **Phase 33**
- [x] **29b:** PlayerResource trait 分析完了 — 32メソッド中31が使用中、最小化不要
- [x] **29c:** dhat ヒーププロファイリング (`--features dhat-heap` で有効化、`dhat-heap.json` 出力)
- [x] **29d:** 入力ポーリング分析完了 — 同期で十分、スキップ

## Phase 30: 非レンダリングスタブ整理

レンダリングパイプライン非依存のスタブを独自ファイルへ移動・実装。

- [x] **30a:** `beatoraja-types` enum 移動 — `JudgeAlgorithm`, `BMSPlayerRule`, `BarSorter` + modifier enums を stubs.rs から専用ファイルへ
- [x] **30b:** `beatoraja-types` DTO 実装 — `KeyInputLog`, `PatternModifyLog` を stubs.rs から移動
- [x] **30c:** `beatoraja-input` `SkinWidgetManager::get_focus()` 実装 — skin_widget_focus を beatoraja-types に抽出
- [x] **30d:** `beatoraja-select` `DownloadTask` 系型 — stubs 削除、md-processor の実装を re-export

## Phase 31: Lua main_state API 拡張 → 5 ignored テスト解除

`compare_render_snapshot.rs` で `#[ignore]` されている 5 テストのブロッカー解消。

- [x] **31a:** `main_state.number(key)` Lua API — DelegateIntegerProperty でスキンLuaから数値参照
- [x] **31b:** `main_state.text(key)` Lua API — DelegateStringProperty でスキンLuaからテキスト参照

Resolves: `render_snapshot_ecfn_result_clear`, `_result_fail`, `_play14_active`, `_course_result`, `timeline_result_has_stable_visible_set`

## Phase 32: SkinNote/SkinBar/SkinJudge 型実装 → 4 ignored テスト解除

`compare_render_snapshot.rs` の残り 4 テストのブロッカー解消。

- [x] **32a:** `SkinNote` — SkinObject::Note variant + skin_data_converter 変換
- [x] **32b:** `SkinBar` — SkinObject::Bar variant + skin_data_converter 変換
- [x] **32c:** `SkinJudge` — SkinObject::Judge variant + skin_data_converter 変換

Resolves: `rust_only_snapshot_ecfn_play7_mid_song`, `_select_with_song`, `skin_state_objects_play_has_note_judge`, `skin_state_objects_select_has_bar`

## Phase 33: フルレンダリングパイプライン完成

Phase 29a-2/29a-3 の blocking 解消。レンダリングスタブの分析・置換。

- [x] **33a:** `SkinText::draw()`, `SkinNumber::draw()`, `SkinImage::draw()` — 検証完了。全て beatoraja-skin に完全実装済み (Phase 26/32 で完了済み)
- [x] **33b:** `SkinObjectRenderer::draw()` — 検証完了。beatoraja-skin/skin_object.rs に完全実装済み。select/modmenu のスタブは API 不一致のため直接置換不可 (異なるフィールド構造・mutability)
- [x] **33c:** `BooleanPropertyFactory` — `StubBooleanProperty` を `DelegateBooleanProperty` に置換。`MainState::boolean_value(id)` trait メソッド追加 (Integer/String と同パターン)
- [x] **33d:** stubs 整理 — result: `SkinObjectData` スタブ削除→ beatoraja-skin re-export。modmenu: 未使用 import (`Skin`, `MusicSelector`) 削除。select/modmenu/decide: API 不一致スタブは保持 (将来の SkinBar/SkinWidget リライト時に解消)

**残留スタブ分析:**
- `beatoraja-result/stubs.rs` (385 lines): MainController/PlayerResource ライフサイクルスタブ、IR re-export — 必須。SkinObjectData は削除済み
- `beatoraja-select/stubs.rs` (278 lines): SkinText/SkinNumber/SkinImage/SkinObject/SkinObjectRenderer — API 不一致 (stub は `&self`/simple fields、real は `&mut self`/SkinObjectData)。SkinBar リライト時に解消
- `beatoraja-modmenu/stubs.rs` (203 lines): Skin/SkinObject/SkinObjectDestination — SkinWidgetManager が Clone+Debug derive に依存。リライト時に解消
- `beatoraja-decide/stubs.rs` (108 lines): MainControllerRef/SkinStub — ライフサイクルスタブ、必須

## Permanent Stubs

- **Twitter4j** (`beatoraja-external`): ~155 lines, `bail!()` — API 廃止済みのため意図的に未実装
