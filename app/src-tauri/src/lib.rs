mod db;
mod indexer;
mod meta;
mod ollama;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

// ── State ─────────────────────────────────────────────────────────────────────

struct AppState {
    db: Arc<Mutex<rusqlite::Connection>>,
    vault_folder: Mutex<Option<PathBuf>>,
}

fn db_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("vault.db")
}

// ── Commands ──────────────────────────────────────────────────────────────────

/// Check if Ollama is reachable; navigate accordingly.
#[tauri::command]
fn check_ollama(app: AppHandle) {
    if ollama::is_running() {
        // Load the main screen
        if let Some(win) = app.get_webview_window("main") {
            win.eval("window.location.href='main.html'").ok();
        }
        // If vault folder is set, trigger index
        let state = app.state::<AppState>();
        if let Some(folder) = state.vault_folder.lock().unwrap().clone() {
            start_index(app.clone(), folder);
        }
    } else {
        if let Some(win) = app.get_webview_window("main") {
            win.eval("window.location.href='ollama_check.html'").ok();
        }
    }
}

/// Open a folder picker dialog and persist the choice.
#[tauri::command]
async fn pick_folder(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_dialog::DialogExt;
    let folder = app
        .dialog()
        .file()
        .set_title("Choose your vault folder")
        .blocking_pick_folder();

    match folder {
        Some(path) => {
            let path_buf: PathBuf = path.into();
            {
                let state = app.state::<AppState>();
                *state.vault_folder.lock().unwrap() = Some(path_buf.clone());
            }
            // Save to app data for next launch
            save_config(&app, &path_buf);
            // Check Ollama before indexing
            check_ollama(app.clone());
            Ok(())
        }
        None => Ok(()), // user cancelled
    }
}

/// Reveal a file in Finder.
#[tauri::command]
fn reveal_in_finder(path: String) {
    std::process::Command::new("open")
        .args(["-R", &path])
        .spawn()
        .ok();
}

/// Semantic + filename search.
#[tauri::command]
fn search(app: AppHandle, query: String) -> Result<Vec<db::SearchResult>, String> {
    let state = app.state::<AppState>();
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    let mut results = db::filename_search(&conn, &query, 20).unwrap_or_default();

    // Add semantic results if Ollama is up
    if ollama::is_running() {
        if let Ok(emb) = ollama::embed(&query) {
            let sem = db::semantic_search(&conn, &emb, 20).unwrap_or_default();
            // Merge: deduplicate by path, prefer higher score
            for sr in sem {
                if !results.iter().any(|r: &db::SearchResult| r.path == sr.path) {
                    results.push(sr);
                }
            }
        }
    }

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(30);
    Ok(results)
}

/// RAG chat over the index.
#[tauri::command]
fn chat(app: AppHandle, question: String) -> Result<serde_json::Value, String> {
    let state = app.state::<AppState>();
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    if !ollama::is_running() {
        return Err("Ollama is not running. Start it with: ollama serve".into());
    }

    // Embed + retrieve top-k chunks
    let emb = ollama::embed(&question).map_err(|e| e.to_string())?;
    let results = db::semantic_search(&conn, &emb, 8).map_err(|e| e.to_string())?;

    if results.is_empty() {
        return Ok(serde_json::json!({
            "answer": "No relevant files found in your vault for that query.",
            "sources": []
        }));
    }

    let sources: Vec<&str> = results.iter().map(|r| r.path.as_str()).collect();
    let context: Vec<String> = results
        .iter()
        .map(|r| format!("File: {}\n{}", r.path, r.snippet))
        .collect();

    let answer = ollama::chat(&question, &context).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "answer": answer, "sources": sources }))
}

// ── Index runner ──────────────────────────────────────────────────────────────

fn start_index(app: AppHandle, folder: PathBuf) {
    // Navigate to main screen
    if let Some(win) = app.get_webview_window("main") {
        win.eval("window.location.href='main.html'").ok();
    }

    let app2 = app.clone();
    std::thread::spawn(move || {
        let state = app2.state::<AppState>();
        let conn = match state.db.lock() {
            Ok(c) => c,
            Err(_) => {
                app2.emit("index-error", "Failed to open database").ok();
                return;
            }
        };

        let app3 = app2.clone();
        let result = indexer::index_folder(&conn, &folder, move |pct, status| {
            app3.emit("index-progress", serde_json::json!({ "pct": pct, "status": status })).ok();
        });

        match result {
            Ok(prog) => {
                app2.emit(
                    "index-done",
                    serde_json::json!({ "total": prog.total, "skipped": prog.skipped, "errors": prog.errors }),
                ).ok();
            }
            Err(e) => {
                app2.emit("index-error", e.to_string()).ok();
            }
        }
    });
}

// ── Config persistence ────────────────────────────────────────────────────────

fn config_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config.json")
}

fn save_config(app: &AppHandle, folder: &Path) {
    let p = config_path(app);
    let j = serde_json::json!({ "vault_folder": folder.to_string_lossy() });
    std::fs::write(p, j.to_string()).ok();
}

fn load_config(app: &AppHandle) -> Option<PathBuf> {
    let p = config_path(app);
    let text = std::fs::read_to_string(p).ok()?;
    let j: serde_json::Value = serde_json::from_str(&text).ok()?;
    let s = j["vault_folder"].as_str()?;
    let path = PathBuf::from(s);
    if path.exists() { Some(path) } else { None }
}

// ── App entry point ───────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Init DB
            let db_p = db_path(app.handle());
            std::fs::create_dir_all(db_p.parent().unwrap()).ok();
            let conn = db::open(&db_p).expect("Failed to open vault database");

            // Determine initial screen
            let saved_folder = load_config(app.handle());
            let has_folder = saved_folder.is_some();

            let state = AppState {
                db: Arc::new(Mutex::new(conn)),
                vault_folder: Mutex::new(saved_folder.clone()),
            };
            app.manage(state);

            // Navigate to correct initial screen
            let handle = app.handle().clone();
            let initial_url = if !has_folder {
                "onboarding.html"
            } else if ollama::is_running() {
                "main.html"
            } else {
                "ollama_check.html"
            };

            if let Some(win) = handle.get_webview_window("main") {
                win.eval(&format!("window.location.href='{initial_url}'")).ok();
                // If returning user with folder + ollama, kick off index
                if has_folder && ollama::is_running() {
                    if let Some(folder) = saved_folder {
                        start_index(handle.clone(), folder);
                    }
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            pick_folder,
            check_ollama,
            reveal_in_finder,
            search,
            chat,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Ancestral Brain");
}
