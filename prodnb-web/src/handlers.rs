//! Request handlers for ProDnB Web Server

use actix_web::{web, HttpResponse, Error, error};
use actix_multipart::Multipart;
use futures::StreamExt;
use tempfile::NamedTempFile;
use std::io::Write;
use serde::{Deserialize, Serialize};
use prodnb_core::{Protein, ProteinFramework};
use anyhow::{Context, Result};

/// Maximum file size for PDB upload (10MB)
const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;

/// Response structure for PDB upload
#[derive(Debug, Serialize)]
pub struct UploadResponse {
    success: bool,
    message: String,
    file_path: Option<String>,
    chain_count: Option<usize>,
    residue_count: Option<usize>,
    atom_count: Option<usize>,
}

/// Request structure for Strudel generation
#[derive(Debug, Deserialize)]
pub struct GenerateRequest {
    file_path: String,
    #[serde(default)]
    style: Option<String>,
    #[serde(default)]
    bpm: Option<u16>,
}

/// Response structure for Strudel generation (streaming)
#[derive(Debug, Serialize, Clone)]
pub struct StrudelChunk {
    chunk_type: String, // "start", "chunk", "done", "error"
    content: String,
}

/// Health check endpoint
pub async fn health_check() -> impl actix_web::Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "ProDnB Web Server"
    }))
}

/// Handle PDB file upload
pub async fn upload_pdb(mut payload: Multipart) -> Result<HttpResponse, Error> {
    let mut file_path: Option<String> = None;

    // Process multipart form
    while let Some(field) = payload.next().await {
        let mut field = field.map_err(|e| {
            error::ErrorBadRequest(format!("Field error: {}", e))
        })?;

        let content_type = field.content_disposition();
        let name = content_type.get_name().unwrap_or("unknown");

        if name == "pdb_file" {
            // Create temporary file
            let temp_file = NamedTempFile::new()
                .map_err(|e| error::ErrorInternalServerError(format!("Temp file error: {}", e)))?;
            
            let temp_path = temp_file.path().to_string_lossy().to_string();
            let mut file = temp_file.reopen()
                .map_err(|e| error::ErrorInternalServerError(format!("File reopen error: {}", e)))?;

            // Copy stream to file with size limit
            let mut size = 0;
            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| {
                    error::ErrorBadRequest(format!("Chunk error: {}", e))
                })?;

                size += data.len();
                if size > MAX_FILE_SIZE {
                    return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                        "error": "File too large",
                        "max_size": MAX_FILE_SIZE,
                        "received_size": size
                    })));
                }

                file.write_all(&data)
                    .map_err(|e| error::ErrorInternalServerError(format!("Write error: {}", e)))?;
            }
            
            log::info!("Uploaded PDB file: {} bytes", size);            file_path = Some(temp_path);
            
            log::info!("Uploaded PDB file: {} bytes", size);
        } else {
            // Skip other fields
            while let Some(_) = field.next().await {}
        }
    }

    // Process the uploaded file
    if let Some(path) = file_path {
        match Protein::load_from_file(&path) {
            Ok(protein) => {
                let response = UploadResponse {
                    success: true,
                    message: format!("Successfully loaded PDB file"),
                    file_path: Some(path),
                    chain_count: Some(protein.chain_count()),
                    residue_count: Some(protein.residue_count()),
                    atom_count: Some(protein.atom_count()),
                };
                Ok(HttpResponse::Ok().json(response))
            }
            Err(e) => {
                log::error!("Failed to parse PDB: {}", e);
                Ok(HttpResponse::BadRequest().json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to parse PDB file: {}", e)
                })))
            }
        }
    } else {
        Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "No file uploaded. Please provide a 'pdb_file' field."
        })))
    }
}

/// Generate Strudel code from uploaded PDB using LLM
pub async fn generate_strudel(
    req: web::Json<GenerateRequest>,
) -> Result<HttpResponse, Error> {
    log::info!("Generating Strudel code from: {}", req.file_path);

    // Load protein from file
    let protein = Protein::load_from_file(&req.file_path)
        .map_err(|e| {
            log::error!("Failed to load protein: {}", e);
            error::ErrorBadRequest(format!("Failed to load PDB file: {}", e))
        })?;

    // Build framework for LLM
    let framework = ProteinFramework::from_protein(&protein)
        .map_err(|e| {
            log::error!("Failed to build framework: {}", e);
            error::ErrorInternalServerError(format!("Failed to build protein framework: {}", e))
        })?;

    let framework_json = framework.to_json()
        .map_err(|e| {
            log::error!("Failed to serialize framework: {}", e);
            error::ErrorInternalServerError(format!("Failed to serialize framework: {}", e))
        })?;

    log::info!("Framework size: {} bytes", framework_json.len());

    // Call LLM API
    match call_groq_api(&framework_json).await {
        Ok(strudel_code) => {
            log::info!("Generated {} chars of Strudel code", strudel_code.len());
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "code": strudel_code,
                "chain_count": protein.chain_count(),
                "residue_count": protein.residue_count(),
            })))
        }
        Err(e) => {
            log::error!("LLM API error: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": format!("Failed to generate Strudel code: {}", e)
            })))
        }
    }
}

/// Call Groq API with protein framework to generate Strudel code
async fn call_groq_api(framework_json: &str) -> Result<String> {
    let api_key = std::env::var("GROQ_API_KEY")
        .context("GROQ_API_KEY environment variable not set")?;

    let prompt = r#"You are a Drum & Bass music programmer. Use this preprocessed protein framework to create appealing Strudel (strudel.cc) code.

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

    let user_content = format!(
        "{}\n\nFramework (preprocessed protein data):\n{}\n\nOutput ONLY valid Strudel code.",
        prompt, framework_json
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .context("Failed to build HTTP client")?;

    let url = "https://api.groq.com/openai/v1/chat/completions";

    let body = serde_json::json!({
        "model": "groq/compound",
        "messages": [
            {"role": "system", "content": "You output only valid Strudel code. No markdown, no explanation."},
            {"role": "user", "content": user_content}
        ],
        "max_tokens": 2048,
        "temperature": 0.4
    });

    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .context("Failed to send request to Groq API")?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("Groq API error {}: {}", status, text);
    }

    let json: serde_json::Value = response.json().await
        .context("Failed to parse Groq API response")?;

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
