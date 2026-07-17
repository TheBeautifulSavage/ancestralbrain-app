/// Folder indexer — walks a directory, extracts metadata + embeddings,
/// stores in SQLite. Resumable: skips files unchanged since last run.
use crate::{db, meta, ollama};
use anyhow::Result;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const AUDIO_EXTS: &[&str] = &[
    "wav", "mp3", "aiff", "aif", "flac", "m4a", "ogg", "opus", "alac",
];
const TEXT_EXTS: &[&str] = &["txt", "md", "markdown"];
const DOC_EXTS: &[&str] = &["pdf"];

pub struct Progress {
    pub total: usize,
    pub done: usize,
    pub skipped: usize,
    pub errors: usize,
}

/// Callback receives (pct: f32, status: String) after each file.
pub fn index_folder<F>(conn: &Connection, folder: &Path, mut on_progress: F) -> Result<Progress>
where
    F: FnMut(f32, String),
{
    let entries: Vec<PathBuf> = WalkDir::new(folder)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect();

    let total = entries.len();
    let mut done = 0usize;
    let mut skipped = 0usize;
    let mut errors = 0usize;

    for path in &entries {
        let path_str = path.to_string_lossy().to_string();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Get mtime + size
        let meta_fs = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => { errors += 1; done += 1; continue; }
        };
        let mtime = meta_fs
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let size = meta_fs.len() as i64;

        // Skip unchanged
        if let Some((db_mtime, db_size)) = db::get_file_state(conn, &path_str) {
            if db_mtime == mtime && db_size == size {
                skipped += 1;
                done += 1;
                let pct = done as f32 / total as f32 * 100.0;
                on_progress(pct, format!("Skipped: {}", path.file_name().unwrap_or_default().to_string_lossy()));
                continue;
            }
        }

        let filename = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Audio metadata
        let (duration_s, sample_rate) = if AUDIO_EXTS.contains(&ext.as_str()) {
            let am = meta::audio_meta(path);
            (am.duration_s, am.sample_rate.map(|r| r as i64))
        } else {
            (None, None)
        };

        let record = db::FileRecord {
            path: path_str.clone(),
            filename: filename.clone(),
            ext: ext.clone(),
            size,
            mtime,
            duration_s,
            sample_rate,
            bpm: None,
            key_sig: None,
        };

        let file_id = match db::upsert_file(conn, &record) {
            Ok(id) => id,
            Err(_) => { errors += 1; done += 1; continue; }
        };
        db::delete_chunks(conn, file_id).ok();

        // Extract + embed text content
        let chunks: Vec<String> = if TEXT_EXTS.contains(&ext.as_str()) {
            meta::text_content(path)
                .map(|t| meta::chunk_text(&t))
                .unwrap_or_default()
        } else if DOC_EXTS.contains(&ext.as_str()) {
            meta::pdf_text(path)
                .map(|t| meta::chunk_text(&t))
                .unwrap_or_default()
        } else if AUDIO_EXTS.contains(&ext.as_str()) {
            // Index filename + path context as a single pseudo-chunk
            vec![format!(
                "Audio file: {} | Path: {}{}",
                filename,
                path_str,
                duration_s.map(|d| format!(" | Duration: {:.0}s", d)).unwrap_or_default()
            )]
        } else {
            // Index filename only for unknown types
            vec![format!("File: {} | Path: {}", filename, path_str)]
        };

        for (seq, chunk_text) in chunks.iter().enumerate() {
            let chunk_id = match db::insert_chunk(conn, file_id, seq as i32, chunk_text) {
                Ok(id) => id,
                Err(_) => continue,
            };
            // Embed — skip silently if Ollama is down (file is still indexed by filename)
            if let Ok(emb) = ollama::embed(chunk_text) {
                db::insert_embedding(conn, chunk_id, &emb).ok();
            }
        }

        done += 1;
        let pct = done as f32 / total as f32 * 100.0;
        on_progress(pct, format!("Indexed: {}", filename));
    }

    Ok(Progress { total, done, skipped, errors })
}
