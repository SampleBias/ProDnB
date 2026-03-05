# ProDnB

**Where Protein Topology Becomes the Drop.**

ProDnB converts PDB (Protein Data Bank) structure files into Drum & Bass music patterns using a two-stage pipeline: (1) deterministic PDB-to-Strudel mapping, then (2) LLM arrangement via Groq's Compound API. Output is valid [Strudel.cc](https://strudel.cc) code for live coding.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         PDB to DnB Strudel Pipeline                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────┐    ┌─────────────────────┐    ┌─────────────────────────┐  │
│  │  PDB File    │───▶│  Stage 1: Mapping   │───▶│  Strudel Primitives     │  │
│  │  (.pdb)      │    │  (Deterministic)   │    │  (JSON)                 │  │
│  └──────────────┘    └─────────────────────┘    └───────────┬─────────────┘  │
│                                      │                       │               │
│                                      │                       ▼               │
│                                      │              ┌─────────────────────┐ │
│                                      │              │  Piano Roll +        │ │
│                                      │              │  Intensity Sliders   │ │
│                                      │              └─────────────────────┘ │
│                                      │                       │               │
│                                      ▼                       │               │
│  ┌──────────────────────────────────────────────────────────┴─────────────┐  │
│  │  Stage 2: LLM Arrangement (Groq Compound API)                         │  │
│  │  • DnB system prompt                                                  │  │
│  │  • Streaming SSE output                                               │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                      │                                       │
│                                      ▼                                       │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │  Strudel Code (copy to strudel.cc)                                    │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## PDB Mapping (Detailed)

The mapping algorithm translates protein structure data into Strudel.cc drum patterns. It is **fully deterministic**: the same PDB file always produces the same primitives.

### PDB File Structure (Input)

A PDB file describes a 3D protein structure. ProDnB parses:

| PDB Record | ProDnB Use |
|------------|------------|
| `ATOM` / `HETATM` | Atom coordinates and metadata |
| `HEADER` | Optional PDB ID |
| `TITLE` | Optional title |

Each atom line provides:

| Field | Column | Description | Mapping Use |
|-------|--------|-------------|-------------|
| Element | 76-77 | Chemical element (C, N, O, S, P, H, etc.) | **Primary mapping** → drum sound |
| B-factor | 60-65 | Temperature factor (structural flexibility) | **Variance** → Euclidean rhythm density |
| Occupancy | 54-59 | Site occupancy (0–1) | **Average** → gain/intensity |
| Chain ID | 21 | Polypeptide chain | Chain count → polyrhythmic layers |
| Residue sequence | 22-26 | Residue index | Chain length → hi-hat density |

### Element-to-Sound Mapping

The core mapping: each chemical element maps to a Strudel drum sample.

| PDB Element | Chemical | Strudel Sound | Rationale |
|-------------|----------|---------------|-----------|
| **C** | Carbon | `bd` (bass drum) | Backbone; most abundant; low, driving |
| **N** | Nitrogen | `sd` (snare) | Peptide bonds; sharp, accented |
| **O** | Oxygen | `hh` (hi-hat) | Common; high, rhythmic |
| **S** | Sulfur | `cp` (clap) | Cysteine; distinct, crisp |
| **P** | Phosphorus | `rim` (rimshot) | Nucleic acids; metallic |
| **H** | Hydrogen | `~` (rest) | Too numerous; mapped to silence |
| **Other** | Fe, Zn, etc. | `perc` | Miscellaneous percussion |

**Implementation** (`prodnb-core/src/strudel.rs`):

```rust
pub fn element_to_sound(element: &str) -> &'static str {
    match element.to_uppercase().as_str() {
        "C" => "bd",
        "N" => "sd",
        "O" => "hh",
        "S" => "cp",
        "P" => "rim",
        "H" => "~",
        _ => "perc",
    }
}
```

### B-Factor Variance → Euclidean Rhythm

The **B-factor** (temperature factor) indicates atom mobility. High variance = more structural variation = denser rhythms.

| B-Factor Variance | Euclidean (beats, segments) | Effect |
|-------------------|----------------------------|--------|
| > 50 | (5, 8) | Dense kick pattern |
| > 20 | (4, 8) | Medium density |
| ≤ 20 | (3, 8) | Sparse pattern |

Strudel Euclidean syntax: `bd(5,8)` = 5 hits distributed over 8 steps.

### Occupancy → Gain

**Occupancy** (0–1) indicates how fully an atom site is occupied. The algorithm:

1. Computes average occupancy across all non-H atoms
2. Maps to gain: `base_gain = (0.5 + avg_occupancy * 0.5).clamp(0.3, 1.0)`
3. Applies per-layer multipliers: kick 0.95×, snare 0.9×, hats 0.6×, perc 0.5×

### Chain Length → Hi-Hat Density

Longer polypeptide chains → denser hi-hat patterns:

- `hat_mult = (max_chain_length / 4).clamp(1, 8)`
- Pattern: `hh*N` (e.g. `hh*8` = 8 hi-hats per cycle = 16th notes at 4/4)

