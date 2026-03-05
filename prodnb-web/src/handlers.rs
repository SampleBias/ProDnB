//! Request handlers for ProDnB Web Server

use actix_web::{web, HttpResponse, Error, error};
use actix_multipart::Multipart;
use actix_web_lab::sse;
use futures::StreamExt;
use tempfile::NamedTempFile;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use prodnb_core::{Protein, ProteinFramework, protein_to_primitives, assemble_strudel, MappedOutput, SliderValues, DnBGenre, GenreParams};
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
    pdb_id: Option<String>,
    title: Option<String>,
}

/// Request structure for Strudel generation
#[derive(Debug, Deserialize)]
pub struct GenerateRequest {
    file_path: String,
    #[serde(default)]
    style: Option<String>,
    #[serde(default)]
    bpm: Option<u16>,
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub octave: Option<u8>,
    #[serde(default)]
    pub melodic: Option<bool>,
    /// When set, LLM infers genre/speed/harmony/drop from this function narrative
    #[serde(default)]
    pub selected_function: Option<String>,
    /// User-editable orchestration instruction (from "Continue the journey"). Takes precedence over selected_function for code gen.
    #[serde(default)]
    pub orchestration_instruction: Option<String>,
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

/// API info - GET /api returns available endpoints (for debugging 404s)
pub async fn api_info() -> impl actix_web::Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "endpoints": [
            "POST /api/upload",
            "POST /api/protein-function",
            "POST /api/infer-beat-design",
            "POST /api/generate-orchestration-instruction",
            "POST /api/map",
            "POST /api/assemble",
            "POST /api/generate",
            "POST /api/generate/stream"
        ]
    }))
}

/// Request for protein function lookup
#[derive(Debug, Deserialize)]
pub struct ProteinFunctionRequest {
    pub file_path: String,
}

/// Single function result from SERPAPI
#[derive(Debug, Serialize)]
pub struct FunctionResult {
    pub title: String,
    pub snippet: String,
}

/// Response for protein function lookup
#[derive(Debug, Serialize)]
pub struct ProteinFunctionResponse {
    pub pdb_id: Option<String>,
    pub title: Option<String>,
    pub functions: Vec<FunctionResult>,
}

/// Fetch protein biological function via SERPAPI
pub async fn protein_function(
    req: web::Json<ProteinFunctionRequest>,
) -> Result<HttpResponse, Error> {
    let protein = Protein::load_from_file(&req.file_path).map_err(|e| {
        log::error!("Failed to load protein: {}", e);
        error::ErrorBadRequest(format!("Failed to load PDB file: {}", e))
    })?;

    let pdb_id = protein.metadata.pdb_id.clone();
    let title = protein.metadata.title.clone();

    let query = match (&pdb_id, &title) {
        (Some(id), _) => format!("PDB {} protein biological function", id),
        (_, Some(t)) if !t.is_empty() => format!("{} protein biological function", t),
        _ => {
            return Ok(HttpResponse::Ok().json(ProteinFunctionResponse {
                pdb_id: None,
                title: None,
                functions: vec![],
            }));
        }
    };

    let api_key = std::env::var("SERP_API_Key").map_err(|_| {
        error::ErrorInternalServerError("SERP_API_Key not set in .env")
    })?;

    let url = format!(
        "https://serpapi.com/search?engine=google&q={}&api_key={}",
        urlencoding::encode(&query),
        api_key
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| error::ErrorInternalServerError(format!("HTTP client error: {}", e)))?;

    let response = client.get(&url).send().await.map_err(|e| {
        log::error!("SERPAPI request failed: {}", e);
        error::ErrorInternalServerError(format!("SERPAPI request failed: {}", e))
    })?;

    let mut functions = Vec::new();
    if response.status().is_success() {
        let json: serde_json::Value = response.json().await.map_err(|e| {
            log::error!("SERPAPI parse error: {}", e);
            error::ErrorInternalServerError("Failed to parse SERPAPI response")
        })?;
        if let Some(organic) = json["organic_results"].as_array() {
            for r in organic.iter().take(3) {
                let title = r["title"].as_str().unwrap_or("").to_string();
                let snippet = r["snippet"].as_str().unwrap_or("").to_string();
                if !title.is_empty() || !snippet.is_empty() {
                    functions.push(FunctionResult { title, snippet });
                }
            }
        }
    }

    Ok(HttpResponse::Ok().json(ProteinFunctionResponse {
        pdb_id,
        title,
        functions,
    }))
}

