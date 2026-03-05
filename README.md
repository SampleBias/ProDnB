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

### Element-to-Sound Mapping (Base)

The base mapping: each chemical element maps to a Strudel drum sample.

| PDB Element | Chemical | Strudel Sound | Rationale |
|-------------|----------|---------------|-----------|
| **C** | Carbon | `bd` (bass drum) | Backbone; most abundant; low, driving |
| **N** | Nitrogen | `sd` (snare) | Peptide bonds; sharp, accented |
| **O** | Oxygen | `hh` (hi-hat) | Common; high, rhythmic |
| **S** | Sulfur | `cp` (clap) | Cysteine; distinct, crisp |
| **P** | Phosphorus | `rim` (rimshot) | Nucleic acids; metallic |
| **H** | Hydrogen | `~` (rest) | Too numerous; mapped to silence |
| **Other** | Fe, Zn, etc. | `perc` | Miscellaneous percussion |

### Dynamic Element Mapping (Phases 1–3)

ProDnB uses a **dynamic mapping** algorithm for variation. Same PDB → same output (deterministic), but richer patterns.

**Phase 1: Element pools, B-factor, occupancy**

- **Element pools**: Each base sound has variants. C → `[bd, bd, perc]`; N → `[sd, sd, cp]`; O → `[hh, hh, oh]`. Atom index rotates within the pool.
- **B-factor substitution**: High B-factor (> 40) = structural flexibility. ~25% of flexible atoms use a "flex" variant (e.g. bd → perc, hh → oh).
- **Occupancy-based rest**: Very low occupancy (< 0.25) → deterministic rest (`~`) for ~33% of those atoms.

**Phase 2: Residue and chain context**

- **Residue-type bias**: Hydrophobic (ALA, VAL, …) → bd; Charged (ARG, LYS, …) → sd; Aromatic (PHE, TYR, …) → cp. Applied ~20% of the time for C/N/O.
- **Chain-index rotation**: Chain 0 = default; Chain 1 = occasional bd→perc; Chain 2 = occasional hh→oh.

**Phase 3: Configurable thresholds**

`MappingConfig` (defaults):

| Parameter | Default | Description |
|-----------|---------|-------------|
| `b_factor_flex_threshold` | 40 | B-factor above which flex variant may apply |
| `occupancy_rest_threshold` | 0.25 | Occupancy below which rest may apply |
| `occupancy_rest_mod` | 1 | Rest probability (0–2) when below threshold |

**API**: `element_to_sound_dynamic(ctx, atom_index, genre, &config)` — use for rhythm seed and layered output. `Protein::all_atoms_with_context()` yields `AtomContext` (atom + residue_name, chain_id, chain_index, residue_seq).

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

A **rhythm seed** is built by sampling atoms with **dynamic mapping**:

1. Iterate atoms with `all_atoms_with_context()` (residue + chain info)
2. Filter out hydrogen
3. Sample every Nth atom (step = `atoms.len() / 24`, clamped 1–4)
4. Take up to 24 atoms
5. Map each via `element_to_sound_dynamic()` — pools, B-factor, occupancy, residue, chain
6. Join with spaces

Example: `"bd perc sd hh bd ~ sd cp bd oh ..."` (varied by structure)

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
| `stack(...)` | `stack(s("bd"), s("hh*8"))` | Layer patterns (JS variadic, no array) |
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

### Step-by-Step Workflow

