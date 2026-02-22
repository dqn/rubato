# Porting TODO — Remaining Work

Phases 1–26b, 26d complete. **1780 tests, 22 ignored.** 27 crates, 122k lines. See AGENTS.md.

## Phase 26: スキンパイプライン完成 → 22 ignored テスト解除

Resolves: `beatoraja-skin/stubs.rs` (287 lines), `beatoraja-launcher/stubs.rs` (partial)

- [x] **26a:** `PixmapResourcePool` (wgpu テクスチャ ロード/キャッシュ/解放) + `SkinSourceImage`
- [x] **26b:** `SkinLoader.load_skin()` (SkinData→Skin 変換 + テクスチャバインド)
- **26c:** RenderSnapshot Java fixture 生成 + 22 テスト `#[ignore]` 解除 (依存: 26b)
- [x] **26d:** バナー/ステージファイル画像 + `ReplayData::exists()` (依存: 26a)

## Phase 27: 楽曲 DB 拡張 + 検索

- **27a:** `rayon::par_iter()` による BMS 並列走査
- **27b:** SQLite FTS5 全文検索 (`get_song_datas_by_text()`)
- **27c:** `SongInformationAccessor` trait + SQLite CRUD

## Phase 28: プラットフォーム固有 + 入力

Resolves: `beatoraja-input/stubs.rs` (44 lines)

- **28a:** gilrs コントローラ + hotplug
- **28b:** KeyCommand (F キー, Alt+Enter, ESC)
- **28c:** Windows named pipe (`#[cfg(windows)]`)
- **28d:** winit モニター列挙
- **28e:** Discord Rich Presence

## Phase 29: リファクタリング + スタブ解消

Resolves: rendering stubs (result/decide/select/modmenu ~972 lines), `beatoraja-types/stubs.rs` (549 lines), `beatoraja-external/stubs.rs` (partial)

- **29a:** API 非互換スタブ解消 (result/decide/select/modmenu rendering, Property traits)
- **29b:** PlayerResource trait 最小化
- **29c:** メモリプロファイリング (`dhat`/`jemalloc_ctl`)
- **29d:** 入力ポーリング非同期化 (nice-to-have)

## Permanent Stubs

- **Twitter4j** (`beatoraja-external`): ~155 lines, `bail!()` — API 廃止済みのため意図的に未実装
