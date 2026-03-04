//! LLM API integration for reorganizing PDB structure into Strudel code.
//!
//! Uses Groq Cloud (Compound). Set GROQ_API_KEY in .env to enable.

use anyhow::Result;
use serde_json::json;
use std::io::{BufRead, BufReader};
use std::sync::mpsc;

const PROMPT: &str = r#"You are a music programmer. Convert this PDB protein structure into Strudel (strudel.cc) drum pattern code.

Mapping rules:
- C (Carbon) atoms → "bd" (bass drum)
- N (Nitrogen) atoms → "sd" (snare)
- O (Oxygen) atoms → "hh" (hi-hat)
- S (Sulfur) atoms → "cp" (clap)
- P (Phosphorus) → "rim"

Create a Drum & Bass style pattern. Use Strudel syntax:
- setcps(bpm/60/4) for tempo
- sound("pattern") for patterns
- stack([...]) for layering
- Use * for repetition, ~ for rest

Return ONLY the Strudel code, no markdown, no explanation. Use setcps around 0.7 for 174 BPM."#;

/// Stream event from LLM
#[derive(Debug)]
pub enum LlmStreamMsg {
    Chunk(String),
    Done,
    Error(String),
}

/// Start streaming LLM response in a background thread. Returns receiver for main loop to poll.
pub fn stream_llm(pdb_content: String) -> Result<mpsc::Receiver<LlmStreamMsg>> {
    let api_key = std::env::var("GROQ_API_KEY")
        .map_err(|_| anyhow::anyhow!("Set GROQ_API_KEY in .env to use LLM"))?;

    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let user_content = format!(
            "{}\n\nOutput ONLY valid Strudel code. No markdown, no explanation.\n\nPDB content:\n{}",
            PROMPT, pdb_content
        );

        let client = match reqwest::blocking::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(15))
            .timeout(std::time::Duration::from_secs(120))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(LlmStreamMsg::Error(format!("HTTP client: {}", e)));
                return;
            }
        };

        let body = json!({
            "model": "groq/compound",
            "stream": true,
            "messages": [
                {"role": "system", "content": "You output only valid Strudel code. No markdown, no explanation."},
                {"role": "user", "content": user_content}
            ],
            "max_tokens": 1024,
            "temperature": 0.3
        });

        let response = match client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
        {
            Ok(r) => r,
            Err(e) => {
                let _ = tx.send(LlmStreamMsg::Error(e.to_string()));
                return;
            }
        };

        if !response.status().is_success() {
            let text = response.text().unwrap_or_default();
            let _ = tx.send(LlmStreamMsg::Error(format!("API error: {}", text)));
            return;
        }

        let reader = BufReader::new(response);
        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            if line.starts_with("data: ") {
                let data = line.trim_start_matches("data: ").trim();
                if data == "[DONE]" {
                    break;
                }
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                        if !content.is_empty() && tx.send(LlmStreamMsg::Chunk(content.to_string())).is_err() {
                            return;
                        }
                    }
                }
            }
        }

        let _ = tx.send(LlmStreamMsg::Done);
    });

    Ok(rx)
}

/// Call Groq Cloud (Compound) to reorganize PDB content and return Strudel code (blocking).
pub fn reorganize_with_llm(pdb_content: &str) -> Result<String> {
    let api_key = std::env::var("GROQ_API_KEY")
        .map_err(|_| anyhow::anyhow!("Set GROQ_API_KEY in .env to use LLM (Groq Cloud)"))?;

    let user_content = format!(
        "{}\n\nOutput ONLY valid Strudel code, no markdown or explanation.\n\nPDB content:\n{}",
        PROMPT, pdb_content
    );

    // Compound can take 60–120s (tool use, reasoning). Use generous timeouts.
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| anyhow::anyhow!("HTTP client: {}", e))?;

    let url = "https://api.groq.com/openai/v1/chat/completions";

    let body = json!({
        "model": "groq/compound",
        "messages": [
            {"role": "system", "content": "You output only valid Strudel code. No markdown, no explanation."},
            {"role": "user", "content": user_content}
        ],
        "max_tokens": 1024,
        "temperature": 0.3
    });

    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("timed out") {
                anyhow::anyhow!(
                    "LLM request timed out (Compound can take 60–120s). Check network or try again."
                )
            } else {
                anyhow::anyhow!("LLM request failed: {}. Ensure GROQ_API_KEY is valid and network is reachable.", msg)
            }
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().unwrap_or_default();
        anyhow::bail!("LLM API error {}: {}", status, text);
    }

    let json: serde_json::Value = response.json()?;
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No content in LLM response"))?
        .trim()
        .to_string();

    // Strip markdown code blocks if present
    let content = if content.starts_with("```") {
        content
            .trim_start_matches("```")
            .trim_start_matches("javascript")
            .trim_start_matches("js")
            .trim_start_matches("strudel")
            .trim_start_matches('\n')
            .trim_end_matches("```")
            .trim()
            .to_string()
    } else {
        content
    };

    Ok(content)
}
