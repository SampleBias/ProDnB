# ProDnB - Development Context

## Project Purpose
ProDnB is a unique web application that converts PDB (Protein Data Bank) files into Strudel music code for Drum & Bass tracks. It uses protein structure data as input and leverages Groq's Compound AI model to generate aesthetically pleasing musical patterns that can be played on strudel.cc.

## Architecture Overview
- **Project Type**: Rust Web Application with AI Integration
- **Development Status**: Active Development - WebUI Complete
- **Context Tracking**: Integrated with Vybrid development workflow

## Technology Stack
- **Backend**: Rust with Actix-web framework
- **Frontend**: HTML, CSS, JavaScript (Vanilla)
- **Templating**: Askama
- **AI Integration**: Groq Cloud API (Compound model)
- **Protein Parsing**: pdbtbx crate
- **Async Runtime**: Tokio

## Project Structure

### Workspace Members
- `prodnb-core` - Core protein parsing, feature extraction, and framework generation
- `prodnb-web` - Web server and UI (NEW - primary focus)
- `prodnb-midi` - MIDI export functionality (retained)
- `prodnb-audio` - Audio synthesis (retained)
- `prodnb-cli` - Command-line interface (retained for optional use)

### Key Components

#### prodnb-web (Primary)
- `src/main.rs` - Actix-web server entry point
- `src/handlers.rs` - HTTP handlers for upload and generation
- `src/templates.rs` - Askama template structs
- `templates/index.html` - Main web UI
- `static/css/style.css` - Dark-themed modern UI styles
- `static/js/app.js` - Drag & drop, file upload, and copy functionality

#### prodnb-core (Core Logic)
- `src/protein.rs` - PDB file parsing
- `src/features.rs` - Protein feature extraction
- `src/framework.rs` - Framework generation for LLM
- `src/strudel.rs` - Strudel code mapping utilities

## Getting Started

### Prerequisites
- Rust 1.70 or later
- Groq API key (get one at https://console.groq.com/keys)
- PDB protein files (.pdb, .ent, .cif format)

### Installation
1. Clone the repository
2. Copy environment template: `cp prodnb-web/.env.example .env`
3. Edit `.env` and add your Groq API key
4. Build: `cargo build --release --package prodnb-web`

### Running the Project
```bash
# Run the web server
cargo run --package prodnb-web

# Or run from release binary
./target/release/prodnb-web
```

The web interface will be available at `http://127.0.0.1:8080`

### Usage Workflow
1. Open web UI in browser
2. Upload a PDB file (drag & drop or click to browse)
3. View protein statistics (chains, residues, atoms)
4. Click "Generate Strudel Code" to generate music
5. Copy the generated code to clipboard
6. Paste into strudel.cc to play the music

## Development Status

### Completed Features
- [x] Web server with Actix-web
- [x] File upload with drag & drop support
- [x] PDB parsing and validation
- [x] Protein feature extraction
- [x] Framework generation for LLM
- [x] Groq Compound API integration
- [x] Strudel code generation
- [x] Modern dark-themed UI
- [x] Copy to clipboard functionality
- [x] Real-time status messages
- [x] Protein statistics display

### Removed Features
- [x] TUI (Terminal User Interface) - Removed as per requirements
- [x] prodnb-tui crate - Excluded from workspace

### Retained Features
- [x] prodnb-core - Essential for protein processing
- [x] prodnb-cli - Available for command-line usage (optional)
- [x] prodnb-midi - MIDI export capabilities
- [x] prodnb-audio - Audio synthesis capabilities

## API Endpoints

### GET /
Returns the main web UI page

### GET /health
Health check endpoint

### POST /api/upload
Upload a PDB file
- Accepts: multipart/form-data with field `pdb_file`
- Returns: JSON with file path and protein statistics

### POST /api/generate
Generate Strudel code from uploaded PDB
- Accepts: JSON `{ "file_path": "/path/to/pdb" }`
- Returns: JSON with generated Strudel code

## Environment Variables

Required:
- `GROQ_API_KEY` - Groq Cloud API key for Compound model

Optional:
- `BIND_ADDRESS` - Server bind address (default: 127.0.0.1:8080)
- `RUST_LOG` - Log level (default: info)

## Key Context for AI Agents

### Development Workflow
- This project follows the Vybrid development methodology
- Three mandatory files are maintained: `tasks/todo.md`, `docs/activity.md`, and `docs/PROJECT_README.md`
- All development activities are tracked and documented systematically

### Recent Major Changes (2026-03-04)
- Removed TUI component completely (prodnb-tui)
- Created new prodnb-web crate with full web UI
- Implemented Groq Compound AI integration for music generation
- Built modern, responsive UI with drag & drop file upload
- Added copy-to-clipboard functionality for easy strudel.cc usage

### Design Philosophy
- Simple, clean UI called "ProDnB"
- Focus on user experience: upload → generate → copy → play
- No complex controls - the AI handles composition
- Protein structure drives musical composition
- Drum & Bass focus (default 174 BPM)

## Documentation Links
- [Task List](../tasks/todo.md) - Current development tasks and progress
- [Activity Log](activity.md) - Detailed timeline of all development activities
- [prodnb-web README](../prodnb-web/README.md) - Web server specific documentation

---
*Auto-generated by Vybrid*
*Created: 2026-03-04 13:55*
*Last Updated: 2026-03-04 18:55*
*Context Version: 2.0*
