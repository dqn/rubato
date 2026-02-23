# Porting TODO — Remaining Work

Phases 1–39 complete. **1905 tests, 0 ignored.** 27 crates, 127k lines. See AGENTS.md.

---

## Phase 34: BMSPlayer 初期化ロジック移植

最大のギャップ。Java版 BMSPlayer コンストラクタの ~390行の初期化処理を移植。ゲームプレイの実動作に必須。

- [x] **34a:** PatternModifier 生成・適用 — PlayerConfig から Random/Mirror/Scatter 等のオプション読み取り → PatternModifier チェーン構築 → BMSModel に適用
- [x] **34b:** 乱数シード管理 — リプレイ再現用シード保存/復元、JavaRandom LCG シード初期化パス
- [x] **34c:** リプレイデータ復元 — ReplayData からパターン/ゲージ/設定を復元、リプレイモード時のオプション上書き
- [x] **34d:** アシストレベル計算 — BPMガイド、カスタムジャッジ、定速等のアシストフラグ判定
- [x] **34e:** DP→SP オプション変換 — ダブルプレイ時の 2P 側オプション処理
- [x] **34f:** 周波数トレーナー統合 — FreqTrainer との速度変更連携

## Phase 35: スコアデータ統計完成

BMSPlayer.createScoreData() の統計計算部分。リザルト画面/DB保存に影響。

- [x] **35a:** タイミング分散計算 — avgduration, average, stddev (ジャッジタイミングの統計)
- [!] **35b:** デバイス種別トラッキング — キーボード/コントローラ/MIDI の入力デバイス判定・記録 (BMSPlayer lacks MainController access; deferred → **Phase 41**)
- [!] **35c:** スキル値計算 — スコアに基づくスキルレーティング算出 (Java ソースに skill 計算の実装なし。移植元が存在しないためスキップ)

## Phase 36: 入力/リプレイ統合

KeyInputProcessor の完全実装。リプレイ再生・オートプレイに必須。

- [!] **36a:** `startJudge()` 完全実装 — KeyInputLog[] リプレイキーログからの入力再生 (スレッド化 + 共有状態が必要、deferred → **Phase 41**)
- [!] **36b:** オートプレイ押下シミュレーション — auto_presstime 配列によるノート自動押下スケジューリング (KeyInputProccessor.input() が MainController 依存、deferred → **Phase 41**)
- [!] **36c:** クイックリトライ検出 — START+SELECT 同時押しによる即リトライ (ControlInputProcessor.input() が BMSPlayer/MainController 依存、deferred → **Phase 41**)
- [x] **36d:** JudgeManager.init() 完全実装 — カスタムジャッジレート、コース制約 (NO_GREAT/NO_GOOD)、LN タイプ設定

## Phase 37: イベントディスパッチ実装

beatoraja-skin EventFactory の StubEvent を実際のイベントロジックに置換。

- [!] **37a:** MusicSelector 操作イベント — ソート変更、ゲージ変更、オプション切替、リプレイ操作 (全108イベントが MusicSelector/MainController 依存、deferred → **Phase 41**)
- [!] **37b:** 設定変更イベント — プレイコンフィグ変更 (ガイドSE, BGA, レーンカバー等) (同上)
- [!] **37c:** IR 操作イベント — IR 接続/ランキング取得/スコア送信のトリガー (同上)
- [!] **37d:** その他イベント — スクリーンショット、Twitter (スキップ)、キーコンフィグリセット等 (同上、keyassign 54個は Java でも空実装)

## Phase 38: オーディオプロセッサ統合

BMSPlayer/MainController とオーディオシステムの接続。

- [!] **38a:** グローバルピッチ制御 — `AudioProcessor.setGlobalPitch()` を MainController 経由で BMSPlayer に接続 (MainController 依存、deferred → **Phase 41**)
- [!] **38b:** ガイド SE 設定 — Config.isGuideSE() に基づくガイドサウンドパスの解決・再生 (同上)
- [!] **38c:** ラウドネス分析統合 — BMSLoudnessAnalyzer の結果をレンダリング時のボリューム正規化に適用 (ebur128 外部依存 + ライフサイクル依存)
- [!] **38d:** 状態遷移 BGM — Select/Decide/Result 間の BGM 再生・フェード制御 (MainController 依存、deferred → **Phase 41**)

## Phase 39: BGA 動画処理

beatoraja-play/bga の動画プロセッサ実装。

