//! SQLite database operations

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::Result;

use super::models::{
    BonDriverSource, Channel, ChannelSource, ScanSession, ScanStatus, Service, TuningSpace,
};

/// Database schema version
const SCHEMA_VERSION: i32 = 1;

/// SQLite database connection wrapper
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create a database at the specified path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Database { conn };
        db.initialize()?;
        Ok(db)
    }

    /// Open an in-memory database (for testing)
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Database { conn };
        db.initialize()?;
        Ok(db)
    }

    /// Initialize database schema
    fn initialize(&self) -> Result<()> {
        // Check if schema_version table exists
        let table_exists: bool = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_version'",
                [],
                |row| row.get::<_, i32>(0).map(|count| count > 0),
            )?;

        if !table_exists {
            // New database, create schema
            return self.create_schema();
        }

        // Check schema version
        let version: Option<i32> = self
            .conn
            .query_row(
                "SELECT version FROM schema_version LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;

        match version {
            Some(v) if v >= SCHEMA_VERSION => {
                // Schema is up to date
                Ok(())
            }
            _ => {
                // Upgrade schema
                self.create_schema()
            }
        }
    }

    /// Create database schema
    fn create_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            -- Schema version tracking
            CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY
            );

            -- Scan sessions
            CREATE TABLE IF NOT EXISTS scan_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                status TEXT NOT NULL CHECK (status IN ('running', 'completed', 'interrupted'))
            );

            -- BonDriver sources
            CREATE TABLE IF NOT EXISTS bondriver_sources (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                scan_session_id INTEGER NOT NULL,
                dll_path TEXT NOT NULL,
                tuner_name TEXT,
                FOREIGN KEY (scan_session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_bondriver_sources_session
                ON bondriver_sources(scan_session_id);

            -- Tuning spaces
            CREATE TABLE IF NOT EXISTS tuning_spaces (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                bondriver_source_id INTEGER NOT NULL,
                space_index INTEGER NOT NULL,
                space_name TEXT NOT NULL,
                FOREIGN KEY (bondriver_source_id) REFERENCES bondriver_sources(id) ON DELETE CASCADE,
                UNIQUE (bondriver_source_id, space_index)
            );

            CREATE INDEX IF NOT EXISTS idx_tuning_spaces_bondriver
                ON tuning_spaces(bondriver_source_id);

            -- Channels
            CREATE TABLE IF NOT EXISTS channels (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tuning_space_id INTEGER NOT NULL,
                channel_index INTEGER NOT NULL,
                channel_name TEXT,
                physical_channel INTEGER,
                FOREIGN KEY (tuning_space_id) REFERENCES tuning_spaces(id) ON DELETE CASCADE,
                UNIQUE (tuning_space_id, channel_index)
            );

            CREATE INDEX IF NOT EXISTS idx_channels_tuning_space
                ON channels(tuning_space_id);

            -- Services
            CREATE TABLE IF NOT EXISTS services (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                channel_id INTEGER NOT NULL,
                service_id INTEGER NOT NULL,
                network_id INTEGER,
                transport_stream_id INTEGER,
                service_name TEXT,
                broadcaster_name TEXT,
                FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_services_channel
                ON services(channel_id);

            -- Channel sources (many-to-many)
            CREATE TABLE IF NOT EXISTS channel_sources (
                channel_id INTEGER NOT NULL,
                bondriver_source_id INTEGER NOT NULL,
                PRIMARY KEY (channel_id, bondriver_source_id),
                FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE,
                FOREIGN KEY (bondriver_source_id) REFERENCES bondriver_sources(id) ON DELETE CASCADE
            );

            -- Insert or update schema version
            INSERT OR REPLACE INTO schema_version (version) VALUES (1);
            "#,
        )?;

        Ok(())
    }

    // ========== ScanSession CRUD ==========

    /// Create a new scan session
    pub fn create_scan_session(&self, session: &mut ScanSession) -> Result<()> {
        self.conn.execute(
            "INSERT INTO scan_sessions (started_at, completed_at, status) VALUES (?1, ?2, ?3)",
            params![
                session.started_at.to_rfc3339(),
                session.completed_at.map(|dt| dt.to_rfc3339()),
                session.status.as_str(),
            ],
        )?;
        session.id = self.conn.last_insert_rowid();
        Ok(())
    }

    /// Update scan session
    pub fn update_scan_session(&self, session: &ScanSession) -> Result<()> {
        self.conn.execute(
            "UPDATE scan_sessions SET started_at = ?1, completed_at = ?2, status = ?3 WHERE id = ?4",
            params![
                session.started_at.to_rfc3339(),
                session.completed_at.map(|dt| dt.to_rfc3339()),
                session.status.as_str(),
                session.id,
            ],
        )?;
        Ok(())
    }

    /// Get scan session by ID
    pub fn get_scan_session(&self, id: i64) -> Result<Option<ScanSession>> {
        let result = self
            .conn
            .query_row(
                "SELECT id, started_at, completed_at, status FROM scan_sessions WHERE id = ?1",
                params![id],
                |row| {
                    let started_at: String = row.get(1)?;
                    let completed_at: Option<String> = row.get(2)?;
                    let status_str: String = row.get(3)?;

                    Ok(ScanSession {
                        id: row.get(0)?,
                        started_at: chrono::DateTime::parse_from_rfc3339(&started_at)
                            .unwrap()
                            .with_timezone(&chrono::Utc),
                        completed_at: completed_at.map(|s| {
                            chrono::DateTime::parse_from_rfc3339(&s)
                                .unwrap()
                                .with_timezone(&chrono::Utc)
                        }),
                        status: ScanStatus::from_str(&status_str).unwrap_or(ScanStatus::Running),
                    })
                },
            )
            .optional()?;
        Ok(result)
    }

    // ========== BonDriverSource CRUD ==========

    /// Create a new BonDriver source
    pub fn create_bondriver_source(&self, source: &mut BonDriverSource) -> Result<()> {
        self.conn.execute(
            "INSERT INTO bondriver_sources (scan_session_id, dll_path, tuner_name) VALUES (?1, ?2, ?3)",
            params![source.scan_session_id, source.dll_path, source.tuner_name],
        )?;
        source.id = self.conn.last_insert_rowid();
        Ok(())
    }

    /// Get BonDriver sources for a scan session
    pub fn get_bondriver_sources(&self, session_id: i64) -> Result<Vec<BonDriverSource>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, scan_session_id, dll_path, tuner_name FROM bondriver_sources WHERE scan_session_id = ?1",
        )?;

        let sources = stmt
            .query_map(params![session_id], |row| {
                Ok(BonDriverSource {
                    id: row.get(0)?,
                    scan_session_id: row.get(1)?,
                    dll_path: row.get(2)?,
                    tuner_name: row.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(sources)
    }

    // ========== TuningSpace CRUD ==========

    /// Create a new tuning space
    pub fn create_tuning_space(&self, space: &mut TuningSpace) -> Result<()> {
        self.conn.execute(
            "INSERT INTO tuning_spaces (bondriver_source_id, space_index, space_name) VALUES (?1, ?2, ?3)",
            params![space.bondriver_source_id, space.space_index, space.space_name],
        )?;
        space.id = self.conn.last_insert_rowid();
        Ok(())
    }

    /// Get tuning spaces for a BonDriver source
    pub fn get_tuning_spaces(&self, bondriver_source_id: i64) -> Result<Vec<TuningSpace>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, bondriver_source_id, space_index, space_name FROM tuning_spaces WHERE bondriver_source_id = ?1",
        )?;

        let spaces = stmt
            .query_map(params![bondriver_source_id], |row| {
                Ok(TuningSpace {
                    id: row.get(0)?,
                    bondriver_source_id: row.get(1)?,
                    space_index: row.get(2)?,
                    space_name: row.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(spaces)
    }

    // ========== Channel CRUD ==========

    /// Create a new channel
    pub fn create_channel(&self, channel: &mut Channel) -> Result<()> {
        self.conn.execute(
            "INSERT INTO channels (tuning_space_id, channel_index, channel_name, physical_channel) VALUES (?1, ?2, ?3, ?4)",
            params![
                channel.tuning_space_id,
                channel.channel_index,
                channel.channel_name,
                channel.physical_channel,
            ],
        )?;
        channel.id = self.conn.last_insert_rowid();
        Ok(())
    }

    /// Get channels for a tuning space
    pub fn get_channels(&self, tuning_space_id: i64) -> Result<Vec<Channel>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tuning_space_id, channel_index, channel_name, physical_channel FROM channels WHERE tuning_space_id = ?1",
        )?;

        let channels = stmt
            .query_map(params![tuning_space_id], |row| {
                Ok(Channel {
                    id: row.get(0)?,
                    tuning_space_id: row.get(1)?,
                    channel_index: row.get(2)?,
                    channel_name: row.get(3)?,
                    physical_channel: row.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(channels)
    }

    // ========== Service CRUD ==========

    /// Create a new service
    pub fn create_service(&self, service: &mut Service) -> Result<()> {
        self.conn.execute(
            "INSERT INTO services (channel_id, service_id, network_id, transport_stream_id, service_name, broadcaster_name) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                service.channel_id,
                service.service_id,
                service.network_id,
                service.transport_stream_id,
                service.service_name,
                service.broadcaster_name,
            ],
        )?;
        service.id = self.conn.last_insert_rowid();
        Ok(())
    }

    /// Get services for a channel
    pub fn get_services(&self, channel_id: i64) -> Result<Vec<Service>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, channel_id, service_id, network_id, transport_stream_id, service_name, broadcaster_name FROM services WHERE channel_id = ?1",
        )?;

        let services = stmt
            .query_map(params![channel_id], |row| {
                Ok(Service {
                    id: row.get(0)?,
                    channel_id: row.get(1)?,
                    service_id: row.get(2)?,
                    network_id: row.get(3)?,
                    transport_stream_id: row.get(4)?,
                    service_name: row.get(5)?,
                    broadcaster_name: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(services)
    }

    // ========== ChannelSource CRUD ==========

    /// Create a channel-source relation
    pub fn create_channel_source(&self, relation: &ChannelSource) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO channel_sources (channel_id, bondriver_source_id) VALUES (?1, ?2)",
            params![relation.channel_id, relation.bondriver_source_id],
        )?;
        Ok(())
    }

    /// Get BonDriver sources for a channel
    pub fn get_channel_sources(&self, channel_id: i64) -> Result<Vec<i64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT bondriver_source_id FROM channel_sources WHERE channel_id = ?1")?;

        let sources = stmt
            .query_map(params![channel_id], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(sources)
    }

    /// Get BonDriver paths for a channel
    pub fn get_channel_source_paths(&self, channel_id: i64) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT bs.dll_path
            FROM channel_sources cs
            JOIN bondriver_sources bs ON cs.bondriver_source_id = bs.id
            WHERE cs.channel_id = ?1
            "#,
        )?;

        let paths = stmt
            .query_map(params![channel_id], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(paths)
    }

    // ========== Export Queries ==========

    /// Get the latest scan session
    pub fn get_latest_scan_session(&self) -> Result<Option<ScanSession>> {
        let result = self
            .conn
            .query_row(
                "SELECT id, started_at, completed_at, status FROM scan_sessions ORDER BY id DESC LIMIT 1",
                [],
                |row| {
                    let started_at: String = row.get(1)?;
                    let completed_at: Option<String> = row.get(2)?;
                    let status_str: String = row.get(3)?;

                    Ok(ScanSession {
                        id: row.get(0)?,
                        started_at: chrono::DateTime::parse_from_rfc3339(&started_at)
                            .unwrap()
                            .with_timezone(&chrono::Utc),
                        completed_at: completed_at.map(|s| {
                            chrono::DateTime::parse_from_rfc3339(&s)
                                .unwrap()
                                .with_timezone(&chrono::Utc)
                        }),
                        status: ScanStatus::from_str(&status_str).unwrap_or(ScanStatus::Running),
                    })
                },
            )
            .optional()?;
        Ok(result)
    }

    /// Get all channels with their services for a scan session
    pub fn get_session_channels(&self, session_id: i64) -> Result<Vec<ExportedChannel>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT
                c.id,
                ts.space_name,
                c.channel_index,
                c.channel_name,
                c.physical_channel
            FROM channels c
            JOIN tuning_spaces ts ON c.tuning_space_id = ts.id
            JOIN bondriver_sources bs ON ts.bondriver_source_id = bs.id
            WHERE bs.scan_session_id = ?1
            ORDER BY ts.space_name, c.channel_index
            "#,
        )?;

        let channels: Vec<(i64, String, u32, Option<String>, Option<u32>)> = stmt
            .query_map(params![session_id], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut result = Vec::new();
        for (channel_id, space_name, channel_index, channel_name, physical_channel) in channels {
            // Get services for this channel
            let services = self.get_services(channel_id)?;

            // Get available sources
            let available_from = self.get_channel_source_paths(channel_id)?;

            result.push(ExportedChannel {
                tuning_space: space_name,
                channel_index,
                channel_name,
                physical_channel,
                services,
                available_from,
            });
        }

        Ok(result)
    }
}

/// Exported channel data (for export functionality)
#[derive(Debug, Clone)]
pub struct ExportedChannel {
    pub tuning_space: String,
    pub channel_index: u32,
    pub channel_name: Option<String>,
    pub physical_channel: Option<u32>,
    pub services: Vec<Service>,
    pub available_from: Vec<String>,
}