1. **Step 1: Upload PDB** — Drag & drop (.pdb, .ent, .cif), max 10MB
2. **Step 2: Protein Function** — Click "Find the function, find the beat" to search via SERPAPI (Google). Shows top 3 results. Select one as your song-generating instruction.
3. **Continue the journey** — Generates an orchestration instruction (anthropomorphizing the protein, poetic interpretation, musical metaphors, technical guidance). Editable before Generate.
4. **Step 3: Genre & Tonal Options** — Auto-populated from AI when you select a function (subgenre, key, octave, melodic). Edit as needed.
5. **Step 4: Generate** — Strudel code via LLM, using orchestration instruction + beat templates + protein mapping.
6. **Step 5: Copy to Strudel** — Copy code to [strudel.cc](https://strudel.cc)

### Other Features

- **Map**: Automatic deterministic mapping on upload
- **Generate (Stream)**: SSE streaming from Compound API
- **Piano Roll**: Visual grid of primitives (layers × 16 steps)
- **Assemble**: Sliders update gain; re-assemble without LLM
- **Playback fixes**: Generated code is post-processed to (1) inject `register('acidenv', ...)` when `.acidenv()` is used, (2) ensure a final `stack(drums, bass, pad, lead)` so all layers play, (3) use `triangle` for melodic synths (most reliable; sine/sawtooth can be silent)
- **Representation key**: Every generated code includes a comment block mapping each layer (drums, bass, pad, lead) to the protein's biological function — so the DJ knows what each slider controls (e.g. "BASS: oxygen transport pulse — slider shapes delivery intensity" for hemoglobin)

---

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/upload` | POST | Upload PDB file (multipart) |
| `/api/protein-function` | POST | Fetch protein function via SERPAPI (top 3 results) |
| `/api/infer-beat-design` | POST | Infer genre/BPM/key/melodic from function text |
| `/api/generate-orchestration-instruction` | POST | Generate orchestration instruction (anthropomorphic + poetic) |
| `/api/map` | POST | Map PDB → primitives JSON |
| `/api/assemble` | POST | Assemble Strudel from primitives + sliders |
| `/api/generate` | POST | Generate Strudel via LLM (non-streaming) |
| `/api/generate/stream` | POST | Generate Strudel via LLM (SSE streaming) |

---

## Setup

1. Set `GROQ_API_KEY` in `.env`
2. Set `SERP_API_Key` in `.env` (for protein function lookup)
3. Build: `cargo build --release --package prodnb-web`
4. Run: `cargo run --package prodnb-web`
5. Open `http://127.0.0.1:8080`

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
│   ├── strudel_knowledge.md  # Local knowledge base for LLM
│   ├── beat_templates.md     # Beat templates (liquid, acid, flamenco, etc.) for blending
│   ├── src/handlers.rs   # API + DnB system prompt
│   ├── templates/
│   └── static/
└── README.md
```

## Playback Troubleshooting

If pasted code doesn't play in Strudel.cc:

1. **`.acidenv is not a function`** — Strudel has no built-in acidenv. ProDnB auto-injects `register('acidenv', ...)` when the LLM uses `.acidenv()` on bass. If you still see this, add manually after `setcps()`:
   ```javascript
   register('acidenv', (x, pat) => pat.lpf(100).lpenv(x * 9).lps(0.2).lpd(0.12));
   ```
2. **Nothing plays** — In Strudel JS mode, only the last expression plays. ProDnB auto-appends `stack(drums, bass, pad, lead)` when layers are defined but no final stack. Ensure your output ends with a single `stack(...)` of all layers.
3. **Melodic layers (n()) silent** — Sample engine works, synth engine may need a click. ProDnB uses `triangle` (Strudel's default, most reliable). If `sine`/`sawtooth` are silent, click the Strudel play area first (browser autoplay), or change `.s("sine")` to `.s("triangle")`.

## Strudel Knowledge Base

`prodnb-web/strudel_knowledge.md` is a local reference for the LLM. It includes:

- **Critical syntax**: In Strudel JS mode, only the last evaluated expression plays. Build layers as `const`, then output ONE `stack(drums, bass, pad, lead)`.
- **Euclidean rhythms**: `bd(5,8)` not `(5,8)bd`
- **Drum samples**: bd, sd, hh, cp, rim, perc, etc.

`prodnb-web/beat_templates.md` provides proven beat patterns (Deep Liquid, Acid/303, Flamenco, Drop/Tension) that the LLM blends and adapts for unique arrangements.

## Orchestration Instruction

When you "Continue the journey," the LLM generates an orchestration instruction that blends:

1. **Anthropomorphizing** — e.g. hemoglobin = "oxygen carrier that fires energy into the system"
2. **Poetic interpretation** — evocative, metaphorical language
3. **Musical metaphors** — transport → "energy surges", binding/release → "tension and release"
4. **Technical guidance** — BPM, rhythm feel, bass character, drop structure

The instruction is editable before Generate and drives the Strudel.cc code generation.

---

## License

MIT License