/// Request for beat design inference
#[derive(Debug, Deserialize)]
pub struct InferBeatDesignRequest {
    pub selected_function: String,
}

/// Inferred beat design params from LLM
#[derive(Debug, Serialize, Deserialize)]
pub struct InferredBeatDesign {
    pub genre: String,
    pub bpm: u16,
    pub key: Option<String>,
    pub melodic: bool,
}

/// Request for orchestration instruction generation
#[derive(Debug, Deserialize)]
pub struct GenerateOrchestrationRequest {
    pub selected_function: String,
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub melodic: Option<bool>,
}

/// Generate orchestration instruction from selection summary
pub async fn generate_orchestration_instruction(
    req: web::Json<GenerateOrchestrationRequest>,
) -> Result<HttpResponse, Error> {
    match generate_orchestration_from_summary(&req.selected_function, req.genre.as_deref(), req.key.as_deref(), req.melodic).await {
        Ok(instruction) => Ok(HttpResponse::Ok().json(serde_json::json!({ "instruction": instruction }))),
        Err(e) => {
            log::error!("Generate orchestration failed: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("{}", e)
            })))
        }
    }
}

/// Internal: call Groq to generate orchestration instruction from summary
async fn generate_orchestration_from_summary(
    summary: &str,
    genre: Option<&str>,
    key: Option<&str>,
    melodic: Option<bool>,
) -> Result<String> {
    let api_key = std::env::var("GROQ_API_KEY").context("GROQ_API_KEY not set")?;

    let recs = [
        genre.map(|g| format!("Genre: {}", g)),
        key.map(|k| format!("Key: {}", k)),
        melodic.map(|m| format!("Melodic layers: {}", m)),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(". ");

    let prompt = format!(
        r#"Given this protein function summary, write an orchestration instruction for an LLM that will generate Drum & Bass Strudel.cc code.

Summary: {}

Recommendations: {}

Your instruction must blend FOUR elements in one flowing paragraph:

1. ANTHROPOMORPHIZE the protein — give it a role, purpose, personality. Example: hemoglobin (HGB) = "the oxygen carrier that fires energy into the system"; a kinase = "the molecular switch that ignites cascades of activity"; an enzyme = "the catalyst that transforms stillness into motion."

2. POETIC INTERPRETATION — describe what the protein does in evocative, metaphorical language. Example: "pulses of oxygen moving through the body like rhythmic waves"; "a trigger that ignites chains of reactions."

3. MUSICAL METAPHORS — translate the biological role into musical imagery that guides the arrangement. Example: oxygen transport → "energy surges in the bass, rhythmic pulses like delivery"; binding/release → "tension and release, build and drop"; cascades → "layers that build and evolve over time."

4. TECHNICAL GUIDANCE — include concrete steps: BPM range, rhythm feel (breakbeat, 2-step, etc.), bass character (acid, deep, modulated), melodic approach (pads, lead, or sparse), drop structure (filter sweeps, cut-and-unleash, etc.).

Write one rich paragraph (4-8 sentences) that weaves all four together. The result should inspire Strudel.cc code generation with both poetic feeling and practical direction. Output ONLY the instruction text, no preamble."#,
        summary,
        if recs.is_empty() { "None" } else { &recs }
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("HTTP client")?;

    let body = serde_json::json!({
        "model": "groq/compound",
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": 768,
        "temperature": 0.6
    });

    let response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .context("Groq request")?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("Groq API error: {}", text);
    }

    let json: serde_json::Value = response.json().await.context("Parse response")?;
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No content in response"))?
        .trim()
        .to_string();

    Ok(content)
}

/// Infer genre/speed/harmony/drop from protein function narrative
pub async fn infer_beat_design(
    req: web::Json<InferBeatDesignRequest>,
) -> Result<HttpResponse, Error> {
    match infer_beat_design_from_function(&req.selected_function).await {
        Ok(inferred) => Ok(HttpResponse::Ok().json(inferred)),
        Err(e) => {
            log::error!("Infer beat design failed: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("{}", e)
            })))
        }
    }
}

