# ProDnB Web Server

A web interface for converting PDB protein files to Strudel Drum & Bass music code.

## Features

- **Drag & Drop PDB Upload**: Upload protein structure files (.pdb, .ent, .cif)
- **AI-Powered Generation**: Uses Groq's Compound AI model to create aesthetically pleasing Strudel code
- **Simple UI**: Clean, modern interface called "ProDnB"
- **Copy-Paste Ready**: Generated code appears in a code block for easy copying to strudel.cc
- **Real-time Feedback**: Shows protein statistics (chains, residues, atoms)

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

1. **PDB Parsing**: The server parses the uploaded PDB file to extract protein structure data
2. **Feature Extraction**: Computes structural features (chains, residues, atom composition)
3. **Framework Generation**: Creates a compact JSON representation for the LLM
4. **AI Processing**: Sends the framework to Groq's Compound model with music generation instructions
5. **Strudel Code**: The AI returns valid Strudel code for Drum & Bass music

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

### POST /api/generate
Generate Strudel code from an uploaded PDB file.

**Request**:
```json
{
  "file_path": "/tmp/tmpXXX.pdb"
}
```

**Response**:
```json
{
  "success": true,
  "code": "setcps(0.7)\nd1 $ sound \"bd sd hh\"...",
  "chain_count": 2,
  "residue_count": 150
}
```

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
