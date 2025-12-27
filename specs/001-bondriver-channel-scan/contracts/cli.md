# CLI Contract: bonrec-rs

**Date**: 2025-12-01
**Feature**: 001-bondriver-channel-scan

## コマンド概要

```
bonrec-rs scan [OPTIONS] <BONDRIVER>...
```

## 引数

### 必須引数

| 引数 | 説明 |
|------|------|
| `<BONDRIVER>...` | BonDriver DLLファイルのパス（複数指定可能） |

### オプション

| オプション | 短縮 | デフォルト | 説明 |
|-----------|------|-----------|------|
| `--output <PATH>` | `-o` | (stdout) | 出力先ファイルパス（未指定時は標準出力） |
| `--format <FORMAT>` | `-f` | json | 出力フォーマット（json/csv） |
| `--timeout <SECONDS>` | `-t` | 5 | SI情報取得タイムアウト（秒） |
| `--space <NAME>...` | `-s` | (all) | スキャン対象のチューニングスペース（複数指定可能） |
| `--db <PATH>` | | ./bonrec.db | SQLiteデータベースパス |
| `--no-progress` | | false | 進捗表示を無効化 |
| `--verbose` | `-v` | false | 詳細ログ出力 |
| `--help` | `-h` | | ヘルプ表示 |
| `--version` | `-V` | | バージョン表示 |

## 使用例

### 基本的なスキャン
```bash
bonrec-rs scan BonDriver_PT3-T.dll
```

### 複数BonDriverを指定
```bash
bonrec-rs scan BonDriver_PT3-T.dll BonDriver_PT3-S.dll
```

### ファイルに出力
```bash
bonrec-rs scan -o channels.json BonDriver_PT3-T.dll
```

### 地上波のみスキャン
```bash
bonrec-rs scan -s "地上D" BonDriver_PT3-T.dll
```

### タイムアウトを10秒に設定
```bash
bonrec-rs scan -t 10 BonDriver_PT3-T.dll
```

### CSV形式で出力
```bash
bonrec-rs scan -f csv -o channels.csv BonDriver_PT3-T.dll
```

## 終了コード

| コード | 意味 |
|--------|------|
| 0 | 正常終了 |
| 1 | 一般的なエラー |
| 2 | BonDriver読み込みエラー |
| 3 | チューナーオープンエラー |
| 4 | ユーザーによる中断（Ctrl+C） |

## 標準出力

### JSON形式（デフォルト）
```json
{
  "scan_session_id": 1,
  "started_at": "2025-12-01T10:00:00Z",
  "completed_at": "2025-12-01T10:15:30Z",
  "status": "completed",
  "channels": [
    {
      "tuning_space": "地上D",
      "channel_index": 27,
      "channel_name": "27ch",
      "physical_channel": 27,
      "services": [...],
      "available_from": [...]
    }
  ]
}
```

### CSV形式
```csv
tuning_space,channel_index,channel_name,physical_channel,service_id,network_id,transport_stream_id,service_name,broadcaster_name,available_from
地上D,27,27ch,27,1024,32736,32736,NHK総合1・東京,NHK,"BonDriver_PT3-T0.dll;BonDriver_PT3-T1.dll"
```

## 標準エラー出力

### 進捗表示（--no-progress未指定時）
```
Scanning with BonDriver_PT3-T.dll...
[████████████████░░░░] 80% (48/60) 地上D ch45
```

### エラーメッセージ
```
Error: Failed to load BonDriver: BonDriver_XXX.dll
  Caused by: The specified module could not be found.
```

### 詳細ログ（-v指定時）
```
[2025-12-01T10:00:00Z INFO ] Starting scan session #1
[2025-12-01T10:00:00Z DEBUG] Loading BonDriver: BonDriver_PT3-T.dll
[2025-12-01T10:00:01Z DEBUG] Tuner opened: PT3 ISDB-T Tuner #0
[2025-12-01T10:00:01Z DEBUG] EnumTuningSpace(0) = "地上D"
[2025-12-01T10:00:01Z DEBUG] EnumTuningSpace(1) = NULL (end)
[2025-12-01T10:00:01Z DEBUG] Scanning space 0: 地上D
[2025-12-01T10:00:01Z DEBUG] SetChannel(0, 0)
[2025-12-01T10:00:06Z DEBUG] SI info received for ch0
...
```

## 中断処理

Ctrl+Cで中断した場合：

1. 現在のチャンネルスキャンを完了
2. それまでに取得した情報をデータベースに保存
3. status: "interrupted" でJSONを出力
4. 終了コード4で終了

```json
{
  "scan_session_id": 1,
  "started_at": "2025-12-01T10:00:00Z",
  "completed_at": "2025-12-01T10:05:30Z",
  "status": "interrupted",
  "channels": [...]
}
```

## データベースサブコマンド（将来拡張）

```bash
# 過去のスキャン結果を表示
bonrec-rs history

# 特定のスキャン結果をエクスポート
bonrec-rs export --session 1 -o channels.json

# データベースを初期化
bonrec-rs init-db
```
