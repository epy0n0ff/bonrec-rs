# Data Model: BonDriverチャンネルスキャン

**Date**: 2025-12-01
**Feature**: 001-bondriver-channel-scan

## エンティティ関係図

```
┌─────────────────┐
│  ScanSession    │
│─────────────────│
│ id              │
│ started_at      │
│ completed_at    │
│ status          │
└────────┬────────┘
         │ 1:N
         ▼
┌─────────────────┐
│ BonDriverSource │
│─────────────────│
│ id              │
│ dll_path        │
│ tuner_name      │
└────────┬────────┘
         │ 1:N
         ▼
┌─────────────────┐      ┌─────────────────┐
│  TuningSpace    │      │ ChannelSource   │
│─────────────────│      │ (多対多関連)    │
│ id              │      │─────────────────│
│ space_index     │      │ channel_id      │
│ space_name      │      │ bondriver_id    │
└────────┬────────┘      └─────────────────┘
         │ 1:N                   ▲
         ▼                       │
┌─────────────────┐              │
│    Channel      │──────────────┘
│─────────────────│
│ id              │
│ channel_index   │
│ channel_name    │
│ physical_ch     │
└────────┬────────┘
         │ 1:N
         ▼
┌─────────────────┐
│    Service      │
│─────────────────│
│ id              │
│ service_id      │
│ network_id      │
│ ts_id           │
│ service_name    │
│ broadcaster     │
└─────────────────┘
```

## エンティティ詳細

### ScanSession（スキャンセッション）

スキャン実行の1回分を表す。

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| id | i64 | ✓ | 主キー |
| started_at | DateTime | ✓ | スキャン開始日時 |
| completed_at | DateTime | | スキャン完了日時（中断時はNULL） |
| status | ScanStatus | ✓ | 状態（Running/Completed/Interrupted） |

**状態遷移**:
```
Created → Running → Completed
              ↓
         Interrupted
```

**検証ルール**:
- started_atは現在日時以前
- completed_atが設定される場合、started_at以降であること

### BonDriverSource（BonDriverソース）

使用されたBonDriver DLLを表す。

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| id | i64 | ✓ | 主キー |
| scan_session_id | i64 | ✓ | スキャンセッションへの参照 |
| dll_path | String | ✓ | DLLファイルパス |
| tuner_name | String | | チューナー名（GetTunerNameから取得） |

**検証ルール**:
- dll_pathはファイルシステム上に存在するパス
- dll_pathは`.dll`拡張子を持つ

### TuningSpace（チューニングスペース）

放送波の種類（地上波、BS、CS等）を表す。

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| id | i64 | ✓ | 主キー |
| bondriver_source_id | i64 | ✓ | BonDriverソースへの参照 |
| space_index | u32 | ✓ | BonDriverでのスペースインデックス |
| space_name | String | ✓ | スペース名（EnumTuningSpaceから取得） |

**一意性**:
- (bondriver_source_id, space_index) の組み合わせは一意

### Channel（チャンネル）

物理チャンネルを表す。

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| id | i64 | ✓ | 主キー |
| tuning_space_id | i64 | ✓ | チューニングスペースへの参照 |
| channel_index | u32 | ✓ | BonDriverでのチャンネルインデックス |
| channel_name | String | | チャンネル名（EnumChannelNameから取得） |
| physical_channel | u32 | | 物理チャンネル番号 |

**一意性**:
- (tuning_space_id, channel_index) の組み合わせは一意

### Service（サービス）

1チャンネル内の放送サービスを表す。

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| id | i64 | ✓ | 主キー |
| channel_id | i64 | ✓ | チャンネルへの参照 |
| service_id | u16 | ✓ | サービスID（MPEG-2 program_number） |
| network_id | u16 | | ネットワークID（NITから） |
| transport_stream_id | u16 | | トランスポートストリームID |
| service_name | String | | サービス名（SDTから） |
| broadcaster_name | String | | 放送局名（SDTから） |

**検証ルール**:
- service_idは0-65535の範囲

### ChannelSource（チャンネルソース関連）

チャンネルと取得元BonDriverの多対多関連。

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| channel_id | i64 | ✓ | チャンネルへの参照 |
| bondriver_source_id | i64 | ✓ | BonDriverソースへの参照 |

**一意性**:
- (channel_id, bondriver_source_id) の組み合わせは一意

## Rust型定義

