/// Ollama client — chat completions + embeddings via localhost:11434
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};

const BASE: &str = "http://localhost:11434";

/// Returns true if Ollama is reachable.
pub fn is_running() -> bool {
    reqwest::blocking::get(BASE)
        .map(|r| r.status().is_success() || r.status().as_u16() == 404)
        .unwrap_or(false)
}

/// List installed model names.
pub fn list_models() -> Result<Vec<String>> {
    #[derive(Deserialize)]
    struct ModelEntry { name: String }
    #[derive(Deserialize)]
    struct Resp { models: Vec<ModelEntry> }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let resp: Resp = client
        .get(format!("{BASE}/api/tags"))
        .send()?
        .json()?;
    Ok(resp.models.into_iter().map(|m| m.name).collect())
}

/// Pull a model from Ollama registry, streaming NDJSON progress.
/// `on_progress(completed, total, status)` is called for each progress line.
pub fn pull_model<F>(model: &str, mut on_progress: F) -> Result<()>
where
    F: FnMut(u64, u64, String),
{
    #[derive(Serialize)]
    struct Req<'a> { name: &'a str, stream: bool }
    #[derive(Deserialize)]
    struct Line {
        status: Option<String>,
        completed: Option<u64>,
        total: Option<u64>,
        error: Option<String>,
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(3600))
        .build()?;
    let resp = client
        .post(format!("{BASE}/api/pull"))
        .json(&Req { name: model, stream: true })
        .send()?;

    if !resp.status().is_success() {
        return Err(anyhow!("Pull failed: HTTP {}", resp.status()));
    }

    let reader = BufReader::new(resp);
    for raw_line in reader.lines() {
        let raw_line = raw_line?;
        if raw_line.trim().is_empty() {
            continue;
        }
        if let Ok(line) = serde_json::from_str::<Line>(&raw_line) {
            if let Some(err) = line.error {
                return Err(anyhow!("Pull error: {err}"));
            }
            let completed = line.completed.unwrap_or(0);
            let total = line.total.unwrap_or(0);
            let status = line.status.unwrap_or_default();
            on_progress(completed, total, status);
        }
    }
    Ok(())
}

/// Generate a short AI summary for a file. Returns empty string on any error.
/// `model` is the chat model name (e.g. "llama3.2:3b").
pub fn generate_summary(filename: &str, folder: &str, ext: &str, extra: &str, model: &str) -> String {
    #[derive(Serialize)]
    struct Message<'a> { role: &'a str, content: String }
    #[derive(Serialize)]
    struct Req<'a> { model: &'a str, messages: Vec<Message<'a>>, stream: bool }
    #[derive(Deserialize)]
    struct RespMessage { content: String }
    #[derive(Deserialize)]
    struct Resp { message: RespMessage }

    let prompt = format!(
        "File: {filename}\nFolder: {folder}\nType: {ext}\n{extra}\n\n\
         In 20 words or less, describe what this file might contain or be used for."
    );

    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(_) => return String::new(),
    };

    let resp: Resp = match client
        .post(format!("{BASE}/api/chat"))
        .json(&Req {
            model,
            messages: vec![Message { role: "user", content: prompt }],
            stream: false,
        })
        .send()
        .and_then(|r| r.json())
    {
        Ok(r) => r,
        Err(_) => return String::new(),
    };

    resp.message.content.trim().to_string()
}

/// Get embedding vector for text. Returns Vec<f32>.
/// `model` is the embedding model name (e.g. "nomic-embed-text").
pub fn embed(text: &str, model: &str) -> Result<Vec<f32>> {
    #[derive(Serialize)]
    struct Req<'a> { model: &'a str, prompt: &'a str }
    #[derive(Deserialize)]
    struct Resp { embedding: Vec<f32> }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;
    let resp: Resp = client
        .post(format!("{BASE}/api/embeddings"))
        .json(&Req { model, prompt: text })
        .send()?
        .json()?;
    Ok(resp.embedding)
}

/// RAG chat: given a question + context chunks, return an answer string.
/// `model` is the chat model name (e.g. "llama3.2:3b").
pub fn chat(question: &str, context_chunks: &[String], model: &str) -> Result<String> {
    #[derive(Serialize)]
    struct Message<'a> { role: &'a str, content: String }
    #[derive(Serialize)]
    struct Req<'a> { model: &'a str, messages: Vec<Message<'a>>, stream: bool }
    #[derive(Deserialize)]
    struct RespMessage { content: String }
    #[derive(Deserialize)]
    struct Resp { message: RespMessage }

    let context = context_chunks.join("\n\n---\n\n");
    let system = format!(
        "You are a file vault assistant. The user is searching their personal library of \
         audio files, documents, and voice memos. Answer their question concisely using \
         ONLY the file excerpts provided. Cite files by their full path in your answer.\n\n\
         FILE EXCERPTS:\n{context}"
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;
    let resp: Resp = client
        .post(format!("{BASE}/api/chat"))
        .json(&Req {
            model,
            messages: vec![
                Message { role: "system", content: system },
                Message { role: "user", content: question.to_string() },
            ],
            stream: false,
        })
        .send()?
        .json()
        .map_err(|e| anyhow!("Ollama chat parse error: {e}"))?;
    Ok(resp.message.content)
}
