/// Folder indexer — walks a directory, extracts metadata + embeddings,
/// stores in SQLite. Resumable: skips files unchanged since last run.
/// Also writes AI-formatted markdown to `_ancestral_brain/` inside the vault.
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

/// Write a companion markdown file for the indexed file into `_ancestral_brain/`.
fn write_markdown(
    vault_root: &Path,
    file_path: &Path,
    filename: &str,
    ext: &str,
    size: i64,
    mtime: i64,
    duration_s: Option<f64>,
    sample_rate: Option<i64>,
    text_preview: Option<&str>,
    ai_summary: &str,
) {
    // Relative path from vault root
    let rel = match file_path.strip_prefix(vault_root) {
        Ok(r) => r,
        Err(_) => return,
    };

    // Output path: <vault_root>/_ancestral_brain/<rel>.md
    let md_path = vault_root
        .join("_ancestral_brain")
        .join(format!("{}.md", rel.to_string_lossy()));

    // Create parent dirs
    if let Some(parent) = md_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    // Determine file type label
    let type_label = if AUDIO_EXTS.contains(&ext) {
        format!("Audio ({})", ext.to_uppercase())
    } else if TEXT_EXTS.contains(&ext) {
        format!("Text ({})", ext.to_uppercase())
    } else if DOC_EXTS.contains(&ext) {
        "PDF Document".to_string()
    } else if ext.is_empty() {
        "Unknown".to_string()
    } else {
        format!("File ({})", ext.to_uppercase())
    };

    // Format size
    let size_human = if size >= 1_073_741_824 {
        format!("{:.1} GB", size as f64 / 1_073_741_824.0)
    } else if size >= 1_048_576 {
        format!("{:.1} MB", size as f64 / 1_048_576.0)
    } else if size >= 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else {
        format!("{} B", size)
    };

    // Format mtime as date string
    let modified_str = format_unix_date(mtime);

    // Indexed date (today)
    let indexed_date = {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        format_unix_date(now)
    };

    let folder_str = file_path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    // Build duration string
    let duration_str = duration_s.map(|d| {
        let secs = d as u64;
        let m = secs / 60;
        let s = secs % 60;
        if m > 0 { format!("{}m {}s", m, s) } else { format!("{}s", s) }
    });

    // YAML frontmatter
    let mut md = format!(
        "---\ntitle: \"{filename}\"\npath: \"{}\"\ntype: {}\next: {ext}\nsize_bytes: {size}\nmodified: {modified_str}\nindexed: {indexed_date}\n",
        file_path.to_string_lossy(),
        if AUDIO_EXTS.contains(&ext) { "audio" }
        else if TEXT_EXTS.contains(&ext) { "text" }
        else if DOC_EXTS.contains(&ext) { "document" }
        else { "file" },
    );
    if let Some(d) = duration_s {
        md.push_str(&format!("duration_s: {:.1}\n", d));
    }
    if let Some(sr) = sample_rate {
        md.push_str(&format!("sample_rate: {}\n", sr));
    }
    md.push_str("---\n\n");

    // Main content
    md.push_str(&format!("# {filename}\n\n"));
    md.push_str(&format!("**Type:** {type_label}\n"));
    md.push_str(&format!(
        "**Path:** {}\n",
        file_path.to_string_lossy()
    ));
    md.push_str(&format!("**Folder:** {folder_str}\n"));
    md.push_str(&format!("**Size:** {size_human}\n"));
    if let Some(dur) = &duration_str {
        md.push_str(&format!("**Duration:** {dur}\n"));
    }
    if let Some(sr) = sample_rate {
        md.push_str(&format!("**Sample Rate:** {} Hz\n", sr));
    }
    md.push_str(&format!("**Modified:** {modified_str}\n"));

    // AI Summary
    if !ai_summary.is_empty() {
        md.push_str("\n## AI Summary\n");
        md.push_str(ai_summary);
        md.push('\n');
    }

    // Content preview for text files
    if let Some(preview) = text_preview {
        md.push_str("\n## Content Preview\n\n");
        let truncated: String = preview.chars().take(500).collect();
        md.push_str(&truncated);
        if preview.len() > 500 {
            md.push_str("\n\n_[truncated]_");
        }
        md.push('\n');
    }

    // Tags
    md.push_str("\n## Tags\n\n");
    if !ext.is_empty() {
        md.push_str(&format!("`#{}` ", ext));
    }
    if AUDIO_EXTS.contains(&ext) {
        md.push_str("`#audio` ");
    } else if TEXT_EXTS.contains(&ext) {
        md.push_str("`#text` ");
    } else if DOC_EXTS.contains(&ext) {
        md.push_str("`#document` ");
    }
    md.push('\n');

    std::fs::write(&md_path, md).ok();
}