### Rhythm Seed

A **rhythm seed** is built by sampling atoms along the backbone:

1. Filter out hydrogen
2. Sample every Nth atom (step = `atoms.len() / 24`, clamped 1–4)
3. Take up to 24 atoms
4. Map each element → sound, join with spaces

Example: `"bd bd sd hh bd ~ sd cp bd hh ..."`

This seed is used as:
- A structural hint for the LLM
- An optional extra percussion layer when Carbon count > 0

### Output: Strudel Primitives JSON

The mapping produces a `MappedOutput`:

```json
{
  "tempo": 174,
  "primitives": [
    {
      "type": "euclidean",
      "sound": "bd",
      "beats": 5,
      "segments": 8,
      "gain": 0.9,
      "layer": "kick"
    },
    {
      "type": "drum",
      "pattern": "sd ~ ~ sd",
      "gain": 0.85,
      "layer": "snare"
    },
    {
      "type": "drum",
      "pattern": "hh*8",
      "gain": 0.6,
      "layer": "hats"
    },
    {
      "type": "drum",
      "pattern": "bd bd sd hh bd ~ sd",
      "gain": 0.5,
      "layer": "perc"
    }
  ],
  "rhythm_seed": "bd bd sd hh bd ~ sd cp bd hh ...",
  "chain_lengths": [120, 85, 42],
  "element_counts": {"C": 450, "N": 120, "O": 180, "S": 8, "P": 2}
}
```

### Default Primitives (Fallback)

When the protein has no mappable atoms, default DnB primitives are used:

- Kick: `bd(5,8)` euclidean, gain 0.9
- Snare: `sd ~ ~ sd` (2 and 4), gain 0.85
- Hats: `hh*8`, gain 0.6

---

## Strudel.cc Syntax Reference

ProDnB outputs valid Strudel (TidalCycles) mini-notation:

| Syntax | Example | Meaning |
|--------|---------|---------|
| `s("pattern")` | `s("bd sd hh")` | Sound pattern |
| `~` | `sd ~ ~ sd` | Rest (silence) |
| `*N` | `hh*8` | N events per cycle |
| `[]` | `[bd sd]` | Subdivide time |
| `(beats,segments)` | `bd(5,8)` | Euclidean rhythm |
| `gain(N)` | `.gain(0.8)` | Volume 0–1 |
| `stack([...])` | `stack(s("bd"), s("hh*8"))` | Layer patterns |
| `setcps(N)` | `setcps(0.725)` | Tempo (174 BPM ≈ 0.725) |

**Drum samples**: `bd`, `sd`, `hh`, `cp`, `rim`, `oh` (open hat), `perc`, `misc`, `fx`

---

## DnB Arrangement (LLM System Prompt)

The LLM receives the primitives and a DnB-specific system prompt:

**Requirements:**
- Tempo: 174 BPM (`setcps(0.725)`)
- Structure: Intro → Buildup → Drop → Breakdown → Drop → Outro
- Kick on 1 and 3+, snare on 2 and 4, hi-hats on 16ths (`hh*8`)
- Syncopation, offbeat emphasis, ghost snares (`~ sd`)
- Phases: 16 or 32 bars per section
- 4/4 time, 16th-note subdivisions

**Constraints:** Use only provided primitives; arrange with `stack()`, `gain()`, `slow()`, `fast()`.

---

## Web UI Features

- **Upload**: Drag & drop PDB (.pdb, .ent, .cif), max 10MB
- **Map**: Automatic deterministic mapping on upload
- **Generate (LLM)**: Non-streaming, full response
- **Generate (Stream)**: SSE streaming from Compound API
- **Piano Roll**: Visual grid of primitives (layers × 16 steps)
- **Intensity Sliders**: Kick, Snare, Hi-Hats, Energy (0–100%)
- **Assemble**: Sliders update gain; re-assemble without LLM

---

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/upload` | POST | Upload PDB file (multipart) |
| `/api/map` | POST | Map PDB → primitives JSON |
| `/api/assemble` | POST | Assemble Strudel from primitives + sliders |
| `/api/generate` | POST | Generate Strudel via LLM (non-streaming) |
| `/api/generate/stream` | POST | Generate Strudel via LLM (SSE streaming) |

---

## Setup

1. Set `GROQ_API_KEY` in `.env`
2. Build: `cargo build --release --package prodnb-web`
3. Run: `cargo run --package prodnb-web`
4. Open `http://127.0.0.1:8080`

---

## Project Structure

```
ProDnB/
├── prodnb-core/          # PDB parsing, mapping, framework
│   ├── src/
│   │   ├── protein.rs    # PDB parser
│   │   ├── features.rs   # Structural features
│   │   ├── strudel.rs    # PDB → Strudel mapping
│   │   └── framework.rs  # LLM framework (primitives + features)
│   └── ...
├── prodnb-web/           # Web server
│   ├── src/handlers.rs   # API + DnB system prompt
│   ├── templates/
│   └── static/
└── README.md
```

---

## License

MIT License
