use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

/// Open (or create) the vault DB at the given path.
/// Enables WAL mode and creates tables + sqlite-vec virtual table.
pub fn open(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    sqlite_vec::load(&conn)?;
    create_schema(&conn)?;
    Ok(conn)
}

fn create_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS files (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            path        TEXT    NOT NULL UNIQUE,
            filename    TEXT    NOT NULL,
            ext         TEXT    NOT NULL DEFAULT '',
            size        INTEGER NOT NULL DEFAULT 0,
            mtime       INTEGER NOT NULL DEFAULT 0,
            duration_s  REAL,
            sample_rate INTEGER,
            bpm         REAL,
            key_sig     TEXT,
            indexed_at  INTEGER NOT NULL DEFAULT (unixepoch())
        );

        CREATE TABLE IF NOT EXISTS chunks (
            id      INTEGER PRIMARY KEY AUTOINCREMENT,
            file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
            seq     INTEGER NOT NULL DEFAULT 0,
            text    TEXT    NOT NULL
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS chunk_embeddings
            USING vec0(embedding FLOAT[768]);
    ")?;
    Ok(())
}

/// Returns (mtime, size) for a known file, or None if not indexed.
pub fn get_file_state(conn: &Connection, path: &str) -> Option<(i64, i64)> {
    conn.query_row(
        "SELECT mtime, size FROM files WHERE path = ?1",
        [path],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .ok()
}

pub fn upsert_file(conn: &Connection, f: &FileRecord) -> Result<i64> {
    conn.execute(
        "INSERT INTO files (path, filename, ext, size, mtime, duration_s, sample_rate, bpm, key_sig)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)
         ON CONFLICT(path) DO UPDATE SET
           filename=excluded.filename, ext=excluded.ext, size=excluded.size,
           mtime=excluded.mtime, duration_s=excluded.duration_s,
           sample_rate=excluded.sample_rate, bpm=excluded.bpm, key_sig=excluded.key_sig,
           indexed_at=unixepoch()",
        rusqlite::params![
            f.path, f.filename, f.ext, f.size, f.mtime,
            f.duration_s, f.sample_rate, f.bpm, f.key_sig,
        ],
    )?;
    let id: i64 = conn.query_row(
        "SELECT id FROM files WHERE path = ?1",
        [&f.path],
        |r| r.get(0),
    )?;
    Ok(id)
}

pub fn delete_chunks(conn: &Connection, file_id: i64) -> Result<()> {
    // Delete embeddings for this file's chunks
    conn.execute(
        "DELETE FROM chunk_embeddings WHERE rowid IN (
            SELECT id FROM chunks WHERE file_id = ?1
         )",
        [file_id],
    )?;
    conn.execute("DELETE FROM chunks WHERE file_id = ?1", [file_id])?;
    Ok(())
}

pub fn insert_chunk(conn: &Connection, file_id: i64, seq: i32, text: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO chunks (file_id, seq, text) VALUES (?1, ?2, ?3)",
        rusqlite::params![file_id, seq, text],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn insert_embedding(conn: &Connection, chunk_id: i64, embedding: &[f32]) -> Result<()> {
    let blob: Vec<u8> = embedding
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect();
    conn.execute(
        "INSERT INTO chunk_embeddings(rowid, embedding) VALUES (?1, ?2)",
        rusqlite::params![chunk_id, blob],
    )?;
    Ok(())
}

#[derive(Debug)]
pub struct FileRecord {
    pub path: String,
    pub filename: String,
    pub ext: String,
    pub size: i64,
    pub mtime: i64,
    pub duration_s: Option<f64>,
    pub sample_rate: Option<i64>,
    pub bpm: Option<f64>,
    pub key_sig: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct SearchResult {
    pub path: String,
    pub filename: String,
    pub score: f32,
    pub snippet: String,
}

/// Filename substring search — returns up to `limit` results.
pub fn filename_search(conn: &Connection, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
    let pattern = format!("%{}%", query.to_lowercase());
    let mut stmt = conn.prepare(
        "SELECT f.path, f.filename, c.text
         FROM files f
         LEFT JOIN chunks c ON c.file_id = f.id AND c.seq = 0
         WHERE lower(f.filename) LIKE ?1
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(rusqlite::params![pattern, limit as i64], |row| {
        Ok(SearchResult {
            path: row.get(0)?,
            filename: row.get(1)?,
            score: 0.5,
            snippet: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Semantic search via sqlite-vec ANN — returns up to `limit` results.
pub fn semantic_search(
    conn: &Connection,
    embedding: &[f32],
    limit: usize,
) -> Result<Vec<SearchResult>> {
    let blob: Vec<u8> = embedding
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect();
    let mut stmt = conn.prepare(
        "SELECT f.path, f.filename, c.text, vec_distance_cosine(ce.embedding, ?1) AS dist
         FROM chunk_embeddings ce
         JOIN chunks c ON c.id = ce.rowid
         JOIN files f ON f.id = c.file_id
         ORDER BY dist ASC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(rusqlite::params![blob, limit as i64], |row| {
        let dist: f32 = row.get(3)?;
        Ok(SearchResult {
            path: row.get(0)?,
            filename: row.get(1)?,
            score: 1.0 - dist,
            snippet: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}
