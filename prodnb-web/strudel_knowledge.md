# Strudel.cc Knowledge Base for ProDnB

Reference for generating clean, executable Strudel code. Use this when arranging DnB patterns.

## Critical Syntax Rules

### Output Format (REQUIRED) — Strudel JS mode
- Strudel default REPL is JavaScript. **Only the last evaluated expression plays** — multiple separate `stack()` calls will NOT all play; each replaces the previous.
- **CRITICAL**: Build each layer as a `const`, then output ONE combined `stack()` at the end. This is the ONLY way all layers play together.
- Tidal syntax (`d1 $`, `$`) is INVALID. Use `const name = ...` for layers, then `stack(drums, bass, pad, lead)`.
- NEVER output multiple separate `stack(...)` calls — only the last would play.

### Euclidean Rhythm
- Format: **sound(beats,segments)** - sound FIRST, then parentheses
- CORRECT: s("bd(5,8)") s("hh(3,8)")
- WRONG: s("(5,8)bd") - invalid, causes parse errors

### Mini-Notation
| Symbol | Meaning | Example |
|--------|---------|---------|
| ~ | Rest/silence | s("bd ~ ~ sd") |
| *N | N events per cycle | s("hh*8") = 8 hi-hats |
| [] | Subdivide time | s("[bd sd] hh") |
| , | Parallel (chord) | s("bd,hh") |
| (n,m) | Euclidean: n beats in m segments | s("bd(5,8)") |
| !N | Repeat without speeding | s("bd!4") |
| ? | 50% chance | s("cp?*8") |

### Drum Samples
bd, sd, hh, oh, cp, rim, perc, misc, fx

### Synth vs Sample (melodic layers)
- **Samples** (s("bd"), s("hh")): Loaded from sample banks. Always work.
- **Synths** (n("0 2 4").s("sine")): WebAudio oscillators. Use **triangle** as default — Strudel's native default, most reliable. If sine/sawtooth are silent, switch to triangle.
- For n() melodic patterns: `.s("triangle")` is the safest. Then `.s("sawtooth")`, `.s("sine")`. Always set `.gain(slider(0.5, 0, 1))` so layers are audible.
- If melodic layers are silent: (1) Click the Strudel play area first (browser autoplay), (2) Try .s("triangle") instead of sine/sawtooth.

### Effects
- .gain(0.8) - volume 0-1
- .bank("RolandTR808") - drum machine
- .speed(1) - playback speed
- .fast(2) - pattern speed

## DnB Pattern Library

### Kick Patterns
```
s("bd*4")
s("bd(5,8)")
s("bd(3,4)")
s("bd ~ bd ~")
```

### Snare Patterns (2 and 4)
```
s("sd ~ ~ sd")
s("~ sd ~ sd")
s("sd*2")
s("[sd ~] [~ sd]")
```

### Hi-Hat Patterns
```
s("hh*8")
s("hh*16")
s("hh(5,8)")
s("[hh hh] [hh ~]")
```

### acidenv (REQUIRED when using .acidenv())
Strudel does NOT have `.acidenv` built-in. You MUST register it before any bass/lead uses it. Paste right after setcps():
```
setcps(168/60/4);
register('acidenv', (x, pat) => pat
  .lpf(100)
  .lpenv(x * 9)
  .lps(0.2)
  .lpd(0.12)
);
```
If your code uses `.acidenv(slider(...))` on bass or lead, you MUST include this register() block first.

### Full DnB Stack (Template) — JS mode
**Build layers as const, then ONE stack.** Only the last evaluated expression plays, so you MUST combine all layers into a single stack():
```
setcps(0.725);
register('acidenv', (x, pat) => pat.lpf(100).lpenv(x * 9).lps(0.2).lpd(0.12));

const drums = stack(
  s("bd(5,8)").gain(slider(0.9, 0, 1)),
  s("sd ~ ~ sd").gain(slider(0.85, 0, 1)),
  s("hh*8").gain(slider(0.6, 0, 1))
)

const bass = n("<0 4 0 9 7>*16").scale("C:minor").octave(3).s("sawtooth")
  .acidenv(slider(0.68, 0, 1)).gain(slider(0.78, 0, 1))

const pad = n("0 2 4 6").scale("C:minor").octave(2).s("sawtooth").lpf(800).gain(slider(0.32, 0, 1))

// ✅ single output — THIS is what plays (REQUIRED)
stack(drums, bass, pad)
```

