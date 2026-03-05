# ProDnB Web Server

A web interface for converting PDB protein files to Strudel Drum & Bass music code.

**For full documentation including the detailed PDB mapping algorithm, see the [main README](../README.md).**

## Features

- **Drag & Drop PDB Upload**: Upload protein structure files (.pdb, .ent, .cif)
- **Two-Stage Pipeline**: Deterministic PDB mapping → LLM arrangement (Groq Compound)
- **Streaming Generation**: SSE streaming for real-time code output
- **Piano Roll**: Visual grid of mapped primitives
- **Intensity Sliders**: Kick, Snare, Hi-Hats, Energy controls
- **Copy-Paste Ready**: Generated code for [strudel.cc](https://strudel.cc)

## Prerequisites

- Rust 1.70 or later
- Groq API key (get one at https://console.groq.com/keys)

## Setup

1. Copy the example environment file:
   ```bash
   cp prodnb-web/.env.example .env
   ```

2. Edit `.env` and add your Groq API key:
   ```
   GROQ_API_KEY=gsk_your_actual_api_key_here
   ```

3. Build the project:
   ```bash
   cargo build --release --package prodnb-web
   ```

## Running the Server

Start the server:
```bash
cargo run --package prodnb-web
```

Or run the release binary:
```bash
./target/release/prodnb-web
```

The server will start on `http://127.0.0.1:8080` by default.

## Usage

1. Open your browser to `http://127.0.0.1:8080`

2. **Upload a PDB file**:
   - Drag and drop your PDB file into the upload area
   - Or click to browse and select a file

3. **Generate Strudel code**:
   - Click the "Generate Strudel Code" button
   - Wait for the AI to process your protein structure
   - The generated code will appear in the code block

4. **Use the code**:
   - Click "Copy Code" to copy to clipboard
   - Paste into [strudel.cc](https://strudel.cc) to play the music

## How It Works

1. **PDB Parsing**: Parse uploaded PDB to extract atoms (element, b-factor, occupancy, chain)
2. **Stage 1 – Mapping**: Deterministic algorithm maps elements → sounds, b-factor variance → euclidean rhythm, occupancy → gain
3. **Primitives**: Output structured JSON (kick, snare, hats, perc) for piano roll and sliders
4. **Stage 2 – LLM**: Framework (primitives + features) sent to Groq Compound with DnB system prompt
5. **Strudel Code**: AI returns valid Strudel code; or assemble from primitives + sliders (no LLM)

## API Endpoints

### GET /
Returns the main web UI page.

### GET /health
Health check endpoint.

### POST /api/upload
Upload a PDB file.

**Request**: multipart/form-data with field `pdb_file`

**Response**:
```json
{
  "success": true,
  "message": "Successfully loaded PDB file",
  "file_path": "/tmp/tmpXXX.pdb",
  "chain_count": 2,
  "residue_count": 150,
  "atom_count": 1200
}
```

### POST /api/map
Map PDB to Strudel primitives (deterministic, no LLM).

**Request**: `{ "file_path": "...", "bpm": 174 }`

**Response**: `MappedOutput` JSON with `primitives`, `rhythm_seed`, `chain_lengths`, `element_counts`

### POST /api/assemble
Assemble Strudel from primitives + optional slider values (no LLM).

**Request**: `{ "primitives": [...], "tempo": 174, "sliders": { "kick": 0.9, "snare": 0.85, "hats": 0.6, "energy": 1.0 } }`

### POST /api/generate
Generate Strudel code via LLM (non-streaming).

**Request**: `{ "file_path": "/tmp/tmpXXX.pdb" }`

**Response**: `{ "success": true, "code": "..." }`

### POST /api/generate/stream
Generate Strudel code via LLM (SSE streaming).

## Environment Variables

- `GROQ_API_KEY`: Your Groq API key (required)
- `BIND_ADDRESS`: Server bind address (default: 127.0.0.1:8080)
- `RUST_LOG`: Log level (default: info)

## Development

Run with debug logging:
```bash
RUST_LOG=debug cargo run --package prodnb-web
```

Run tests:
```bash
cargo test --package prodnb-web
```

## Architecture

```
prodnb-web/
├── src/
│   ├── main.rs          # Server entry point
│   ├── lib.rs           # Library exports
│   ├── handlers.rs      # HTTP request handlers
│   └── templates.rs     # Askama template structs
├── templates/
│   └── index.html       # Main UI template
├── static/
│   ├── css/
│   │   └── style.css    # UI styles
│   └── js/
│       └── app.js       # Frontend JavaScript
└── Cargo.toml
```

## License

MIT License - see workspace Cargo.toml
