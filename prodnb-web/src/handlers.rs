//! Request handlers for ProDnB Web Server

use actix_web::{web, HttpResponse, Error, error};
use actix_multipart::Multipart;
use actix_web_lab::sse;
use futures::StreamExt;
use tempfile::NamedTempFile;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use prodnb_core::{Protein, ProteinFramework, protein_to_primitives, assemble_strudel, MappedOutput, SliderValues};
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
            let mut temp_file = NamedTempFile::new()
                .map_err(|e| error::ErrorInternalServerError(format!("Temp file error: {}", e)))?;

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

                temp_file.write_all(&data)
                    .map_err(|e| error::ErrorInternalServerError(format!("Write error: {}", e)))?;
            }

            temp_file.flush()
                .map_err(|e| error::ErrorInternalServerError(format!("Flush error: {}", e)))?;

            // Persist to a stable path - NamedTempFile deletes on drop, so we must persist
            // before the temp_file goes out of scope
            let persist_path = std::env::temp_dir().join(format!(
                "prodnb_{}.pdb",
                SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
            ));
            temp_file.persist(&persist_path)
                .map_err(|e| error::ErrorInternalServerError(format!("Failed to save uploaded file: {}", e)))?;

            let temp_path = persist_path.to_string_lossy().to_string();
            file_path = Some(temp_path);
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

/// Map PDB to primitives (stage 1, deterministic). Returns JSON for piano roll.
pub async fn map_primitives(
    req: web::Json<GenerateRequest>,
) -> Result<HttpResponse, Error> {
    log::info!("Mapping PDB to primitives: {}", req.file_path);

    let protein = match Protein::load_from_file(&req.file_path) {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to load protein: {}", e);
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Failed to load PDB file: {}", e)
            })));
        }
    };

    let bpm = req.bpm.unwrap_or(174);
    let mapped = match protein_to_primitives(&protein, bpm) {
        Ok(m) => m,
        Err(e) => {
            log::error!("Failed to map primitives: {}", e);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Mapping failed: {}", e)
            })));
        }
    };

    Ok(HttpResponse::Ok().json(mapped))
}

/// Assemble Strudel from primitives + sliders (no LLM).
#[derive(Debug, Deserialize)]
pub struct AssembleRequest {
    pub primitives: Vec<prodnb_core::StrudelPrimitive>,
    pub tempo: u16,
    #[serde(default)]
    pub sliders: Option<SliderValues>,
}

pub async fn assemble_from_primitives(
    req: web::Json<AssembleRequest>,
) -> Result<HttpResponse, Error> {
    let mapped = MappedOutput {
        tempo: req.tempo,
        primitives: req.primitives.clone(),
        rhythm_seed: String::new(),
        chain_lengths: vec![],
        element_counts: std::collections::HashMap::new(),
    };
    let code = assemble_strudel(&mapped, req.sliders.as_ref());
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "code": code
    })))
}

