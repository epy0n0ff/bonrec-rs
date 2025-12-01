# Implementation Plan: BonDriverチャンネルスキャン

**Branch**: `001-bondriver-channel-scan` | **Date**: 2025-12-01 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-bondriver-channel-scan/spec.md`

## Summary

BonDriverインターフェースを使用して、Windows環境でTVチューナーの全チャンネルをスキャンし、チャンネル情報（物理チャンネル、サービスID、放送局名等）を取得してJSON形式で出力するCLIツール。複数のBonDriverを同時指定可能で、スキャン結果はSQLiteデータベースに記録する。Rustで実装し、Windows DLLの動的ロードを行う。

## Technical Context

**Language/Version**: Rust 1.75+ (stable)
**Primary Dependencies**:
- `libloading` (Windows DLL動的ロード)
- `rusqlite` (SQLiteデータベース)
- `serde` / `serde_json` (JSON出力)
- `clap` (CLIパーサー)
- `indicatif` (進捗表示)
- `ctrlc` (中断処理)
- `tokio` (非同期処理、SI情報取得タイムアウト用)

**Storage**: SQLite (スキャン結果の永続化)
**Testing**: cargo test (単体テスト、統合テスト)
**Target Platform**: Windows (x86_64-pc-windows-msvc)
**Project Type**: single (CLIアプリケーション)
**Performance Goals**:
- チャンネルあたりのスキャン時間: SI情報取得タイムアウト + チューニング時間
- 全チャンネルスキャン: 放送波×チャンネル数に比例
**Constraints**:
- BonDriver DLLの動的ロード（Windows API）
- SI情報取得タイムアウト: ユーザー指定可能（デフォルト5秒）
- 中断時のリソース解放（チューナーの適切なクローズ）
**Scale/Scope**:
- 地上波: 約60チャンネル、BS: 約30チャンネル、CS: 約200チャンネル
- 複数BonDriver対応（一般的に2-4個）

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Constitution未設定のため、標準的なRustプロジェクトのベストプラクティスに従う:
- ✅ 単一のCLIアプリケーションとして構築（Library-First原則に従い、コア機能はライブラリとして分離）
- ✅ CLI経由での操作（stdin/args → stdout、エラー → stderr）
- ✅ テストファースト（単体テスト、統合テスト）
- ✅ JSON + 人間可読形式の出力サポート

## Project Structure

### Documentation (this feature)

```text
specs/001-bondriver-channel-scan/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (CLI contract)
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
src/
├── main.rs              # CLIエントリーポイント
├── lib.rs               # ライブラリルート
├── cli/
│   ├── mod.rs           # CLIモジュール
│   └── args.rs          # コマンドライン引数定義
├── bondriver/
│   ├── mod.rs           # BonDriverモジュール
│   ├── ffi.rs           # FFI定義（BonDriver API）
│   ├── loader.rs        # DLL動的ロード
│   └── tuner.rs         # チューナー操作
├── scanner/
│   ├── mod.rs           # スキャナーモジュール
│   ├── channel.rs       # チャンネルスキャンロジック
│   └── si_parser.rs     # SI情報パース
├── storage/
│   ├── mod.rs           # ストレージモジュール
│   ├── db.rs            # SQLite操作
│   └── models.rs        # データベースモデル
├── output/
│   ├── mod.rs           # 出力モジュール
│   ├── json.rs          # JSON出力
│   └── formatter.rs     # 出力フォーマッター
└── error.rs             # エラー型定義

tests/
├── unit/
│   ├── bondriver_tests.rs
│   ├── scanner_tests.rs
│   └── storage_tests.rs
└── integration/
    └── scan_workflow_tests.rs
```

**Structure Decision**: 単一プロジェクト構成を採用。コア機能（bondriver, scanner, storage, output）はライブラリとしてモジュール化し、main.rsがCLIエントリーポイントとして機能する。これにより、将来的にライブラリとしての再利用も可能。

## Complexity Tracking

該当なし（Constitution違反なし）
