use crate::core::model::{
    CategoryKey, Distribution, DistributionMode, ExportStatus, PhotoDetail, PhotoRow, Scores,
    ValueStats, CATEGORY_KEYS,
};
use anyhow::{anyhow, Result};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn init(app: &AppHandle) -> Result<Self> {
        let path = app
            .path()
            .app_data_dir()
            .map_err(|e| anyhow!("app data dir: {}", e))?;
        std::fs::create_dir_all(&path)?;
        let db_path = PathBuf::from(path).join("images.db");
        let conn = Connection::open(db_path)?;
        let db = Db { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS photos (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL,
                file_name TEXT NOT NULL,
                category TEXT NOT NULL,
                scores TEXT NOT NULL,
                tags TEXT,
                caption TEXT,
                text_in_image TEXT,
                model TEXT,
                is_valuable INTEGER,
                valuable_score REAL,
                export_status TEXT NOT NULL,
                error_message TEXT,
                analysis_log TEXT,
                analysis_duration_ms INTEGER,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
        ",
        )?;
        self.ensure_column("photos", "analysis_log", "TEXT")?;
        self.ensure_column("photos", "analysis_duration_ms", "INTEGER")?;
        self.ensure_column("photos", "model", "TEXT")?;
        self.ensure_column("photos", "is_valuable", "INTEGER")?;
        self.ensure_column("photos", "valuable_score", "REAL")?;
        Ok(())
    }

    fn ensure_column(&self, table: &str, column: &str, column_type: &str) -> Result<()> {
        let mut stmt = self
            .conn
            .prepare(&format!("PRAGMA table_info({})", table))?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            if name == column {
                return Ok(());
            }
        }
        self.conn.execute(
            &format!(
                "ALTER TABLE {} ADD COLUMN {} {}",
                table, column, column_type
            ),
            [],
        )?;
        Ok(())
    }

    pub fn insert_photo(&self, row: &PhotoDetail) -> Result<()> {
        let scores_json = serde_json::to_string(&row.scores.to_map())?;
        let tags_json = serde_json::to_string(&row.tags)?;
        self.conn.execute(
            "INSERT OR REPLACE INTO photos
            (id, path, file_name, category, scores, tags, caption, text_in_image, model, is_valuable, valuable_score, export_status, error_message, analysis_log, analysis_duration_ms)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                row.id,
                row.path,
                row.file_name,
                row.category.as_str(),
                scores_json,
                tags_json,
                row.caption,
                row.text_in_image,
                row.model,
                row.is_valuable.map(|b| if b { 1 } else { 0 }),
                row.valuable_score,
                export_status_to_str(&row.export_status),
                row.error_message,
                row.analysis_log,
                row.analysis_duration_ms,
            ],
        )?;
        Ok(())
    }

    pub fn list_photos(&self) -> Result<Vec<PhotoRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, file_name, category, scores, tags, export_status, error_message, analysis_duration_ms, model, is_valuable, valuable_score FROM photos ORDER BY created_at DESC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                let scores_map: HashMap<String, f32> =
                    serde_json::from_str(row.get::<_, String>(4)?.as_str()).unwrap_or_default();
                let scores = Scores::from_map(&scores_map);
                let top = scores.top();
                Ok(PhotoRow {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    file_name: row.get(2)?,
                    category: CategoryKey::from(row.get::<_, String>(3)?.as_str()),
                    scores: scores.clone(),
                    top_score: top.1,
                    tags: serde_json::from_str(row.get::<_, String>(5)?.as_str())
                        .unwrap_or_default(),
                    export_status: str_to_export_status(row.get::<_, String>(6)?.as_str()),
                    error_message: row.get(7)?,
                    analysis_duration_ms: row.get(8)?,
                    model: row.get(9)?,
                    is_valuable: row
                        .get::<_, Option<i64>>(10)?
                        .map(|v| v != 0),
                    valuable_score: row.get(11)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_photo_detail(&self, id: &str) -> Result<PhotoDetail> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, file_name, category, scores, tags, export_status, error_message, caption, text_in_image, analysis_log, analysis_duration_ms, model, is_valuable, valuable_score
            FROM photos WHERE id=?1",
        )?;
        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            let scores_map: HashMap<String, f32> =
                serde_json::from_str(row.get::<_, String>(4)?.as_str()).unwrap_or_default();
            let scores = Scores::from_map(&scores_map);
            let top = scores.top();
            return Ok(PhotoDetail {
                id: row.get(0)?,
                path: row.get(1)?,
                file_name: row.get(2)?,
                category: CategoryKey::from(row.get::<_, String>(3)?.as_str()),
                scores: scores.clone(),
                top_score: top.1,
                tags: serde_json::from_str(row.get::<_, String>(5)?.as_str()).unwrap_or_default(),
                export_status: str_to_export_status(row.get::<_, String>(6)?.as_str()),
                error_message: row.get(7)?,
                analysis_log: row.get(10)?,
                analysis_duration_ms: row.get(11)?,
                caption: row.get(8)?,
                text_in_image: row.get(9)?,
                model: row.get(12)?,
                is_valuable: row.get::<_, Option<i64>>(13)?.map(|v| v != 0),
                valuable_score: row.get(14)?,
            });
        }
        Err(anyhow!("not found"))
    }

    pub fn get_value_stats(&self) -> Result<ValueStats> {
        let mut stmt = self.conn.prepare(
            "SELECT
              SUM(CASE WHEN is_valuable = 1 THEN 1 ELSE 0 END) AS valuable,
              SUM(CASE WHEN is_valuable = 0 THEN 1 ELSE 0 END) AS not_valuable,
              SUM(CASE WHEN is_valuable IS NULL THEN 1 ELSE 0 END) AS unknown
            FROM photos",
        )?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let valuable: i64 = row.get(0)?;
            let not_valuable: i64 = row.get(1)?;
            let unknown: i64 = row.get(2)?;
            return Ok(ValueStats {
                valuable: valuable.max(0) as usize,
                not_valuable: not_valuable.max(0) as usize,
                unknown: unknown.max(0) as usize,
            });
        }
        Ok(ValueStats {
            valuable: 0,
            not_valuable: 0,
            unknown: 0,
        })
    }

    pub fn clear_photos(&self) -> Result<()> {
        self.conn.execute("DELETE FROM photos", [])?;
        Ok(())
    }

    pub fn get_distribution(&self, mode: DistributionMode) -> Result<Distribution> {
        let rows = self.list_photos()?;
        let mut by_category: HashMap<String, f32> = CATEGORY_KEYS
            .iter()
            .map(|c| (c.as_str().to_string(), 0.0f32))
            .collect();

        if rows.is_empty() {
            return Ok(Distribution { mode, by_category });
        }

        match mode {
            DistributionMode::CountRatio => {
                for row in rows.iter() {
                    *by_category.get_mut(row.category.as_str()).unwrap() += 1.0;
                }
                let total = rows.len() as f32;
                for val in by_category.values_mut() {
                    *val = (*val / total).round_to(4);
                }
            }
            DistributionMode::AvgScore => {
                for row in rows.iter() {
                    let map = row.scores.to_map();
                    for (k, v) in map {
                        *by_category.get_mut(&k).unwrap() += v;
                    }
                }
                let total = rows.len() as f32;
                for val in by_category.values_mut() {
                    *val = (*val / total).round_to(4);
                }
            }
        }

        Ok(Distribution { mode, by_category })
    }
}

trait Roundable {
    fn round_to(self, digits: u32) -> Self;
}

impl Roundable for f32 {
    fn round_to(self, digits: u32) -> Self {
        let pow = 10f32.powi(digits as i32);
        (self * pow).round() / pow
    }
}

fn export_status_to_str(status: &ExportStatus) -> &'static str {
    match status {
        ExportStatus::Pending => "pending",
        ExportStatus::Success => "success",
        ExportStatus::Error => "error",
    }
}

fn str_to_export_status(raw: &str) -> ExportStatus {
    match raw {
        "success" => ExportStatus::Success,
        "pending" => ExportStatus::Pending,
        "error" => ExportStatus::Error,
        _ => ExportStatus::Error,
    }
}