- [x] **39a:** FFmpegProcessor 実装 — 外部 FFmpeg プロセスによる動画デコード (フレーム抽出 → テクスチャ) (既に実装済み、610行)
- [!] **39b:** MovieProcessor/GdxVideoProcessor — 動画再生パイプライン統合 (スキン統合依存、deferred → **Phase 40**)
- [!] **39c:** BGA 表示統合 — BMSPlayer の BGA レイヤーとスキンレンダリングの接続 (Phase 40/41 依存)

## Phase 40: SkinWidget リライト + レンダリングスタブ解消

API 不整合スタブ (~481行) の解消。select/modmenu のレンダリングパイプライン完成。

- [ ] **40a:** SkinWidget API 設計 — `&self` + simple fields → `&mut self` + SkinObjectData の borrow 問題を解決するアーキテクチャ設計
- [ ] **40b:** beatoraja-select レンダリングスタブ置換 (278行) — SkinText/SkinNumber/SkinImage/SkinObjectRenderer を実 API に接続
- [ ] **40c:** beatoraja-modmenu レンダリングスタブ置換 (203行) — Skin/SkinObject/SkinObjectDestination + MusicSelector 結合
- [ ] **40d:** ImGuiRenderer egui 統合 — modmenu の egui レンダリングパイプライン接続

## Phase 41: ライフサイクルスタブ統合

クロスクレート API 境界のライフサイクルスタブ (~939行) を実オブジェクトに置換。

- [x] **41a:** PlayerResource クロスクレート統合 — result/decide/external の PlayerResource スタブを trait ベースの実オブジェクトに置換
- [x] **41b:** MainController クロスクレート統合 — result/decide の MainControllerRef を実 MainController 参照に置換
- [ ] **41d:** デバイス種別トラッキング (from 35b) — create_score_data() で MainController.get_input_processor().get_device_type() を接続
- [ ] **41c:** AudioProcessor 統合 — result/decide の AudioProcessorStub を実オーディオドライバに接続
- [ ] **41e:** startJudge() 完全実装 (from 36a) — JudgeThread をスレッド化し KeyInputLog[] リプレイ入力再生を接続
- [ ] **41f:** KeyInputProccessor.input() 実装 (from 36b) — auto_presstime + キービーム + スクラッチアニメーション
- [ ] **41g:** ControlInputProcessor.input() 実装 (from 36c) — START+SELECT クイックリトライ + レーンカバー操作
- [ ] **41h:** EventFactory 実イベント実装 (from 37a-d) — 108 StubEvent を MusicSelector/MainController 経由の実ロジックに置換
- [ ] **41i:** オーディオプロセッサ統合 (from 38a-d) — グローバルピッチ、ガイドSE、ラウドネス、状態遷移BGM

## Phase 42: Launcher egui 完全移行

JavaFX 設定 UI の egui 完全移行。設定ビューの動的動作実装。

- [ ] **42a:** 設定ビュー initialize/update/commit — PlayConfigurationView 等 14 ビューの初期化・更新・保存ロジック
- [ ] **42b:** エディタビュー — CourseEditorView, FolderEditorView, TableEditorView の実動作
- [ ] **42c:** DisplayMode/MonitorInfo 統合 — winit からのモニター情報を Launcher UI に反映

## Phase 43: BMSPlayer.create() + Skin ロード統合

BMSPlayer のスキンロード/初期化完成。

- [ ] **43a:** `BMSPlayer.create()` 完成 — loadSkin(), ガイドSEパス解決, 入力プロセッサモード設定
- [x] **43b:** プラクティスモード統合 — PracticeConfiguration のプロパティを BMSModel に適用 (周波数/LNモード/乱数シード/時間範囲)

---

## 軽微な未移植項目

| 項目 | 影響 | 備考 |
|------|------|------|
| `BMSModel.compareTo()` | 低 | 必要時に Ord 実装可。Java でも未使用 |
| ~~`BMSModel.getEventLane()` / `getLanes()`~~ | ~~低~~ | ✅ 実装済み |
| `BMSModelUtils.getAverageNotesPerTime()` | 低 | Java でも未使用 (デッドコード) |
| OBS reconnect lifecycle | 低 | server_uri/password の inner 保持が必要 |

## Permanent Stubs

- **Twitter4j** (`beatoraja-external`): ~446 lines, `bail!()` — API 廃止済みのため意図的に未実装
- **ShortDirectPCM** (`beatoraja-audio`): Java 固有の DirectBuffer — Rust では不要
