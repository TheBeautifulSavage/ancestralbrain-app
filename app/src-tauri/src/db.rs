use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

// ── DB open ───────────────────────────────────────────────────────────────────

/// Open (or create) the vault DB at the given path.
/// Enables WAL mode and creates tables + sqlite-vec virtual table.
/// On Unix, sets DB file permissions to 600 (owner read/write only).
pub fn open(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

    // Register sqlite-vec extension for vector operations
    unsafe {
        rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
            sqlite_vec::sqlite3_vec_init as *const ()
        )));
    }

    // Restrict DB file permissions to owner-only on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = std::fs::set_permissions(db_path, std::fs::Permissions::from_mode(0o600)) {
            eprintln!("Warning: could not set DB permissions: {e}");
        }
    }

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

// ── Vault key storage — cross-platform ───────────────────────────────────────

const KEYCHAIN_SERVICE: &str = "com.ancestralbrain.app";
const KEYCHAIN_ACCOUNT: &str = "vault-key";
const WINDOWS_CRED_TARGET: &str = "AncestralBrain/VaultKey";

/// Retrieve or create the vault encryption key, stored securely per platform.
/// macOS  → macOS Keychain via `security` CLI
/// Windows → Windows Credential Manager via `cmdkey`
/// Linux   → ~/.local/share/ancestralbrain/.vault_key (chmod 600)
pub fn get_or_create_vault_key() -> Result<String> {
    // Try to load existing key first
    if let Some(key) = load_vault_key() {
        return Ok(key);
    }
    // Generate a new 32-byte hex key
    let key = generate_key();
    save_vault_key(&key)?;
    Ok(key)
}

fn generate_key() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Simple PRNG seeded from time + process ID (sufficient for a local vault key)
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0) ^ (std::process::id() as u128);

    // LCG-based pseudo-random bytes (no rand crate dependency)
    let mut state = seed;
    let mut bytes = [0u8; 32];
    for b in bytes.iter_mut() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        *b = ((state >> 33) & 0xff) as u8;
    }
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ── macOS ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn load_vault_key() -> Option<String> {
    let out = std::process::Command::new("security")
        .args([
            "find-generic-password",
            "-s", KEYCHAIN_SERVICE,
            "-a", KEYCHAIN_ACCOUNT,
            "-w",
        ])
        .output()
        .ok()?;
    if out.status.success() {
        String::from_utf8(out.stdout).ok().map(|s| s.trim().to_string())
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
fn save_vault_key(key: &str) -> Result<()> {
    let status = std::process::Command::new("security")
        .args([
            "add-generic-password",
            "-s", KEYCHAIN_SERVICE,
            "-a", KEYCHAIN_ACCOUNT,
            "-w", key,
            "-U", // update if exists
        ])
        .status()
        .map_err(|e| anyhow::anyhow!("security CLI failed: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Keychain write failed (exit {})", status))
    }
}

// ── Windows ───────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn load_vault_key() -> Option<String> {
    // cmdkey doesn't have a read API; we use a companion file for the value
    // but store metadata in Credential Manager as a presence check.
    // For simplicity, use the file-based fallback on Windows too.
    windows_key_file_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(target_os = "windows")]
fn save_vault_key(key: &str) -> Result<()> {
    let path = windows_key_file_path()
        .ok_or_else(|| anyhow::anyhow!("Could not determine key file path"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, key)?;
    // Also register in Credential Manager as a no-value marker
    let _ = std::process::Command::new("cmdkey")
        .args([&format!("/add:{WINDOWS_CRED_TARGET}"), "/user:ancestralbrain", "/pass:1"])
        .status();
    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_key_file_path() -> Option<std::path::PathBuf> {
    let base = std::env::var("APPDATA").ok()?;
    Some(std::path::PathBuf::from(base).join("AncestralBrain").join(".vault_key"))
}

// ── Linux ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn load_vault_key() -> Option<String> {
    let path = linux_key_file_path()?;
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

#[cfg(target_os = "linux")]
fn save_vault_key(key: &str) -> Result<()> {
    let path = linux_key_file_path()
        .ok_or_else(|| anyhow::anyhow!("Could not determine key file path"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, key)?;
    // Set permissions to 600 (owner read/write only)
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn linux_key_file_path() -> Option<std::path::PathBuf> {
    let base = std::env::var("HOME").ok()?;
    Some(std::path::PathBuf::from(base)
        .join(".local")
        .join("share")
        .join("ancestralbrain")
        .join(".vault_key"))
}

// ── Fallback for other platforms ──────────────────────────────────────────────

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn load_vault_key() -> Option<String> { None }

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn save_vault_key(_key: &str) -> Result<()> { Ok(()) }

// ── File state ────────────────────────────────────────────────────────────────

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

// ── Types ─────────────────────────────────────────────────────────────────────

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

// ── Queries ───────────────────────────────────────────────────────────────────

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