/// Minimal unix timestamp → "YYYY-MM-DD" formatter (no external deps).
fn format_unix_date(ts: i64) -> String {
    let ts = ts.max(0) as u64;
    let days = ts / 86400;

    let mut year = 1970u32;
    let mut remaining = days;

    loop {
        let leap = is_leap(year);
        let days_in_year = if leap { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }

    let months = if is_leap(year) {
        [31u32, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1u32;
    for &days_in_month in &months {
        if remaining < days_in_month as u64 {
            break;
        }
        remaining -= days_in_month as u64;
        month += 1;
    }
    let day = remaining + 1;

    format!("{:04}-{:02}-{:02}", year, month, day)
}

fn is_leap(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Callback receives (pct: f32, status: String) after each file.
/// `vault_root` is the root of the vault — used for writing `_ancestral_brain/` markdown.
/// `embed_model` and `chat_model` come from hardware detection.
pub fn index_folder<F>(
    conn: &Connection,
    folder: &Path,
    vault_root: &Path,
    embed_model: &str,
    chat_model: &str,
    mut on_progress: F,
) -> Result<Progress>
where
    F: FnMut(f32, String),
{
    // Canonicalize vault root for reliable prefix stripping
    let vault_root = vault_root.canonicalize().unwrap_or_else(|_| vault_root.to_path_buf());
    let ab_dir = vault_root.join("_ancestral_brain");

    // Create _ancestral_brain/ with restricted permissions (user-only on Unix)
    if !ab_dir.exists() {
        std::fs::create_dir_all(&ab_dir).ok();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&ab_dir, std::fs::Permissions::from_mode(0o700)).ok();
        }
    }

    let entries: Vec<PathBuf> = WalkDir::new(folder)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        // Skip _ancestral_brain/ directory itself
        .filter(|e| !e.path().starts_with(&ab_dir))
        .map(|e| e.path().to_path_buf())
        .collect();

    let total = entries.len();
    let mut done = 0usize;
    let mut skipped = 0usize;
    let mut errors = 0usize;

    let ollama_up = ollama::is_running();

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
        let text_content_opt: Option<String> = if TEXT_EXTS.contains(&ext.as_str()) {
            meta::text_content(path)
        } else if DOC_EXTS.contains(&ext.as_str()) {
            meta::pdf_text(path)
        } else {
            None
        };

        let chunks: Vec<String> = if let Some(ref text) = text_content_opt {
            meta::chunk_text(text)
        } else if AUDIO_EXTS.contains(&ext.as_str()) {
            vec![format!(
                "Audio file: {} | Path: {}{}",
                filename,
                path_str,
                duration_s.map(|d| format!(" | Duration: {:.0}s", d)).unwrap_or_default()
            )]
        } else {
            vec![format!("File: {} | Path: {}", filename, path_str)]
        };

        for (seq, chunk_text) in chunks.iter().enumerate() {
            let chunk_id = match db::insert_chunk(conn, file_id, seq as i32, chunk_text) {
                Ok(id) => id,
                Err(_) => continue,
            };
            if let Ok(emb) = ollama::embed(chunk_text, embed_model) {
                db::insert_embedding(conn, chunk_id, &emb).ok();
            }
        }

        // Generate AI summary for audio or non-text files when Ollama is up
        let ai_summary = if ollama_up && (AUDIO_EXTS.contains(&ext.as_str()) || text_content_opt.is_none()) {
            let extra = duration_s
                .map(|d| format!("Duration: {:.0}s", d))
                .unwrap_or_default();
            let folder_str = path
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            ollama::generate_summary(&filename, &folder_str, &ext, &extra, chat_model)
        } else {
            String::new()
        };

        // Write companion markdown
        let text_preview = text_content_opt.as_deref();
        write_markdown(
            &vault_root,
            path,
            &filename,
            &ext,
            size,
            mtime,
            duration_s,
            sample_rate,
            text_preview,
            &ai_summary,
        );

        done += 1;
        let pct = done as f32 / total as f32 * 100.0;
        on_progress(pct, format!("Indexed: {}", filename));
    }

    Ok(Progress { total, done, skipped, errors })
}
