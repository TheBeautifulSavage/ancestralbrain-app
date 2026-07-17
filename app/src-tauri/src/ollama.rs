/// Ollama client — chat completions + embeddings via localhost:11434
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

const BASE: &str = "http://localhost:11434";
const CHAT_MODEL: &str = "llama3.1:8b";
const EMBED_MODEL: &str = "nomic-embed-text";

/// Returns true if Ollama is reachable.
pub fn is_running() -> bool {
    reqwest::blocking::get(BASE)
        .map(|r| r.status().is_success() || r.status().as_u16() == 404)
        .unwrap_or(false)
}

/// Get embedding vector for text. Returns Vec<f32>.
pub fn embed(text: &str) -> Result<Vec<f32>> {
    #[derive(Serialize)]
    struct Req<'a> { model: &'a str, prompt: &'a str }
    #[derive(Deserialize)]
    struct Resp { embedding: Vec<f32> }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;
    let resp: Resp = client
        .post(format!("{BASE}/api/embeddings"))
        .json(&Req { model: EMBED_MODEL, prompt: text })
        .send()?
        .json()?;
    Ok(resp.embedding)
}

/// RAG chat: given a question + context chunks, return an answer string.
pub fn chat(question: &str, context_chunks: &[String]) -> Result<String> {
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
            model: CHAT_MODEL,
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
