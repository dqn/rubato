# Porting TODO — Remaining Work

Phases 1–39 complete, Phase 40a + 41a/41b + 43b complete. **1928 tests, 0 ignored.** 27 crates, 127k lines. See AGENTS.md.

---

## Phase 40: SkinWidget リライト + レンダリングスタブ解消

API 不整合スタブ (~481行) の解消。select/modmenu のレンダリングパイプライン完成。

- [x] **40a:** SkinWidget API 設計 — `&self` + simple fields → `&mut self` + SkinObjectData の borrow 問題を解決するアーキテクチャ設計
- [ ] **40b:** beatoraja-select レンダリングスタブ置換 (278行) — SkinText/SkinNumber/SkinImage/SkinObjectRenderer を実 API に接続
- [ ] **40c:** beatoraja-modmenu レンダリングスタブ置換 (203行) — Skin/SkinObject/SkinObjectDestination + MusicSelector 結合
- [ ] **40d:** ImGuiRenderer egui 統合 — modmenu の egui レンダリングパイプライン接続
- [ ] **40e:** MovieProcessor 動画再生パイプライン統合 — GdxVideoProcessor のスキン統合

## Phase 41: ライフサイクルスタブ統合

クロスクレート API 境界のライフサイクルスタブを実オブジェクトに置換。

- [ ] **41c:** AudioProcessor 統合 — result/decide の AudioProcessorStub を実オーディオドライバに接続
- [ ] **41d:** デバイス種別トラッキング — create_score_data() で MainController.get_input_processor().get_device_type() を接続
- [ ] **41e:** startJudge() 完全実装 — JudgeThread をスレッド化し KeyInputLog[] リプレイ入力再生を接続
- [ ] **41f:** KeyInputProcessor.input() 実装 — auto_presstime + キービーム + スクラッチアニメーション
- [ ] **41g:** ControlInputProcessor.input() 実装 — START+SELECT クイックリトライ + レーンカバー操作
- [ ] **41h:** EventFactory 実イベント実装 — 108 StubEvent を MusicSelector/MainController 経由の実ロジックに置換
- [ ] **41i:** オーディオプロセッサ統合 — グローバルピッチ、ガイドSE、ラウドネス、状態遷移BGM
- [ ] **41j:** BGA 表示統合 — BMSPlayer の BGA レイヤーとスキンレンダリングの接続

## Phase 42: Launcher egui 完全移行

JavaFX 設定 UI の egui 完全移行。設定ビューの動的動作実装。

- [ ] **42a:** 設定ビュー initialize/update/commit — PlayConfigurationView 等 14 ビューの初期化・更新・保存ロジック
- [ ] **42b:** エディタビュー — CourseEditorView, FolderEditorView, TableEditorView の実動作
- [ ] **42c:** DisplayMode/MonitorInfo 統合 — winit からのモニター情報を Launcher UI に反映

## Phase 43: BMSPlayer.create() + Skin ロード統合

BMSPlayer のスキンロード/初期化完成。

- [ ] **43a:** `BMSPlayer.create()` 完成 — loadSkin(), ガイドSEパス解決, 入力プロセッサモード設定

---

## 軽微な未移植項目

| 項目 | 影響 | 備考 |
|------|------|------|
| `BMSModel.compareTo()` | 低 | 必要時に Ord 実装可。Java でも未使用 |
| `BMSModelUtils.getAverageNotesPerTime()` | 低 | Java でも未使用 (デッドコード) |
| OBS reconnect lifecycle | 低 | server_uri/password の inner 保持が必要 |
| Skill rating calculation | 低 | Java ソースに実装なし (移植元不在) |

## Permanent Stubs

- **Twitter4j** (`beatoraja-external`): ~446 lines, `bail!()` — API 廃止済みのため意図的に未実装
- **ShortDirectPCM** (`beatoraja-audio`): Java 固有の DirectBuffer — Rust では不要
