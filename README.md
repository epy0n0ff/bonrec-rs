# bonrec-rs

BonDriverを使用してチャンネルスキャンを行い、チャンネル情報を取得するRust製CLIツール。

## 機能

- BonDriverを指定して全チャンネルをスキャン
- 複数のBonDriverを同時に指定可能（重複チャンネルは自動マージ）
- 特定の放送波（地上波、BS、CS等）のみをスキャン可能
- JSON/CSV形式で出力
- スキャン結果をSQLiteデータベースに保存
- 保存済みスキャン結果のエクスポート
- Ctrl+Cで安全に中断（部分的な結果を保存）
- プログレスバー表示

## 動作環境

- Windows 10/11 (64-bit)
- BonDriver対応チューナー

## インストール

### ビルド済みバイナリ

[Releases](https://github.com/epy0n0ff/bonrec-rs/releases)ページからダウンロード

### ソースからビルド

```powershell
git clone https://github.com/epy0n0ff/bonrec-rs.git
cd bonrec-rs
cargo build --release
```

実行ファイル: `target/release/bonrec-rs.exe`

## 使い方

### 基本的なスキャン

```powershell
bonrec-rs scan C:\BonDriver\BonDriver_PT3-T.dll
```

### 複数チューナーでスキャン

```powershell
bonrec-rs scan BonDriver_PT3-T.dll BonDriver_PT3-S.dll
```

### ファイルに出力

```powershell
# JSON形式
bonrec-rs scan -o channels.json BonDriver_PT3-T.dll

# CSV形式
bonrec-rs scan -f csv -o channels.csv BonDriver_PT3-T.dll
```

### 特定の放送波のみスキャン

```powershell
# 地上波のみ
bonrec-rs scan -s "地上D" BonDriver_PT3-T.dll

# BSとCSのみ
bonrec-rs scan -s "BS" -s "CS110" BonDriver_PT3-S.dll
```

### データベースからエクスポート

```powershell
# 最新のスキャン結果をJSON出力
bonrec-rs export --latest

# 最新のスキャン結果をファイルに出力
bonrec-rs export --latest -o channels.json

# 特定のセッションIDを指定して出力
bonrec-rs export --session 1 -o channels.json

# CSV形式で出力
bonrec-rs export --latest -f csv -o channels.csv
```

### その他のオプション

```
USAGE:
    bonrec-rs [OPTIONS] <COMMAND>

COMMANDS:
    scan      Scan channels using BonDriver
    export    Export scan results from database

OPTIONS:
    -v, --verbose    Enable verbose logging
    -h, --help       Print help
    -V, --version    Print version

SCAN OPTIONS:
    -o, --output <FILE>      Output file path (default: stdout)
    -f, --format <FORMAT>    Output format: json, csv (default: json)
    -t, --timeout <SECS>     SI information timeout (default: 5)
    -s, --space <NAME>       Tuning spaces to scan (can specify multiple)
        --db <PATH>          SQLite database path (default: bonrec.db)
        --no-progress        Disable progress bar

EXPORT OPTIONS:
        --session <ID>       Scan session ID to export
        --latest             Export the latest scan session
    -o, --output <FILE>      Output file path (default: stdout)
    -f, --format <FORMAT>    Output format: json, csv (default: json)
        --db <PATH>          SQLite database path (default: bonrec.db)
```

## 出力形式

### JSON

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
      "services": [
        {
          "service_id": 1024,
          "service_name": "NHK総合1・東京",
          "broadcaster_name": "NHK"
        }
      ],
      "available_from": ["BonDriver_PT3-T.dll"]
    }
  ]
}
```

### CSV

```csv
tuning_space,channel_index,channel_name,physical_channel,service_id,network_id,transport_stream_id,service_name,broadcaster_name,available_from
地上D,27,27ch,27,1024,32736,32736,NHK総合1・東京,NHK,BonDriver_PT3-T.dll
```

## Exit Codes

| Code | Description |
|------|-------------|
| 0 | Success |
| 1 | General error |
| 2 | BonDriver load error |
| 3 | Tuner open error |
| 4 | User interrupted |

## 開発

### テスト

```powershell
cargo test
```

### ドキュメント

```powershell
cargo doc --open
```

## ライセンス

MIT License
