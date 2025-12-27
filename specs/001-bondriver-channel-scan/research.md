# Research: BonDriverチャンネルスキャン

**Date**: 2025-12-01
**Feature**: 001-bondriver-channel-scan

## 1. BonDriver API インターフェース

### 決定事項
BonDriver API (IBonDriver, IBonDriver2, IBonDriver3)を使用してチューナーを制御する。

### 根拠
BonDriverは日本のデジタルTV（ISDB-T/ISDB-S）チューナー向けの標準化されたAPIであり、TVTestやEDCBなど多くのアプリケーションで採用されている。

### API関数シグネチャ

#### IBonDriver (基本インターフェース)
```cpp
class IBonDriver {
public:
    virtual const BOOL OpenTuner(void) = 0;
    virtual void CloseTuner(void) = 0;
    virtual const BOOL SetChannel(const BYTE bCh) = 0;
    virtual const float GetSignalLevel(void) = 0;
    virtual const DWORD WaitTsStream(const DWORD dwTimeOut = 0) = 0;
    virtual const DWORD GetReadyCount(void) = 0;
    virtual const BOOL GetTsStream(BYTE *pDst, DWORD *pdwSize, DWORD *pdwRemain) = 0;
    virtual const BOOL GetTsStream(BYTE **ppDst, DWORD *pdwSize, DWORD *pdwRemain) = 0;
    virtual void PurgeTsStream(void) = 0;
    virtual void Release(void) = 0;
};
```

#### IBonDriver2 (拡張インターフェース) - チャンネルスキャンに必須
```cpp
class IBonDriver2 : public IBonDriver {
public:
    virtual LPCTSTR GetTunerName(void) = 0;
    virtual const BOOL IsTunerOpening(void) = 0;
    virtual LPCTSTR EnumTuningSpace(const DWORD dwSpace) = 0;
    virtual LPCTSTR EnumChannelName(const DWORD dwSpace, const DWORD dwChannel) = 0;
    virtual const BOOL SetChannel(const DWORD dwSpace, const DWORD dwChannel) = 0;
    virtual const DWORD GetCurSpace(void) = 0;
    virtual const DWORD GetCurChannel(void) = 0;
};
```

#### DLLエントリーポイント
```cpp
extern "C" __declspec(dllexport) IBonDriver* CreateBonDriver(void);
```

### 代替案
- 直接チューナーデバイスにアクセス → ドライバー依存性が高く、互換性が低い
- 既存のC++ライブラリをラップ → 依存関係が複雑になる

## 2. Rust FFI / DLL動的ロード

### 決定事項
`libloading`クレートを使用してBonDriver DLLを動的ロードする。

### 根拠
- Rustの標準的なDLL動的ロードライブラリ
- クロスプラットフォーム対応（今回はWindowsのみだが）
- 安全なAPI設計（ライフタイム管理）

### 実装パターン
```rust
use libloading::{Library, Symbol};

// FFI型定義
type BOOL = i32;
type BYTE = u8;
type DWORD = u32;
type LPCTSTR = *const u16; // Wide char string on Windows

// CreateBonDriver関数型
type CreateBonDriverFn = extern "system" fn() -> *mut IBonDriver;

// DLLロード
unsafe {
    let lib = Library::new("BonDriver_XXX.dll")?;
    let create_fn: Symbol<CreateBonDriverFn> = lib.get(b"CreateBonDriver")?;
    let driver = create_fn();
}
```

### 代替案
- `winapi` + 直接LoadLibrary → より低レベルだが手動管理が必要
- `windows-rs` → Microsoft公式だが、この用途には過剰

## 3. MPEG-2 TS パース / SI情報取得

### 決定事項
`mpeg2ts-reader`クレートをベースに、ARIB仕様に対応したSI情報パースを実装する。

### 根拠
- `mpeg2ts-reader`はISO/IEC 13818-1準拠のパーサー
- PAT/PMT/NIT/SDT等の基本的なPSI/SIテーブルに対応
- Zero-copyアクセスで効率的

### SI情報テーブル

| テーブル | PID | 用途 |
|---------|-----|------|
| PAT (Program Association Table) | 0x0000 | プログラム一覧、PMT PID |
| PMT (Program Map Table) | PATで指定 | ストリーム情報 |
| NIT (Network Information Table) | 0x0010 | ネットワーク情報 |
| SDT (Service Description Table) | 0x0011 | サービス名、放送局名 |

### ARIB仕様
- ARIB STD-B10: SI情報の構造と運用規定
- ARIB STD-B24: 文字コード（8単位符号、Shift_JIS互換拡張）

### 代替案
- TSDuck（C++）をFFI経由で使用 → 依存関係が複雑
- 完全自作パーサー → 開発工数が大きい
- 既存Rustクレート + ARIB対応レイヤー → 採用

## 4. SQLiteストレージ

### 決定事項
`rusqlite`クレートを使用してスキャン結果をSQLiteに保存する。

### 根拠
- Rustで最も成熟したSQLiteバインディング
- 同期API（非同期不要なCLIに適合）
- 組み込み可能（外部依存なし）

### スキーマ設計