/// Generate Strudel code with streaming (SSE).
pub async fn generate_strudel_stream(
    req: web::Json<GenerateRequest>,
) -> Result<impl actix_web::Responder, Error> {
    log::info!("Generating Strudel (stream) from: {}", req.file_path);

    let protein = Protein::load_from_file(&req.file_path)
        .map_err(|e| {
            log::error!("Failed to load protein: {}", e);
            error::ErrorBadRequest(format!("Failed to load PDB file: {}", e))
        })?;

    let framework = ProteinFramework::from_protein(&protein)
        .map_err(|e| {
            log::error!("Failed to build framework: {}", e);
            error::ErrorInternalServerError(format!("Failed to build framework: {}", e))
        })?;

    let framework_json = framework.to_json()
        .map_err(|e| {
            log::error!("Failed to serialize framework: {}", e);
            error::ErrorInternalServerError(format!("Failed to serialize: {}", e))
        })?;

    let user_content = format!(
        "Arrange these pre-mapped Strudel primitives into a full Drum & Bass track.\n\nFramework:\n{}\n\nOutput ONLY valid Strudel code.",
        framework_json
    );

    let api_key = std::env::var("GROQ_API_KEY").map_err(|_| {
        error::ErrorInternalServerError("GROQ_API_KEY not set")
    })?;

    let (tx, mut rx) = tokio::sync::mpsc::channel::<sse::Event>(32);

    tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(sse::Event::Data(sse::Data::new_json(serde_json::json!({
                    "chunk_type": "error",
                    "content": format!("HTTP client error: {}", e)
                })).unwrap())).await;
                return;
            }
        };

        let body = serde_json::json!({
            "model": "groq/compound",
            "messages": [
                {"role": "system", "content": DNB_SYSTEM_PROMPT},
                {"role": "user", "content": user_content}
            ],
            "max_tokens": 2048,
            "temperature": 0.4,
            "stream": true
        });

        let response = match client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                let _ = tx.send(sse::Event::Data(sse::Data::new_json(serde_json::json!({
                    "chunk_type": "error",
                    "content": format!("Request failed: {}", e)
                })).unwrap())).await;
                return;
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            let _ = tx.send(sse::Event::Data(sse::Data::new_json(serde_json::json!({
                "chunk_type": "error",
                "content": format!("API error {}: {}", status, text)
            })).unwrap())).await;
            return;
        }

        let mut stream = response.bytes_stream();
        let mut buf = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = match chunk {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(sse::Event::Data(sse::Data::new_json(serde_json::json!({
                        "chunk_type": "error",
                        "content": format!("Stream error: {}", e)
                    })).unwrap())).await;
                    break;
                }
            };
            buf.push_str(&String::from_utf8_lossy(&chunk));

            let lines: Vec<&str> = buf.split('\n').collect();
            for line in lines.iter().take(lines.len().saturating_sub(1)) {
                if line.starts_with("data: ") {
                    let data = &line[6..];
                    if data == "[DONE]" {
                        let _ = tx.send(sse::Event::Data(sse::Data::new_json(serde_json::json!({
                            "chunk_type": "done"
                        })).unwrap())).await;
                        return;
                    }
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                            if !content.is_empty() {
                                let _ = tx.send(sse::Event::Data(sse::Data::new_json(serde_json::json!({
                                    "chunk_type": "chunk",
                                    "content": content
                                })).unwrap())).await;
                            }
                        }
                    }
                }
            }
            if let Some(last) = lines.last() {
                if last.starts_with("data: ") {
                    buf = format!("{}\n", last);
                } else {
                    buf = last.to_string();
                }
            } else {
                buf.clear();
            }
        }

        let _ = tx.send(sse::Event::Data(sse::Data::new_json(serde_json::json!({
            "chunk_type": "done"
        })).unwrap())).await;
    });

    let event_stream = futures::stream::poll_fn(move |cx| {
        rx.poll_recv(cx).map(|opt| match opt {
            Some(ev) => Some(Ok::<_, std::convert::Infallible>(ev)),
            None => None,
        })
    });

    Ok(sse::Sse::from_stream(event_stream)
        .with_retry_duration(std::time::Duration::from_secs(10)))
}

/// DnB arrangement system prompt for LLM (stage 2 after deterministic mapping).
const DNB_SYSTEM_PROMPT: &str = r#"You are a Drum & Bass music arranger specializing in Strudel.cc. You receive pre-mapped Strudel primitives from a protein structure.

DnB REQUIREMENTS:
- Tempo: 174 BPM (setcps(0.725))
- Structure: Intro (minimal) → Buildup → Drop (full) → Breakdown → Drop → Outro
- Kick on 1 and 3+, snare on 2 and 4, hi-hats on 16ths (hh*8)
- Use syncopation, offbeat emphasis, ghost snares (~ sd)
- Phases: 16 or 32 bars per section
- 4/4 time, 16th-note subdivisions

Strudel syntax (REQUIRED):
- s("bd sd hh") or sound("bd sd hh") for patterns
- setcps(0.725) for tempo
- stack() for layering
- Mini-notation: ~ rest, * speed, [] sub-sequences, (beats,segments) euclidean
- gain() for intensity. Drum sounds: bd, sd, hh, cp, rim, oh

Use ONLY the provided primitives. Arrange them with stack(), gain(), slow(), fast().
Output ONLY valid Strudel code. No markdown, no explanation."#;

/// Call Groq API with protein framework to generate Strudel code
async fn call_groq_api(framework_json: &str) -> Result<String> {
    let api_key = std::env::var("GROQ_API_KEY")
        .context("GROQ_API_KEY environment variable not set")?;

    let user_content = format!(
        "Arrange these pre-mapped Strudel primitives into a full Drum & Bass track.\n\nFramework (includes primitives from protein mapping):\n{}\n\nOutput ONLY valid Strudel code.",
        framework_json
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .context("Failed to build HTTP client")?;

    let url = "https://api.groq.com/openai/v1/chat/completions";

    let body = serde_json::json!({
        "model": "groq/compound",
        "messages": [
            {"role": "system", "content": DNB_SYSTEM_PROMPT},
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
