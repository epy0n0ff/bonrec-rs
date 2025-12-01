# Tasks: BonDriverチャンネルスキャン

**Input**: Design documents from `/specs/001-bondriver-channel-scan/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: テストタスクは含まれていません（明示的なリクエストがないため）。必要に応じて追加可能です。

**Organization**: タスクはユーザーストーリーごとにグループ化されており、各ストーリーを独立して実装・テストできます。

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 並列実行可能（異なるファイル、依存関係なし）
- **[Story]**: このタスクが属するユーザーストーリー（US1, US2, US3）
- 説明には正確なファイルパスを含む

## Path Conventions

```text
src/
├── main.rs              # CLIエントリーポイント
├── lib.rs               # ライブラリルート
├── cli/                 # CLIモジュール
├── bondriver/           # BonDriver FFI
├── scanner/             # チャンネルスキャン
├── storage/             # SQLiteストレージ
├── output/              # 出力フォーマッター
└── error.rs             # エラー型
```

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Rustプロジェクトの初期化と基本構造の構築

- [x] T001 Create Cargo.toml with dependencies (libloading, rusqlite, serde, serde_json, clap, indicatif, ctrlc, thiserror, chrono, mpeg2ts-reader)
- [x] T002 Create project directory structure per plan.md in src/
- [x] T003 [P] Create src/lib.rs with module declarations (cli, bondriver, scanner, storage, output, error)
- [x] T004 [P] Create src/main.rs with minimal CLI entry point
- [x] T005 [P] Configure .gitignore for Rust project and SQLite files

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 全ユーザーストーリーに必要なコアインフラの構築

**⚠️ CRITICAL**: このフェーズが完了するまで、ユーザーストーリーの作業は開始できません

### Error Handling & Core Types

- [x] T006 Define BonrecError enum with all error variants in src/error.rs
- [x] T007 [P] Define ScanStatus enum in src/storage/models.rs
- [x] T008 [P] Define data model structs (ScanSession, BonDriverSource, TuningSpace, Channel, Service) in src/storage/models.rs

### BonDriver FFI Layer

- [x] T009 Define BonDriver FFI types (BOOL, BYTE, DWORD, LPCTSTR) in src/bondriver/ffi.rs
- [x] T010 Define IBonDriver vtable struct in src/bondriver/ffi.rs
- [x] T011 Define IBonDriver2 vtable struct extending IBonDriver in src/bondriver/ffi.rs
- [x] T012 Implement BonDriver DLL loader using libloading in src/bondriver/loader.rs
- [x] T013 Implement BonDriver wrapper struct with RAII Drop in src/bondriver/tuner.rs
- [x] T014 Implement tuner operations (open, close, set_channel, get_signal_level) in src/bondriver/tuner.rs
- [x] T015 Implement channel enumeration (EnumTuningSpace, EnumChannelName) in src/bondriver/tuner.rs
- [x] T016 Implement TS stream reading (GetTsStream, WaitTsStream, PurgeTsStream) in src/bondriver/tuner.rs
- [x] T017 Create src/bondriver/mod.rs exporting public API

### Storage Layer

- [x] T018 Implement SQLite database initialization with schema in src/storage/db.rs
- [x] T019 Implement ScanSession CRUD operations in src/storage/db.rs
- [x] T020 Implement BonDriverSource CRUD operations in src/storage/db.rs
- [x] T021 Implement TuningSpace CRUD operations in src/storage/db.rs
- [x] T022 Implement Channel CRUD operations in src/storage/db.rs
- [x] T023 Implement Service CRUD operations in src/storage/db.rs
- [x] T024 Implement ChannelSource (many-to-many) operations in src/storage/db.rs
- [x] T025 Create src/storage/mod.rs exporting public API

**Checkpoint**: Foundation ready - ユーザーストーリーの実装を開始できます

---

## Phase 3: User Story 1 - 全チャンネルスキャンの実行 (Priority: P1) 🎯 MVP

**Goal**: BonDriverを指定してスキャンを実行し、全チャンネルの情報をJSON形式で標準出力に表示する

**Independent Test**: `bonrec-rs scan BonDriver_PT3-T.dll` を実行し、チャンネル一覧がJSON形式で出力されることを確認

### SI Information Parser

- [x] T026 [P] [US1] Implement MPEG-2 TS packet parser basics in src/scanner/si_parser.rs
- [x] T027 [P] [US1] Implement PAT (Program Association Table) parser in src/scanner/si_parser.rs
- [x] T028 [US1] Implement SDT (Service Description Table) parser for service names in src/scanner/si_parser.rs
- [x] T029 [US1] Implement NIT (Network Information Table) parser for network IDs in src/scanner/si_parser.rs
- [x] T030 [US1] Implement ARIB character decoding for Japanese text in src/scanner/si_parser.rs

### Channel Scanner

- [x] T031 [US1] Implement single channel scan logic with SI timeout in src/scanner/channel.rs
- [x] T032 [US1] Implement full tuning space scan (all channels in a space) in src/scanner/channel.rs
- [x] T033 [US1] Implement progress tracking with indicatif in src/scanner/channel.rs
- [x] T034 [US1] Implement Ctrl+C interrupt handling with ctrlc in src/scanner/channel.rs
- [x] T035 [US1] Implement graceful shutdown saving partial results in src/scanner/channel.rs
- [x] T036 [US1] Create src/scanner/mod.rs exporting ChannelScanner

### JSON Output

- [x] T037 [P] [US1] Define ScannedChannel and ScannedService output structs in src/output/json.rs
- [x] T038 [P] [US1] Define ScanResult output struct in src/output/json.rs
- [x] T039 [US1] Implement JSON serialization for ScanResult in src/output/json.rs
- [x] T040 [US1] Implement stdout JSON output in src/output/json.rs
- [x] T041 [US1] Create src/output/mod.rs exporting output functions

### CLI Integration for US1

- [x] T042 [US1] Define CLI args struct with clap (bondriver paths, db path, timeout, verbose) in src/cli/args.rs
- [x] T043 [US1] Implement scan subcommand handler in src/cli/mod.rs
- [x] T044 [US1] Integrate scanner with storage (save to SQLite) in src/cli/mod.rs
- [x] T045 [US1] Integrate scanner with JSON output in src/cli/mod.rs
- [x] T046 [US1] Update src/main.rs to use CLI module

**Checkpoint**: User Story 1完了 - 基本的なチャンネルスキャンが動作し、JSON出力が可能

---

## Phase 4: User Story 2 - チャンネル情報の出力フォーマット選択 (Priority: P2)

**Goal**: スキャン結果をファイル出力、CSV形式に対応

**Independent Test**: `bonrec-rs scan -o channels.json BonDriver.dll` および `bonrec-rs scan -f csv -o channels.csv BonDriver.dll` が正しく動作

### File Output

- [x] T047 [P] [US2] Implement file output option (-o) in src/output/formatter.rs
- [x] T048 [P] [US2] Implement CSV output format in src/output/formatter.rs

### CLI Extension for US2

- [x] T049 [US2] Add --output/-o and --format/-f options to CLI args in src/cli/args.rs
- [x] T050 [US2] Implement output format selection logic in src/cli/mod.rs
- [x] T051 [US2] Implement file vs stdout output routing in src/cli/mod.rs

**Checkpoint**: User Story 2完了 - ファイル出力とCSV形式が利用可能

---

## Phase 5: User Story 3 - 特定放送波のみのスキャン (Priority: P3)

**Goal**: 地上波のみ、BS/CSのみなど特定のチューニングスペースを選択してスキャン

**Independent Test**: `bonrec-rs scan -s "地上D" BonDriver.dll` で地上波のみスキャンされることを確認

### Space Filtering

- [x] T052 [US3] Implement tuning space filter logic in src/scanner/channel.rs
- [x] T053 [US3] Add --space/-s option to CLI args in src/cli/args.rs
- [x] T054 [US3] Integrate space filter with scan command in src/cli/mod.rs

### Multiple BonDriver Support

- [x] T055 [US3] Implement multiple BonDriver sequential scanning in src/scanner/channel.rs
- [x] T056 [US3] Implement channel deduplication with source tracking in src/scanner/channel.rs
- [x] T057 [US3] Update JSON output to include available_from field in src/output/json.rs

**Checkpoint**: User Story 3完了 - 放送波フィルタと複数BonDriver対応が完了

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: 全ユーザーストーリーに影響する改善

- [x] T058 [P] Add --no-progress option for CI/script usage in src/cli/args.rs
- [x] T059 [P] Add --verbose/-v option for debug logging in src/cli/args.rs
- [x] T060 Implement proper exit codes per CLI contract in src/main.rs
- [x] T061 [P] Add error context with thiserror for better error messages in src/error.rs
- [x] T062 Validate quickstart.md scenarios work correctly
- [x] T063 [P] Add README.md with usage examples at repository root

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: 依存なし - 即座に開始可能
- **Foundational (Phase 2)**: Setupの完了に依存 - 全ユーザーストーリーをブロック
- **User Stories (Phase 3-5)**: Foundationalフェーズの完了に依存
  - 優先順位順に順次実行 (P1 → P2 → P3)
  - または並列実行可能（複数開発者がいる場合）
- **Polish (Phase 6)**: 必要なユーザーストーリーの完了に依存

### User Story Dependencies

- **User Story 1 (P1)**: Foundational完了後に開始可能 - 他ストーリーへの依存なし
- **User Story 2 (P2)**: Foundational完了後に開始可能 - US1の出力構造に依存
- **User Story 3 (P3)**: Foundational完了後に開始可能 - US1のスキャンロジックに依存

### Within Each User Story

- モデル → サービス → エンドポイントの順
- コア実装 → 統合の順
- ストーリー完了後に次の優先度へ移行

### Parallel Opportunities

**Phase 2 (Foundational)**:
- T007, T008 は並列実行可能
- T009-T017 (BonDriver FFI) は順次実行（依存関係あり）
- T018-T025 (Storage) は T007, T008 完了後に並列実行可能

**Phase 3 (User Story 1)**:
- T026, T027 (SI Parser基礎) は並列実行可能
- T037, T038 (出力構造体) は並列実行可能

**Phase 4-5 (User Stories 2-3)**:
- 各ストーリー内の [P] タスクは並列実行可能

---

## Parallel Example: Foundational Phase

```bash
# FFI Types (parallel):
# - T007: Define ScanStatus enum
# - T008: Define data model structs

