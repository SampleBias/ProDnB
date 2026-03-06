# ProDnB

**A Pro-Bio (Algo)Synthetic Music Machine**

ProDnB converts PDB (Protein Data Bank) structure files into music using a two-stage pipeline: (1) deterministic PDB-to-Strudel mapping driven by the protein's 3D structural fingerprint, then (2) LLM arrangement via Groq's Compound API. Output is valid [Strudel.cc](https://strudel.cc) code for live coding.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         PDB to Music Pipeline                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────┐    ┌─────────────────────┐    ┌─────────────────────────┐ │
│  │  PDB File    │───▶│  Stage 1: Mapping   │───▶│  Strudel Primitives     │ │
│  │  (.pdb)      │    │  (Deterministic)    │    │  (JSON + visuals)       │ │
│  └──────────────┘    └─────────────────────┘    └───────────┬─────────────┘ │
│                              │                               │               │
│                     ┌────────┴────────┐                      │               │
│                     │  Structural     │                      │               │
│                     │  Fingerprint    │                      │               │
│                     │  + Genre Infer  │                      │               │
│                     └─────────────────┘                      │               │
│                              │                               ▼               │
│                              ▼                      ┌─────────────────────┐  │
│                     ┌─────────────────────┐         │  Piano Roll +       │  │
│                     │  ProteinFramework   │         │  Intensity Sliders  │  │
│                     │  (for LLM)          │         └─────────────────────┘  │
│                     └────────┬────────────┘                  │               │
│                              ▼                               │               │
│  ┌──────────────────────────────────────────────────────────┴─────────────┐ │
│  │  Stage 2: LLM Arrangement (Groq Compound API)                         │ │
│  │  • Genre-aware system prompt  • Streaming SSE  • Visual feedback      │ │
│  └──────────────────────────────────────────────────────────────────────┘ │
│                              │                                              │
│                              ▼                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │  Strudel Code (copy to strudel.cc) — with inline visuals            │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## PDB Mapping (Detailed)

The mapping algorithm translates protein structure data into Strudel.cc patterns. It is **fully deterministic**: the same PDB file always produces the same primitives.

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
| B-factor | 60-65 | Temperature factor (structural flexibility) | **Variance** → euclidean density; **contour** → per-step gain |
| Occupancy | 54-59 | Site occupancy (0–1) | **Average** → gain/intensity |
| Chain ID | 21 | Polypeptide chain | Chain count → polyrhythmic layers |
| Residue name | 17-20 | Amino acid type | Residue bias + genre inference |
| Residue sequence | 22-26 | Residue index | Chain length → hi-hat density |

### Element-to-Sound Mapping (Base)

Each chemical element maps to a Strudel drum sample:

| PDB Element | Chemical | Strudel Sound | Rationale |
|-------------|----------|---------------|-----------|
| **C** | Carbon | `bd` (bass drum) | Backbone; most abundant; low, driving |
| **N** | Nitrogen | `sd` (snare) | Peptide bonds; sharp, accented |
| **O** | Oxygen | `hh` (hi-hat) | Common; high, rhythmic |
| **S** | Sulfur | `cp` (clap) | Cysteine; distinct, crisp |
| **P** | Phosphorus | `rim` (rimshot) | Nucleic acids; metallic |
| **H** | Hydrogen | `~` (rest) | Too numerous; mapped to silence |
| **Other** | Fe, Zn, etc. | `perc` | Miscellaneous percussion |

This base mapping is adjusted per genre (see Genre System below).

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

---

## Structural Fingerprint

The structural fingerprint extracts five dimensions of 3D geometry from the protein backbone, producing a **rhythmic identity unique to each fold**:

### 1. Distance Rhythm

Inter-CA atom distances along the backbone, normalized 0–1:
- **0.0** = tight helix packing (~3.8Å) → subdivided hits `[bd sd]`
- **1.0** = fully extended loop (~7.5Å) → rests and sparse hits

### 2. B-Factor Contour

Per-residue B-factors normalized within the protein (0 = most rigid, 1 = most flexible). Used as per-step gain pattern on hi-hats and perc layers:
- Rigid regions → loud, crisp hits
- Flexible regions → soft, flowing hits

### 3. Backbone Motifs (Secondary Structure)

Each residue classified as Helix, Sheet, or Coil using CA geometry:
- **Helix**: CA_i to CA_i+3 < 6Å
- **Sheet**: CA_i to CA_i+2 = 6–7.5Å
- **Coil**: Everything else

The motif composition drives the snare/drum pattern:

| Dominant Motif | Pattern Style | Example |
|----------------|--------------|---------|
| Helix (>50%) | Driving 16th-note feel | `[bd bd] sd [hh bd] sd` |
| Sheet (>40%) | Sparse staccato | `bd ~ sd ~ cp ~ sd ~` |
| Coil (>50%) | Broken syncopation | `~ bd [~ sd] hh ~ [cp ~] bd` |

### 4. Contact Accent Positions

Residues that are close in 3D space (<8Å) but far apart in sequence (>20 residues). These represent the protein's tertiary fold — disulfide bridges, hydrophobic core contacts, domain interfaces. Mapped to a sparse accent layer (`cp`, `rim`, `oh`) unique to each fold topology.

### 5. Swing

Backbone angle variance (CA-CA-CA angles), normalized 0–1:
- **0.0** = regular helix (uniform angles) → straight timing
- **1.0** = highly variable loops → rhythmic swing/shuffle

---

## Genre System

### Available Genres (10)

| Genre | BPM | Kick Style | Snare/Clap | Hats | Character |
|-------|-----|-----------|------------|------|-----------|
| Liquid | 172 | Euclidean (sparser) | Structure-derived motif | `hh*N` B-factor contour | Soulful, flowing |
| Jump Up | 174 | Euclidean (structure) | Structure-derived motif | `hh*N` | High-energy, wobble |
| Neurofunk | 174 | Euclidean (structure) | Structure-derived motif | `hh*N` | Dark, techy (S→rim, P→fx) |
| Dancefloor | 174 | Euclidean (structure) | Structure-derived motif | `hh*N` | Anthemic, mainstream |
| Jungle | 168 | Euclidean (denser, +1 beat) | Structure-derived motif | `hh*N` | Breakbeat-heavy |
| Techstep | 174 | Euclidean (structure) | Structure-derived motif | `hh*N` | Dark, stripped-back (S→rim) |
| Darkstep | 174 | Euclidean (structure) | Structure-derived motif | `hh*N` | Aggressive, distorted (S→fx) |
| Halftime | 85 | Euclidean (spacious, 16-grid) | `~ ~ sd ~` (snare on 3) | `hh*N/2` (sparse) | Deep, half-speed |
| Breakcore | 180 | Euclidean (hyper-dense, +2, 12-grid) | Chopped double-pattern | `[hh oh]*N` (alternating) | Chaotic, extreme |
| Trance | 138 | Four-on-the-floor `bd(4,4)` | `~ cp ~ cp` (offbeat clap) | `[~ oh]*N` (offbeat open) | Euphoric, driving |

### Genre-Aware Element Mapping

Each genre adjusts the element-to-sound mapping:

| Genre | S (Sulfur) | P (Phosphorus) | N (Nitrogen) | O (Oxygen) |
|-------|-----------|----------------|-------------|-----------|
| Default/Liquid/JumpUp/Dancefloor/Jungle | cp | rim/perc | sd | hh |
| Neurofunk | rim (metallic) | fx (industrial) | sd | hh |
| Techstep | rim (stripped) | rim | sd | hh |
| Darkstep | fx (distorted) | rim | sd | hh |
| Trance | hh (texture) | rim | cp (clap) | oh (open hat) |
| Breakcore | cp | fx (chaotic) | sd | hh |

### Auto-Inference from Protein Structure

When no genre is explicitly selected, ProDnB infers one from the protein's 3D structure. Every protein maps to a genre:

| Priority | Structural Signal | Detection | Genre | Musical Logic |
|----------|-------------------|-----------|-------|---------------|
| 1 | **High disorder** (B-factor var >60 + coil >50%) | Backbone flexibility + no regular structure | **Breakcore** | Chaotic structure → chaotic chopped breaks |
| 2 | **Disulfide-dense** (≥3 S-S bonds <3Å) | CYS SG atom proximity | **Techstep** | Cross-linked rigidity → dark, metallic rolling |
| 3 | **Beta-sheet rich** (>40% sheet) | CA geometry | **Neurofunk** | Angular pleated folds → techy, industrial |
| 4 | **Flexible sheets** (>25% sheet + B-factor var >40) | Sheet + high flexibility | **Darkstep** | Aggressive flex → distorted, dark atmosphere |
| 5 | **Alpha-helix dominant** (>50% helix) | CA geometry | **Liquid** | Smooth coiled ribbons → soulful, flowing |
| 6 | **Small & compact** (≤100 residues, Rg <15Å) | Size + compactness | **Halftime** | Minimal fold → deep, spacious half-speed |
| 7 | **Multi-chain** (≥4 chains) | Chain count | **Jungle** | Complex assembly → polyrhythmic breaks |
| 8 | **High charged ratio** (>30% ARG/LYS/ASP/GLU) | Residue composition | **JumpUp** | Electrostatic energy → high-energy wobble |
| 9 | **High aromatic ratio** (>15% PHE/TYR/TRP/HIS) | Residue composition | **Trance** | Pi-stacking rings → euphoric, driving |
| 10 | **Large protein** (>500 residues) | Residue count | **Dancefloor** | Big anthemic structure → big anthemic beat |
| 11 | **Fallback** | Everything else | **Dancefloor** | Universal safe default |

Rules are evaluated in priority order — a disulfide-rich beta-sheet protein gets Techstep, not Neurofunk.

---

## Structural Feature → Musical Parameter Mapping

### B-Factor Variance → Euclidean Rhythm

| B-Factor Variance | Euclidean (beats, segments) | Effect |
|-------------------|----------------------------|--------|
| > 50 | (5, 8) | Dense kick pattern |
| > 20 | (4, 8) | Medium density |
| ≤ 20 | (3, 8) | Sparse pattern |

Motif composition further adjusts the grid:
- Helix-heavy → -1 beat (regular)
- Coil-heavy → +1 beat, 12-segment (polyrhythmic)
- Sheet-heavy → 16-segment (sparse)

### Occupancy → Gain

1. Computes average occupancy across all non-H atoms
2. Maps to gain: `base_gain = (0.5 + avg_occupancy * 0.5).clamp(0.3, 1.0)`
3. Applies per-layer multipliers: kick 0.95×, snare 0.9×, hats 0.6×, perc 0.5×

### Chain Length → Hi-Hat Density

- `hat_mult = (max_chain_length / 4).clamp(1, 8)`
- Pattern: `hh*N` (e.g. `hh*8` = 8 hi-hats per cycle = 16th notes at 4/4)

### Distance-Modulated Rhythm Seed

Instead of uniform sampling, the rhythm seed follows backbone geometry:
- **Tight helix regions** → subdivided hits `[bd sd]`
- **Normal spacing** → single hits
- **Extended loops** → rests and sparse placement

Each step's gain follows the B-factor contour, creating a velocity profile unique to each protein.

---

## Visual Feedback

Every layer gets inline Strudel visual feedback derived from the protein's structural data:

### Color Palette Selection

| Protein Character | Palette | Colors |
|-------------------|---------|--------|
| High swing (>0.5, disordered) | Vivid | magenta, #FF006E, #8338EC, hotpink |
| Many chains (>4) | Warm | #FF6B35, #FF9F1C, coral, orange |
| Large atom count (>3000) | Cool | cyan, #00B4D8, teal, #48CAE4 |
| Otherwise | Seed-selected | Deterministic from structure |

### Per-Layer Visual Type

| Layer | Visual Options |
|-------|---------------|
| Kick | `_punchcard` / `_spiral` |
| Snare (motif) | `_pianoroll` / `_punchcard` / `_spiral` |
| Hi-hats | `_spiral` (with dual colors for B-factor contour) |
| Perc (fingerprint) | `_punchcard` / `_pianoroll` / `_spiral` |
| Contacts (fold) | `_spiral` / `_punchcard` |
| Melodic | `_pianoroll` (with labels) / `_spiral` |

Visual selection is deterministic — same protein always produces the same visual identity.

---

## Output: Strudel Primitives JSON

The mapping produces a `MappedOutput`:

```json
{
  "tempo": 174,
  "swing": 0.42,
  "genre": "neurofunk",
  "primitives": [
    {
      "type": "euclidean",
      "sound": "bd",
      "beats": 4,
      "segments": 8,
      "gain": 0.9,
      "layer": "kick"
    },
    {
      "type": "drum",
      "pattern": "bd ~ sd ~ cp ~ sd ~",
      "gain": 0.85,
      "layer": "snare"
    },
    {
      "type": "drum",
      "pattern": "hh*6",
      "gain": 0.6,
      "layer": "hats",
      "gain_pattern": "0.48 0.52 0.36 0.48 0.44 0.40"
    },
    {
      "type": "drum",
      "pattern": "[bd sd] hh ~ perc bd [~ cp] hh ...",
      "gain": 0.5,
      "layer": "perc",
      "gain_pattern": "0.65 0.42 0.78 ..."
    },
    {
      "type": "drum",
      "pattern": "~ ~ cp ~ rim ~ ~ cp ~ rim ~ ~ ~ ~ cp ~",
      "gain": 0.7,
      "layer": "contacts"
    }
  ],
  "rhythm_seed": "bd [bd sd] hh ~ perc bd ...",
  "chain_lengths": [120, 85, 42],
  "element_counts": {"C": 450, "N": 120, "O": 180, "S": 8, "P": 2}
}
```

---

## Strudel.cc Syntax Reference

ProDnB outputs valid Strudel mini-notation:

| Syntax | Example | Meaning |
|--------|---------|---------|
| `s("pattern")` | `s("bd sd hh")` | Sound pattern |
| `~` | `sd ~ ~ sd` | Rest (silence) |
| `*N` | `hh*8` | N events per cycle |
| `[]` | `[bd sd]` | Subdivide time |
| `(beats,segments)` | `bd(5,8)` | Euclidean rhythm |
| `gain(N)` | `.gain(0.8)` | Volume 0–1 |
| `gain("pattern")` | `.gain("0.8 0.5 0.9")` | Per-step volume |
| `stack(...)` | `stack(s("bd"), s("hh*8"))` | Layer patterns |
| `setcps(N)` | `setcps(0.725)` | Tempo (174 BPM ≈ 0.725) |
| `.color("...")` | `.color("cyan magenta")` | Visual feedback color |
| `._punchcard()` | `._punchcard({ fillActive: 1 })` | Inline piano roll |
| `._spiral()` | `._spiral({ steady: 0.96 })` | Circular visualizer |
| `._pianoroll()` | `._pianoroll({ labels: 1 })` | Note visualizer |

**Drum samples**: `bd`, `sd`, `hh`, `cp`, `rim`, `oh` (open hat), `perc`, `fx`

---

## LLM Arrangement (Stage 2)

The LLM receives the `ProteinFramework` (primitives + structural features + fingerprint summary) and a genre-aware system prompt:

**Framework includes:**
- Deterministic primitives from Stage 1
- Element counts, chain lengths, rhythm seed
- Structural features: radius of gyration, contact density, B-factor variance
- Composition ratios: hydrophobic, charged, aromatic
- Fingerprint summary: helix/sheet/coil fractions, swing, fold contact count
- Genre, key, octave, melodic flag

**Requirements:**
- Use appropriate tempo for genre (80–185 BPM)
- Structure: Intro → Buildup → Drop → Breakdown → Drop → Outro
- Include visual feedback on every layer (`.color()` + inline visual)
- Representation key: comment block mapping each layer to protein biology

---

## Web UI Features

### Step-by-Step Workflow

1. **Step 1: Upload PDB** — Drag & drop (.pdb, .ent, .cif), max 10MB
2. **Step 2: Protein Function** — Click "Find the function, find the beat" to search via SERPAPI. Select one as your song-generating instruction.
3. **Continue the journey** — Generates an orchestration instruction (anthropomorphizing the protein, poetic interpretation, musical metaphors, technical guidance). Editable before Generate.
4. **Step 3: Genre & Tonal Options** — Subgenre (10 options), BPM (80–185), key, octave, melodic toggle. Auto-populated from AI when you select a function. If left on "Default", genre is auto-inferred from the protein's 3D structure.
5. **Step 4: Generate** — Strudel code via LLM with visual feedback.
6. **Step 5: Copy to Strudel** — Copy code to [strudel.cc](https://strudel.cc)

### Other Features

- **Map**: Automatic deterministic mapping on upload (with genre inference)
- **Generate (Stream)**: SSE streaming from Compound API
- **Piano Roll**: Visual grid of primitives (layers × 16 steps)
- **Assemble**: Sliders update gain; re-assemble without LLM
- **Visual feedback**: Every layer includes `.color()` and inline visuals (`_punchcard`, `_pianoroll`, `_spiral`)
- **Playback fixes**: Generated code is post-processed to inject `register('acidenv', ...)`, ensure a final `stack()`, and use `triangle` for melodic synths
- **Representation key**: Comment block mapping each layer to the protein's biological function

---

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/upload` | POST | Upload PDB file (multipart) |
| `/api/protein-function` | POST | Fetch protein function via SERPAPI |
| `/api/infer-beat-design` | POST | Infer genre/BPM/key/melodic from function text |
| `/api/generate-orchestration-instruction` | POST | Generate orchestration instruction |
| `/api/map` | POST | Map PDB → primitives JSON (with genre inference) |
| `/api/assemble` | POST | Assemble Strudel from primitives + sliders |
| `/api/generate` | POST | Generate Strudel via LLM (non-streaming) |
| `/api/generate/stream` | POST | Generate Strudel via LLM (SSE streaming) |

---

## Setup

1. Set `GROQ_API_KEY` in `.env`
2. Set `SERP_API_Key` in `.env` (for protein function lookup)
3. Build: `cargo build --release --package prodnb-web`
4. Run: `cargo run --release --package prodnb-web`
5. Open `http://127.0.0.1:8080`

---

## Project Structure

```
ProDnB/
├── prodnb-core/          # PDB parsing, mapping, framework
│   ├── src/
│   │   ├── protein.rs    # PDB parser (ATOM/HETATM/HEADER/TITLE)
│   │   ├── features.rs   # Structural features + fingerprint extraction
│   │   ├── strudel.rs    # PDB → Strudel mapping + genre inference + visuals
│   │   ├── genre.rs      # 10 genre definitions + default BPM
│   │   ├── framework.rs  # LLM framework (primitives + features + fingerprint)
│   │   ├── composition.rs # Arrangement sections
│   │   └── rng.rs        # Deterministic RNG
├── prodnb-web/           # Web server
│   ├── strudel_knowledge.md  # Local knowledge base for LLM
│   ├── beat_templates.md     # Beat templates for blending
│   ├── src/handlers.rs   # API + system prompt + genre inference
│   ├── templates/        # HTML (index.html)
│   └── static/           # CSS, JS, favicon
├── prodnb-midi/          # MIDI export
├── prodnb-audio/         # Audio synthesis
├── prodnb-cli/           # CLI + TUI
└── README.md
```

## Playback Troubleshooting

If pasted code doesn't play in Strudel.cc:

1. **`.acidenv is not a function`** — Strudel has no built-in acidenv. ProDnB auto-injects `register('acidenv', ...)` when the LLM uses `.acidenv()` on bass.
2. **Nothing plays** — In Strudel JS mode, only the last expression plays. Ensure output ends with a single `stack(...)` of all layers.
3. **Melodic layers (n()) silent** — ProDnB uses `triangle` (most reliable). If silent, click the Strudel play area first (browser autoplay).

---

## License

MIT License
