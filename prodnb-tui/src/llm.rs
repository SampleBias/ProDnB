//! LLM API integration for reorganizing PDB structure into Strudel code.
//!
//! Uses Groq Cloud with Llama. Set GROQ_API_KEY to enable.

use anyhow::Result;
use serde_json::json;

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

Return ONLY the Strudel code, no explanation. Use setcps around 0.7 for 174 BPM."#;

/// Call Groq Cloud (Compound) to reorganize PDB content and return Strudel code.
/// Uses the Responses API: POST /openai/v1/responses with model groq/compound.
pub fn reorganize_with_llm(pdb_content: &str) -> Result<String> {
    let api_key = std::env::var("GROQ_API_KEY")
        .map_err(|_| anyhow::anyhow!("Set GROQ_API_KEY in .env to use LLM (Groq Cloud)"))?;

    let input = format!(
        "{}\n\nOutput ONLY valid Strudel code, no markdown or explanation.\n\nPDB content:\n{}",
        PROMPT, pdb_content
    );

    let client = reqwest::blocking::Client::new();
    let url = "https://api.groq.com/openai/v1/responses";

    let body = json!({
        "model": "groq/compound",
        "input": input
    });

    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().unwrap_or_default();
        anyhow::bail!("LLM API error {}: {}", status, text);
    }

    let json: serde_json::Value = response.json()?;
    let content = json["output_text"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No output_text in LLM response"))?
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