```sql
-- スキャン実行履歴
CREATE TABLE scan_sessions (
    id INTEGER PRIMARY KEY,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    status TEXT NOT NULL, -- 'running', 'completed', 'interrupted'
    bondriver_paths TEXT NOT NULL -- JSON array
);

-- BonDriverソース
CREATE TABLE bondriver_sources (
    id INTEGER PRIMARY KEY,
    scan_session_id INTEGER NOT NULL,
    dll_path TEXT NOT NULL,
    tuner_name TEXT,
    FOREIGN KEY (scan_session_id) REFERENCES scan_sessions(id)
);

-- チューニングスペース
CREATE TABLE tuning_spaces (
    id INTEGER PRIMARY KEY,
    bondriver_source_id INTEGER NOT NULL,
    space_index INTEGER NOT NULL,
    space_name TEXT NOT NULL,
    FOREIGN KEY (bondriver_source_id) REFERENCES bondriver_sources(id)
);

-- チャンネル
CREATE TABLE channels (
    id INTEGER PRIMARY KEY,
    tuning_space_id INTEGER NOT NULL,
    channel_index INTEGER NOT NULL,
    channel_name TEXT,
    physical_channel INTEGER,
    FOREIGN KEY (tuning_space_id) REFERENCES tuning_spaces(id),
    UNIQUE (tuning_space_id, channel_index)
);

-- サービス
CREATE TABLE services (
    id INTEGER PRIMARY KEY,
    channel_id INTEGER NOT NULL,
    service_id INTEGER NOT NULL,
    network_id INTEGER,
    transport_stream_id INTEGER,
    service_name TEXT,
    broadcaster_name TEXT,
    FOREIGN KEY (channel_id) REFERENCES channels(id)
);

-- チャンネル-BonDriver関連（多対多）
CREATE TABLE channel_sources (
    channel_id INTEGER NOT NULL,
    bondriver_source_id INTEGER NOT NULL,
    PRIMARY KEY (channel_id, bondriver_source_id),
    FOREIGN KEY (channel_id) REFERENCES channels(id),
    FOREIGN KEY (bondriver_source_id) REFERENCES bondriver_sources(id)
);
```

### 代替案
- JSON/YAMLファイル → クエリ性能が低い、大量データに不向き
- 別のRDBMS（PostgreSQL等） → 外部依存が増える
- `sqlx` → 非同期API、マクロ依存が重い

## 5. エラーハンドリングとリソース管理

### 決定事項
- `thiserror`でエラー型を定義
- RAIIパターン（Drop trait）でリソース自動解放
- チューナーはMutexで排他制御（スレッドセーフ）

### 根拠
- BonDriverは内部バッファを持ち、スレッドセーフではない
- チューナーリソースは確実に解放する必要がある
- 明確なエラー型でデバッグ容易性を確保

### 実装パターン
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BonDriverError {
    #[error("Failed to load DLL: {path}")]
    LoadError { path: String, source: libloading::Error },

    #[error("Failed to open tuner")]
    TunerOpenFailed,

    #[error("Channel not found: space={space}, channel={channel}")]
    ChannelNotFound { space: u32, channel: u32 },

    #[error("TS stream timeout after {timeout_ms}ms")]
    StreamTimeout { timeout_ms: u32 },

    #[error("Weak signal: {level}dB")]
    WeakSignal { level: f32 },
}

// RAIIによる自動解放
pub struct TunerGuard {
    driver: BonDriver,
}

impl Drop for TunerGuard {
    fn drop(&mut self) {
        self.driver.close_tuner();
        // BonDriverのDropがRelease()を呼ぶ
    }
}
```

## 6. 進捗表示とユーザー中断

### 決定事項
- `indicatif`クレートでプログレスバー表示
- `ctrlc`クレートでCtrl+C中断をハンドリング

### 根拠
- CLIアプリケーションの標準的なUX
- 長時間のスキャン処理での進捗可視化は必須
- グレースフルシャットダウンでデータ損失を防ぐ

### 代替案
- 標準出力に直接書く → プログレスバーの実装が複雑
- tokio::signal → 非同期ランタイム依存

## 7. 依存クレート一覧

| クレート | バージョン | 用途 |
|---------|-----------|------|
| libloading | 0.8 | DLL動的ロード |
| rusqlite | 0.31 | SQLiteアクセス |
| serde | 1.0 | シリアライズ/デシリアライズ |
| serde_json | 1.0 | JSON出力 |
| clap | 4.4 | CLIパーサー |
| indicatif | 0.17 | プログレスバー |
| ctrlc | 3.4 | Ctrl+Cハンドリング |
| thiserror | 1.0 | エラー型定義 |
| mpeg2ts-reader | 0.18 | MPEG-2 TSパース |
| encoding_rs | 0.8 | ARIB文字コード対応 |

## 参考資料

- [BonDriver Interface Documentation | px4_drv](https://deepwiki.com/tsukumijima/px4_drv/3.3-bondriver-interface)
- [IBonDriver.h Header | TvtPlay GitHub](https://github.com/xtne6f/TvtPlay)
- [recisdb-rs: Rust BonDriver Implementation](https://github.com/kazuki0824/recisdb-rs)
- [libloading Rust Crate Documentation](https://docs.rs/libloading/latest/libloading/)
- [mpeg2ts-reader Crate](https://crates.io/crates/mpeg2ts-reader)
- [ARIB STD-B10 Official Documentation](https://www.arib.or.jp/english/std_tr/broadcasting/)