```rust
// 列挙型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanStatus {
    Running,
    Completed,
    Interrupted,
}

// スキャンセッション
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSession {
    pub id: i64,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub status: ScanStatus,
}

// BonDriverソース
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BonDriverSource {
    pub id: i64,
    pub scan_session_id: i64,
    pub dll_path: String,
    pub tuner_name: Option<String>,
}

// チューニングスペース
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuningSpace {
    pub id: i64,
    pub bondriver_source_id: i64,
    pub space_index: u32,
    pub space_name: String,
}

// チャンネル
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: i64,
    pub tuning_space_id: i64,
    pub channel_index: u32,
    pub channel_name: Option<String>,
    pub physical_channel: Option<u32>,
}

// サービス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: i64,
    pub channel_id: i64,
    pub service_id: u16,
    pub network_id: Option<u16>,
    pub transport_stream_id: Option<u16>,
    pub service_name: Option<String>,
    pub broadcaster_name: Option<String>,
}

// 出力用の統合型（重複排除後）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedChannel {
    pub tuning_space: String,
    pub channel_index: u32,
    pub channel_name: Option<String>,
    pub physical_channel: Option<u32>,
    pub services: Vec<ScannedService>,
    pub available_from: Vec<String>, // BonDriver DLLパス一覧
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedService {
    pub service_id: u16,
    pub network_id: Option<u16>,
    pub transport_stream_id: Option<u16>,
    pub service_name: Option<String>,
    pub broadcaster_name: Option<String>,
}

// スキャン結果全体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub scan_session_id: i64,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub status: ScanStatus,
    pub channels: Vec<ScannedChannel>,
}
```

## SQLiteスキーマ

```sql
-- スキャンセッション
CREATE TABLE scan_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    status TEXT NOT NULL CHECK (status IN ('running', 'completed', 'interrupted'))
);

-- BonDriverソース
CREATE TABLE bondriver_sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    scan_session_id INTEGER NOT NULL,
    dll_path TEXT NOT NULL,
    tuner_name TEXT,
    FOREIGN KEY (scan_session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
);

CREATE INDEX idx_bondriver_sources_session ON bondriver_sources(scan_session_id);

-- チューニングスペース
CREATE TABLE tuning_spaces (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bondriver_source_id INTEGER NOT NULL,
    space_index INTEGER NOT NULL,
    space_name TEXT NOT NULL,
    FOREIGN KEY (bondriver_source_id) REFERENCES bondriver_sources(id) ON DELETE CASCADE,
    UNIQUE (bondriver_source_id, space_index)
);

CREATE INDEX idx_tuning_spaces_bondriver ON tuning_spaces(bondriver_source_id);

-- チャンネル
CREATE TABLE channels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tuning_space_id INTEGER NOT NULL,
    channel_index INTEGER NOT NULL,
    channel_name TEXT,
    physical_channel INTEGER,
    FOREIGN KEY (tuning_space_id) REFERENCES tuning_spaces(id) ON DELETE CASCADE,
    UNIQUE (tuning_space_id, channel_index)
);

CREATE INDEX idx_channels_tuning_space ON channels(tuning_space_id);

-- サービス
CREATE TABLE services (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    channel_id INTEGER NOT NULL,
    service_id INTEGER NOT NULL,
    network_id INTEGER,
    transport_stream_id INTEGER,
    service_name TEXT,
    broadcaster_name TEXT,
    FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE
);

CREATE INDEX idx_services_channel ON services(channel_id);

-- チャンネルソース関連（重複検出時の情報保持用）
CREATE TABLE channel_sources (
    channel_id INTEGER NOT NULL,
    bondriver_source_id INTEGER NOT NULL,
    PRIMARY KEY (channel_id, bondriver_source_id),
    FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE,
    FOREIGN KEY (bondriver_source_id) REFERENCES bondriver_sources(id) ON DELETE CASCADE
);
```

## JSON出力フォーマット

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
          "network_id": 32736,
          "transport_stream_id": 32736,
          "service_name": "NHK総合1・東京",
          "broadcaster_name": "NHK"
        },
        {
          "service_id": 1025,
          "network_id": 32736,
          "transport_stream_id": 32736,
          "service_name": "NHK総合2・東京",
          "broadcaster_name": "NHK"
        }
      ],
      "available_from": [
        "C:\\BonDriver\\BonDriver_PT3-T0.dll",
        "C:\\BonDriver\\BonDriver_PT3-T1.dll"
      ]
    }
  ]
}
```
