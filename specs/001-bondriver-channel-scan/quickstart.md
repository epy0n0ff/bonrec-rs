# Quickstart: BonDriverチャンネルスキャン

## 前提条件

- Windows 10/11 (64-bit)
- BonDriver DLLファイル（対応チューナー用）
- チューナーデバイスがインストール済み

## ビルド

### Rustツールチェーンのインストール
```powershell
# rustup (Windows)
winget install Rustlang.Rustup

# または公式インストーラー
# https://www.rust-lang.org/tools/install
```

### ビルド実行
```powershell
# クローン
git clone https://github.com/epy0n0ff/bonrec-rs.git
cd bonrec-rs

# ブランチ切り替え
git checkout 001-bondriver-channel-scan

# ビルド（リリース版）
cargo build --release

# 実行ファイルは target/release/bonrec-rs.exe
```

## 基本的な使い方

### 1. 全チャンネルスキャン

```powershell
# BonDriverを指定してスキャン
.\target\release\bonrec-rs.exe scan C:\BonDriver\BonDriver_PT3-T.dll
```

出力例：
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
      "available_from": ["C:\\BonDriver\\BonDriver_PT3-T.dll"]
    }
  ]
}
```

### 2. 複数チューナーでスキャン

```powershell
# 地上波用とBS/CS用のBonDriverを両方指定
.\target\release\bonrec-rs.exe scan `
  C:\BonDriver\BonDriver_PT3-T.dll `
  C:\BonDriver\BonDriver_PT3-S.dll
```

### 3. 結果をファイルに保存

```powershell
# JSONファイルに出力
.\target\release\bonrec-rs.exe scan -o channels.json C:\BonDriver\BonDriver_PT3-T.dll

# CSVファイルに出力
.\target\release\bonrec-rs.exe scan -f csv -o channels.csv C:\BonDriver\BonDriver_PT3-T.dll
```

### 4. 特定の放送波のみスキャン

```powershell
# 地上波のみ
.\target\release\bonrec-rs.exe scan -s "地上D" C:\BonDriver\BonDriver_PT3-T.dll

# BSとCSのみ
.\target\release\bonrec-rs.exe scan -s "BS" -s "CS110" C:\BonDriver\BonDriver_PT3-S.dll
```

### 5. SI情報取得タイムアウトの調整

```powershell
# タイムアウトを10秒に（信号が弱い場合）
.\target\release\bonrec-rs.exe scan -t 10 C:\BonDriver\BonDriver_PT3-T.dll

# タイムアウトを3秒に（高速スキャン）
.\target\release\bonrec-rs.exe scan -t 3 C:\BonDriver\BonDriver_PT3-T.dll
```

## トラブルシューティング

### BonDriverが読み込めない

```
Error: Failed to load BonDriver: BonDriver_XXX.dll
  Caused by: The specified module could not be found.
```

**解決策**:
1. DLLファイルのパスが正しいか確認
2. 32bit/64bitが一致しているか確認（bonrec-rsは64bit）
3. 依存DLL（Visual C++ ランタイム等）がインストールされているか確認

### チューナーが開けない

```
Error: Failed to open tuner
```

**解決策**:
1. 他のアプリケーション（TVTest等）がチューナーを使用していないか確認
2. デバイスマネージャーでチューナーが認識されているか確認
3. BonDriverの設定ファイル（.ini等）を確認

### SI情報が取得できない

```
Warning: SI info timeout for channel 27
```

**解決策**:
1. `-t`オプションでタイムアウトを延長（例: `-t 10`）
2. アンテナ接続を確認
3. 信号レベルを確認（TVTest等で）

## 開発者向け

### テスト実行

```powershell
# 単体テスト
cargo test

# 統合テスト（実際のBonDriverが必要）
cargo test --test integration -- --ignored
```

### デバッグビルド

```powershell
# デバッグビルド
cargo build

# 詳細ログ付きで実行
.\target\debug\bonrec-rs.exe scan -v C:\BonDriver\BonDriver_PT3-T.dll
```

### ドキュメント生成

```powershell
cargo doc --open
```

## 次のステップ

- スキャン結果を使った録画設定の生成
- mirakurun形式へのエクスポート
- 定期的なスキャンの自動化