/// Internal helper: call Groq to infer genre/bpm/key/melodic from function text
async fn infer_beat_design_from_function(function_text: &str) -> Result<InferredBeatDesign> {
    let api_key = std::env::var("GROQ_API_KEY").context("GROQ_API_KEY not set")?;

    let prompt = format!(
        r#"Given this song-generating instruction (defines the feeling and journey the listener will experience), infer Drum & Bass beat design parameters.

Instruction: {}

Respond with ONLY valid JSON (no markdown, no explanation):
{{"genre": "liquid"|"jump_up"|"neurofunk"|"dancefloor"|"jungle", "bpm": 160-185, "key": "C"|"Am"|"Dm"|"Em"|"Gm"|etc or null, "melodic": true|false}}

Genre mapping: liquid=soulful/melodic, jump_up=high-energy/wobble, neurofunk=dark/techy, dancefloor=anthemic, jungle=breakbeat-heavy."#,
        function_text
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("HTTP client")?;

    let body = serde_json::json!({
        "model": "groq/compound",
        "messages": [
            {"role": "user", "content": prompt}
        ],
        "max_tokens": 256,
        "temperature": 0.3
    });

    let response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .context("Groq request")?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("Groq API error: {}", text);
    }

    let json: serde_json::Value = response.json().await.context("Parse response")?;
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No content in response"))?
        .trim();

    let content = content
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let inferred: InferredBeatDesign = serde_json::from_str(content)
        .context("Parse inferred JSON")?;

    Ok(inferred)
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
                    pdb_id: protein.metadata.pdb_id.clone(),
                    title: protein.metadata.title.clone(),
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

    // Build framework for LLM (with genre params; infer from selected_function if set)
    let (bpm, genre_params) = resolve_genre_params(&req).await.map_err(|e| {
        log::error!("Failed to resolve genre params: {}", e);
        error::ErrorInternalServerError(format!("{}", e))
    })?;
    let framework = ProteinFramework::from_protein_with_params(&protein, bpm, genre_params.as_ref())
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
    let instruction = req.orchestration_instruction.as_deref().or(req.selected_function.as_deref());
    match call_groq_api(&framework_json, instruction).await {
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

/// Build GenreParams from request.
fn build_genre_params(req: &GenerateRequest) -> Option<GenreParams> {
    let genre = req.genre.as_deref().and_then(DnBGenre::from_str)?;
    let mut params = GenreParams::new(genre);
    if let Some(k) = &req.key {
        params.key = Some(k.clone());
    }
    if let Some(o) = req.octave {
        params.octave = Some(o.clamp(2, 5));
    }
    if let Some(m) = req.melodic {
        params.melodic = m;
    }
    Some(params)
}

/// Resolve (bpm, genre_params) from request. When selected_function is set and no explicit genre, infers via LLM.
async fn resolve_genre_params(req: &GenerateRequest) -> Result<(u16, Option<GenreParams>)> {
    if req.genre.is_none() && req.key.is_none() {
        if let Some(ref func) = req.selected_function {
            let inferred = infer_beat_design_from_function(func).await?;
            let genre = DnBGenre::from_str(&inferred.genre).unwrap_or(DnBGenre::Neurofunk);
            let mut params = GenreParams::new(genre);
            params.key = inferred.key.clone();
            params.melodic = inferred.melodic;
            params.octave = Some(3);
            return Ok((inferred.bpm, Some(params)));
        }
    }
    let bpm = req.bpm.unwrap_or(174);
    let params = build_genre_params(req);
    Ok((bpm, params))
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

    let (bpm, genre_params) = resolve_genre_params(&req).await.map_err(|e| {
        log::error!("Failed to resolve genre params: {}", e);
        error::ErrorInternalServerError(format!("{}", e))
    })?;
    let mapped = match protein_to_primitives(&protein, bpm, genre_params.as_ref()) {
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
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub octave: Option<u8>,
    #[serde(default)]
    pub melodic: Option<bool>,
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
        genre: req.genre.clone(),
        key: req.key.clone(),
        octave: req.octave,
        melodic: req.melodic.unwrap_or(false),
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

    let (bpm, genre_params) = resolve_genre_params(&req).await.map_err(|e| {
        log::error!("Failed to resolve genre params: {}", e);
        error::ErrorInternalServerError(format!("{}", e))
    })?;
    let framework = ProteinFramework::from_protein_with_params(&protein, bpm, genre_params.as_ref())
        .map_err(|e| {
            log::error!("Failed to build framework: {}", e);
            error::ErrorInternalServerError(format!("Failed to build framework: {}", e))
        })?;

    let framework_json = framework.to_json()
        .map_err(|e| {
            log::error!("Failed to serialize framework: {}", e);
            error::ErrorInternalServerError(format!("Failed to serialize: {}", e))
        })?;

    let instruction = req.orchestration_instruction.as_ref()
        .or(req.selected_function.as_ref());
    let instruction_note = instruction
        .map(|s| format!("\n\nORCHESTRATION INSTRUCTION (how to arrange drums, bass, melodic layers):\n{}\n\nFollow this instruction. Blend beat templates. Keep protein mapping intact but adlib on arrangement.", s))
        .unwrap_or_default();

    let user_content = format!(
        "Arrange these pre-mapped Strudel primitives into Drum & Bass Strudel code.{}Framework (primitives with pattern, gain, layer):\n{}\n\nOutput a SUBSTANTIAL arrangement. CRITICAL: Build each layer as const (const drums = ..., const bass = ..., const pad = ..., const lead = ...), then output ONE final stack(drums, bass, pad, lead) — in Strudel JS mode only the last expression plays, so multiple separate stack() calls will NOT work. Include setcps, register() if using acidenv, drums, bass, pads, lead. Use ._pianoroll() on melodic layers. Output ONLY valid Strudel JS code. NO d1 $. Blend elements from the beat templates.",
        instruction_note,
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
                {"role": "system", "content": dnb_system_prompt()},
                {"role": "user", "content": user_content}
            ],
            "max_tokens": 4096,
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

/// Strudel knowledge base - local reference for clean executable code
const STRUDEL_KNOWLEDGE: &str = include_str!("../strudel_knowledge.md");

/// Beat templates for inspiration - blend and mix for complete works
const BEAT_TEMPLATES: &str = include_str!("../beat_templates.md");

/// DnB arrangement system prompt for LLM (includes knowledge base + beat templates)
fn dnb_system_prompt() -> String {
    format!(r#"You are a Drum & Bass music arranger for Strudel.cc. Use this knowledge base for clean, executable code:

{}

---

BEAT TEMPLATES — blend and mix these for inspiration. Adapt to the framework and genre:
{}

---

RULES:
- Use ONLY the provided primitives from the framework
- CRITICAL: In Strudel JS mode, ONLY THE LAST EVALUATED EXPRESSION PLAYS. Multiple separate stack() calls will NOT all play — each replaces the previous. You MUST: (1) build each layer as const (e.g. const drums = stack(...), const bass = n(...)), (2) output ONE final stack(drums, bass, pad, lead) at the end. This is the ONLY way all layers play together.
- BLEND elements from the beat templates above — mix kicks, basses, pads, percussion from different templates for unique results
- If framework has genre, key, octave, melodic: match that subgenre style (liquid/jump_up/neurofunk/dancefloor/jungle)
- Strudel default REPL is JS: NO d1 $, NO stack([...]). Use const for layers, then ONE stack(layer1, layer2, ...).
- For melodic layers: n(\"0 2 4 6\").scale(\"C:minor\").s(\"sawtooth\").gain(slider(...)). Add ._pianoroll() if desired.
- No markdown, no comments that break parsing."#, STRUDEL_KNOWLEDGE, BEAT_TEMPLATES)
}

/// Call Groq API with protein framework to generate Strudel code
async fn call_groq_api(framework_json: &str, instruction: Option<&str>) -> Result<String> {
    let api_key = std::env::var("GROQ_API_KEY")
        .context("GROQ_API_KEY environment variable not set")?;

    let instruction_note = instruction
        .map(|s| format!("\n\nORCHESTRATION INSTRUCTION (how to arrange drums, bass, melodic layers):\n{}\n\nFollow this instruction. Blend beat templates. Keep protein mapping intact but adlib on arrangement.", s))
        .unwrap_or_default();

    let user_content = format!(
        "Arrange these pre-mapped Strudel primitives into Drum & Bass Strudel code.{}Framework (primitives with pattern, gain, layer):\n{}\n\nOutput a SUBSTANTIAL arrangement. CRITICAL: Build each layer as const (const drums = ..., const bass = ..., const pad = ..., const lead = ...), then output ONE final stack(drums, bass, pad, lead) — in Strudel JS mode only the last expression plays, so multiple separate stack() calls will NOT work. Include setcps, register() if using acidenv, drums, bass, pads, lead. Use ._pianoroll() on melodic layers. Output ONLY valid Strudel JS code. NO d1 $. Blend elements from the beat templates.",
        instruction_note,
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
            {"role": "system", "content": dnb_system_prompt()},
            {"role": "user", "content": user_content}
        ],
        "max_tokens": 4096,
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
    let mut content = if content.starts_with("```") {
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

    // Fix common LLM mistake: (5,8)bd -> bd(5,8) (euclidean must be sound first)
    content = fix_euclidean_order(&content);
    // Convert Tidal syntax to Strudel JS: remove d1 $, stack([...]) -> stack(...)
    content = tidal_to_js_syntax(&content);

    Ok(content)
}

/// Convert Tidal (Haskell) syntax to Strudel JS mode for default REPL.
fn tidal_to_js_syntax(code: &str) -> String {
    use regex::Regex;
    let mut out = code.to_string();
    // Remove d1 $, d2 $, etc.
    if let Ok(re) = Regex::new(r"\bd\d+\s*\$?\s*") {
        out = re.replace_all(&out, "").to_string();
    }
    // stack([ -> stack( (variadic, no array)
    out = out.replace("stack([", "stack(");
    // Replace only the last ]) to avoid breaking nested stacks
    if let Some(pos) = out.rfind("])") {
        out = format!("{}){}", &out[..pos], &out[pos + 2..]);
    }
    out
}

/// Fix reversed euclidean patterns: (beats,segments)sound -> sound(beats,segments)
fn fix_euclidean_order(code: &str) -> String {
    let sounds = ["bd", "sd", "hh", "cp", "rim", "oh", "perc"];
    let mut out = code.to_string();
    for sound in sounds {
        for beats in 2..=7 {
            for segments in 4..=16 {
                let wrong = format!("\"({},{}){}\"", beats, segments, sound);
                let right = format!("\"{}({},{})\"", sound, beats, segments);
                out = out.replace(&wrong, &right);
            }
        }
    }
    out
}
