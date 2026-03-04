//! LLM API integration for reorganizing protein framework into Strudel code.
//!
//! Uses Groq Cloud (Compound). Set GROQ_API_KEY in .env to enable.
//! Framework is built from GPU-preprocessed (or CPU) protein data.

use anyhow::Result;
use serde_json::json;
use std::sync::mpsc;

const PROMPT: &str = r#"You are a Drum & Bass music programmer. Use this preprocessed protein framework to create appealing Strudel (strudel.cc) code.

Element → drum mapping (use as hints, not strict):
- C (Carbon) → bd (bass drum)
- N (Nitrogen) → sd (snare)
- O (Oxygen) → hh (hi-hat)
- S (Sulfur) → cp (clap)
- P (Phosphorus) → rim

Strudel syntax (REQUIRED):
- s("bd sd hh") or sound("bd sd hh") for patterns
- setcps(0.7) for tempo (~174 BPM D&B)
- stack(s("bd"), s("hh*8"), s("sd")) for layering
- Mini-notation: ~ rest, * speed (e.g. hh*8 = 8 hi-hats per cycle), [] sub-sequences, , parallel
- Drum sounds: bd, sd, hh, cp, rim, oh (open hat)

Use rhythm_seed as structural inspiration. Use chain_lengths for polyrhythmic layers.
Create something that sounds good: driving bass, crisp snares, tight hats. D&B energy.

Return ONLY valid Strudel code. No markdown, no explanation."#;

/// Stream event from LLM
#[derive(Debug)]
pub enum LlmStreamMsg {
    Chunk(String),
    Done,
    Error(String),
}

/// Start LLM request in a background thread. Fetches from Groq and sends result to output area.
/// Uses blocking API (Compound may not stream reliably). Code appears in output when done.
/// Takes framework JSON (from GPU/CPU preprocessing), not raw PDB.
pub fn stream_llm(framework_json: String) -> Result<mpsc::Receiver<LlmStreamMsg>> {
    std::env::var("GROQ_API_KEY")
        .map_err(|_| anyhow::anyhow!("Set GROQ_API_KEY in .env to use LLM"))?;

    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let _ = tx.send(LlmStreamMsg::Chunk("Calling Groq Compound...\n".into()));
        match reorganize_with_llm(&framework_json) {
            Ok(code) => {
                let _ = tx.send(LlmStreamMsg::Chunk(code));
            }
            Err(e) => {
                let _ = tx.send(LlmStreamMsg::Error(e.to_string()));
                return;
            }
        }
        let _ = tx.send(LlmStreamMsg::Done);
    });

    Ok(rx)
}

/// Call Groq Cloud (Compound) to reorganize framework and return Strudel code (blocking).
pub fn reorganize_with_llm(framework_json: &str) -> Result<String> {
    let api_key = std::env::var("GROQ_API_KEY")
        .map_err(|_| anyhow::anyhow!("Set GROQ_API_KEY in .env to use LLM (Groq Cloud)"))?;

    let user_content = format!(
        "{}\n\nFramework (preprocessed protein data):\n{}\n\nOutput ONLY valid Strudel code.",
        PROMPT, framework_json
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