# After T007, T008 complete, Storage (parallel):
# - T018: SQLite initialization
# - T019-T024: CRUD operations (can be parallelized after T018)
```

## Parallel Example: User Story 1

```bash
# SI Parser (parallel):
Task: "Implement MPEG-2 TS packet parser basics in src/scanner/si_parser.rs"
Task: "Implement PAT parser in src/scanner/si_parser.rs"

# Output structs (parallel):
Task: "Define ScannedChannel and ScannedService in src/output/json.rs"
Task: "Define ScanResult in src/output/json.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1: Setup を完了
2. Phase 2: Foundational を完了（⚠️ 必須 - 全ストーリーをブロック）
3. Phase 3: User Story 1 を完了
4. **検証**: `bonrec-rs scan BonDriver.dll` でJSON出力を確認
5. MVP としてリリース可能

### Incremental Delivery

1. Setup + Foundational → 基盤完成
2. User Story 1 追加 → 独立テスト → デプロイ (MVP!)
3. User Story 2 追加 → 独立テスト → デプロイ (ファイル出力対応)
4. User Story 3 追加 → 独立テスト → デプロイ (フル機能)
5. 各ストーリーは前のストーリーを壊さずに価値を追加

---

## Notes

- [P] タスク = 異なるファイル、依存関係なし
- [Story] ラベルはタスクを特定のユーザーストーリーにマッピング
- 各ユーザーストーリーは独立して完了・テスト可能であるべき
- 各タスクまたは論理的なグループの後にコミット
- チェックポイントで停止してストーリーを独立して検証可能
- 避けるべき: 曖昧なタスク、同一ファイルの競合、独立性を損なうクロスストーリー依存
