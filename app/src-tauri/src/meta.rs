/// Audio/document metadata extraction.
use std::path::Path;
use symphonia::core::{
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};

pub struct AudioMeta {
    pub duration_s: Option<f64>,
    pub sample_rate: Option<u32>,
}

pub fn audio_meta(path: &Path) -> AudioMeta {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return AudioMeta { duration_s: None, sample_rate: None },
    };
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let probed = match symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    ) {
        Ok(p) => p,
        Err(_) => return AudioMeta { duration_s: None, sample_rate: None },
    };
    let format = probed.format;
    let track = match format.tracks().first() {
        Some(t) => t,
        None => return AudioMeta { duration_s: None, sample_rate: None },
    };
    let params = &track.codec_params;
    let sample_rate = params.sample_rate;
    let duration_s = match (params.n_frames, params.sample_rate) {
        (Some(frames), Some(rate)) if rate > 0 => Some(frames as f64 / rate as f64),
        _ => None,
    };
    AudioMeta {
        duration_s,
        sample_rate: sample_rate.map(|r| r as u32),
    }
}

/// Extract text from plain text / markdown files.
pub fn text_content(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// Extract text from PDF using lopdf.
pub fn pdf_text(path: &Path) -> Option<String> {
    let doc = lopdf::Document::load(path).ok()?;
    let mut out = String::new();
    for page_id in doc.page_iter() {
        if let Ok(text) = doc.extract_text(&[page_id.0]) {
            out.push_str(&text);
            out.push('\n');
        }
    }
    if out.trim().is_empty() { None } else { Some(out) }
}

/// Chunk a text string into pieces of ~500 words with 50-word overlap.
pub fn chunk_text(text: &str) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![];
    }
    let chunk_size = 500;
    let overlap = 50;
    let mut chunks = vec![];
    let mut start = 0;
    while start < words.len() {
        let end = (start + chunk_size).min(words.len());
        chunks.push(words[start..end].join(" "));
        if end == words.len() {
            break;
        }
        start += chunk_size - overlap;
    }
    chunks
}
