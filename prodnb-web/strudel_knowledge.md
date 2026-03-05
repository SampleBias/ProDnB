# Strudel.cc Knowledge Base for ProDnB

Reference for generating clean, executable Strudel code. Use this when arranging DnB patterns.

## Critical Syntax Rules

### Output Format (REQUIRED) — Strudel JS mode
- Strudel default REPL is JavaScript. Tidal syntax (`d1 $`, `$`) is INVALID and causes parser errors.
- Use: `setcps(0.725)` then `stack(p1, p2, p3)` — variadic args, NO array brackets, NO `d1 $`
- NEVER use: `d1 $ stack([...])` — that's Tidal (Haskell) syntax
- NEVER use variable assignments (bd = s(...)) - causes "bd is not defined"
- All patterns inline inside s("...")

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

### Full DnB Stack (Template) — JS mode
Use slider(value, 0, 1) for intensity. stack(p1, p2, ...) variadic — no array, no d1 $:
```
setcps(0.725)

stack(
  s("bd(5,8)").gain(slider(0.9, 0, 1)),
  s("sd ~ ~ sd").gain(slider(0.85, 0, 1)),
  s("hh*8").gain(slider(0.6, 0, 1))
)
```

### With Percussion Layer
```
setcps(0.725)

stack(
  s("bd(5,8)").gain(slider(0.95, 0, 1)),
  s("sd ~ ~ sd").gain(slider(0.9, 0, 1)),
  s("hh*8").gain(slider(0.54, 0, 1)),
  s("sd bd bd bd bd hh bd bd bd hh sd hh hh bd hh hh bd bd bd bd sd bd sd bd").gain(slider(0.5, 0, 1))
)
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
- scale("C:minor") or scale("Am") - set key
- n("0 2 4 6").scale("C:minor").s("sawtooth") - melodic pattern
- transpose(N) - semitones
- Octave in scale: "C4:minor" defaults root to octave 4

## Piano Roll
- Piano roll is a Strudel.cc feature - patterns are the piano roll
- Output full patterns in stack(p1, p2, ...) - kick, snare, hats, perc layers
- No separate piano roll UI in ProDnB

## Common Mistakes to Avoid
1. (5,8)bd -> use bd(5,8)
2. bd = s(...) -> inline in stack
3. sound "bd" -> use s("bd")
4. d1 $ or stack([...]) -> use stack(p1, p2, ...) for JS mode

## Mode Guard (for LLM/linting)
- If output contains `d1` or `$` → convert to JS or reject with: "Tidal syntax detected; use stack(p1, p2, ...) for Strudel default REPL."
- ProDnB post-processes LLM output to fix Tidal→JS automatically.