## Intensity Controls (Strudel.cc slider)
- Use slider(value, 0, 1) for gain - Strudel.cc adds interactive sliders in the REPL
- Example: s("bd(5,8)").gain(slider(0.95, 0, 1))
- Kick 95% -> .gain(slider(0.95, 0, 1))
- Snare 90% -> .gain(slider(0.9, 0, 1))
- Hi-Hats 60% -> .gain(slider(0.6, 0, 1))
- ALWAYS use slider(gain, 0, 1) on each layer - no ProDnB UI sliders

## DnB Subgenre Guidelines
When framework includes `genre`, match the style:
- **liquid**: Soulful, melodic. Pads, soft hats, melodic bass. Use .scale(), .note() for pads. n("0 2 4 6").scale("C:minor").s("sawtooth")
- **jump_up**: High-energy, wobble. Aggressive basses, punchy kicks. Dense patterns.
- **neurofunk**: Dark, techy. Industrial, metallic (rim, fx), reese bass. Sparse, syncopated.
- **dancefloor**: Anthemic, mainstream. Big kicks, catchy. Major keys, uplifting.
- **jungle**: Breakbeat-heavy. Amen-style breaks, ragga. Dense percussion, cp/rim.

## Tonal Syntax (for melodic layers)
- scale("Root:minor") — Strudel requires colon format. Use "G:minor", "A:minor", "C:minor" — NOT "Gm" or "Am".
- n("0 2 4 6").scale("C:minor").s("triangle") - melodic pattern
- transpose(N) - semitones
- Octave in scale: "C4:minor" defaults root to octave 4

## Piano Roll
- Piano roll is a Strudel.cc feature - patterns are the piano roll
- Output full patterns in stack(p1, p2, ...) - kick, snare, hats, perc layers
- No separate piano roll UI in ProDnB

## Representation Key (REQUIRED for ProDnB)
Every generated code MUST start with a comment block that maps each layer to the protein's biological function. This lets the DJ know what each slider controls.

Format:
```
// === PROTEIN: [PDB_ID] — [Title/name]
// Function: [one-line biological role, e.g. "Oxygen transport in blood"]
// REPRESENTATION KEY (adjust sliders to shape the biology):
//   DRUMS: [e.g. Carbon backbone rhythm from C atoms]
//   BASS:  [e.g. Oxygen binding pulse — delivery intensity]
//   PAD:   [e.g. Heme pocket dynamics]
//   LEAD:  [e.g. Subunit interface melody]
```
Include inline comments on each const: `// slider = [biological meaning]`

## Common Mistakes to Avoid
1. (5,8)bd -> use bd(5,8)
2. Multiple separate stack() calls -> only the last plays! Use const for layers, then ONE stack(drums, bass, pad, lead)
3. Using .acidenv() without register('acidenv', ...) -> Strudel will error ".acidenv is not a function". Always add the register block right after setcps when using acid bass.
4. Defining drums, bass, pad, lead but not outputting stack() -> nothing plays! The final line MUST be stack(drums, bass, pad, lead).
5. n() melodic layers silent -> use .s("triangle") instead of sine/sawtooth; ensure .gain() is set; user may need to click play area first.
6. sound "bd" -> use s("bd")
7. d1 $ or stack([...]) -> use const + stack(p1, p2, ...) for JS mode

## Mode Guard (for LLM/linting)
- If output contains `d1` or `$` → convert to JS or reject with: "Tidal syntax detected; use stack(p1, p2, ...) for Strudel default REPL."
- ProDnB post-processes LLM output to fix Tidal→JS automatically.
