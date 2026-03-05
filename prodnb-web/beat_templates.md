# Beat Templates — Inspiration for Blending

Use these proven Strudel patterns as inspiration. Blend and mix elements to create complete works. Adapt to the framework primitives and genre. Convert $: to stack() where needed for JS mode.

---

## Template: Deep Liquid (125 BPM, E minor)
setcps(125/4)
// Stereo bass, sub, pads, polyrhythmic top, kick, hats, percussion
$: n("<<[0 ~ 7 ~] [0 5 ~ 5] [0 ~ 3 7] [0 7 5 3]> ...>>").scale("e:minor").trans(-24)
  .s("<triangle sawtooth triangle z_sawtooth>".slow(4))
  .lpf("<400 600 350 500>".slow(4)).resonance("<8 5 10 6>".slow(4))
  .gain(0.85).distort(0.3).pan(0.1)
$: n("<<0 0 0 0> <0 ~ 0 ~>>").scale("e:minor").trans(-36).s("sine").gain(0.6).lpf(150)
$: n("<[0 3 7] [0 4 7] [-1 2 5]>").scale("e:minor").s("gm_pad_2_warm").lpf("<1200 1500 1000>".slow(8)).gain(0.3).room(0.7)
$: s("<bd:0!4 [bd:0 ~ bd:0 bd:0]>").gain(0.7).lpf(100)
$: s("<<hh:8*4 hh:8*8 hh:8*6> <hh:8*8 hh:8*4 hh:8*12>>").gain("<<0.25 0.3 0.35> <0.3 0.25 0.35>>")

---

## Template: Flamenco / World (120 BPM, A minor)
setcps(120/4)
// Guitar, fiddle, bass, flamenco percussion
$: n("<<[0 3 7 10] [7 10 14 17] [5 8 12 15]>>").scale("a:minor").s("gm_acoustic_guitar_nylon kalimba").lpf(2500).gain(0.6)
$: n("<<[0 ~ ~ ~] [7 ~ ~ ~] [5 ~ ~ ~]>>").scale("a:minor").trans(-24).s("triangle").lpf(400).gain(0.7)
$: s("<bd:2!4 bd:2!4 [bd:2 bd:2 ~ bd:2]>").gain(0.5)
$: s("~ cp:3 ~ <cp:3 [cp:3 cp:3]>").gain(0.4)
$: s("shaker_small*8").gain("<0.3 0.4 0.5 0.4>")

---

## Template: Acid / 303 (136 BPM, G minor)
setcps(136/4)
register('acidenv', (x, pat) => pat.lpf(100).lpenv(x * 9).lps(.2).lpd(.12))
$: n("<0 4 0 9 7>*16").scale("g:minor").trans(-12).octave(3).s("sawtooth").acidenv(slider(0.655))
$: n("<0>*16").scale("g:minor").trans(-24).octave(3).s("supersaw").acidenv(slider(0.632))
$: s("top:1/2").fit().octave(5)
$: s("bd!4").duck("3:4:5:6").duckdepth(.8).duckattack(.16)

---

## Template: Walking Bass / 303 (140 BPM, G minor)
setcps(140/4)
$: s("<bd:2!4 bd:2!4 [bd:2 bd:2 ~ bd:2 bd:2] [bd:2*2 ~ bd:2 bd:2]>").gain(.8)
$: s("~ cp:2 ~ <cp:2 [cp:2 cp:2]>").gain(0.3)
$: s("hh:3*8").gain("<<0.3 0.6 0.4 0.7> <0.3 0.6 0.3 0.6>>")
$: n("<[0 [0 3] [5 7] [3 5]] [[-2 0] [3 5] [7 10] [5 3]]>").add(-14).scale("g:minor")
  .s("<gm_viola gm_cello gm_contrabass>".slow(4))
  .lpf("<1500 2500 1000 1800>".slow(4)).resonance("<1 10 1 5>".slow(4)).gain(0.4)

---

## Template: Drop / Tension (140 BPM)
// Kick drops out then slams back; bass goes silent then BOWWWW
$: s("<bd:2!4!6 [~ ~ ~ [bd:2 bd:2*4]]>")
$: n("<[0 [0 3] [5 7] [3 5]]!6 ~ [0!16]>").add(-21).scale("g:minor")
  .s("<gm_contrabass!6 ~ supersquare>".slow(8))
  .lpf(sine.range(30, 5000).fast(4))
  .resonance("<1!6 ~ 35>".slow(8)).gain("<0.4!6 ~ 1.0>".slow(8))
$: s("<~!6 wind ~>").gain("<~!6 [0 0.1 0.3 0.6] ~>".slow(8))
$: s("hh:3*8").gain(0.6)
$: s("~ cp:2 ~ cp:2").gain(0.4)

---

## Template: Acid Filter Sweep Drop
$: n("<[0 [0 3] [5 7] [3 5]]!6 ~ [[0!8]*2]>").add(-14).scale("g:minor")
  .s("<gm_viola!6 ~ sawtooth>".slow(8))
  .lpf("<1500!6 ~ [100 4000]>".slow(8))
  .resonance("<1!6 ~ 25>".slow(8)).gain("<0.4!6 ~ 0.7>".slow(8))
$: s("<[hh:3*8]!6 [hh:3*16] [hh:3*8]>").gain("<0.6!6 0.8 0.7>")
$: s("<[~ cp:2 ~ cp:2]!6 ~ [cp:2!4]>").gain("<0.4!6 ~ 0.8>")

---

## Blending Instructions
- Pick elements from multiple templates: e.g. kick from one, bass from another, pads from a third
- Match tempo (setcps) to framework BPM: setcps(bpm/60/4) for 4-beat cycle
- Adapt scales to framework key (e.g. "e:minor" -> framework key)
- Use .gain(slider(X, 0, 1)) for intensity control on each layer
- Combine protein-derived primitives with template-inspired patterns for unique results

## CRITICAL: JS Output Structure
In Strudel JS mode, only the last evaluated expression plays. Build layers as const, then ONE stack:
```
const drums = stack(s("bd(5,8)").gain(...), s("sd ~ ~ sd").gain(...), s("hh*8").gain(...))
const bass = n("<0 4 0 9 7>*16").scale("C:minor").octave(3).s("sawtooth").acidenv(slider(...))
const pad = n("0 2 4 6").scale("C:minor").octave(2).s("sawtooth").lpf(800).gain(...)
const lead = n("<0 2 4 7>!8").scale("C:minor").octave(4).s("sawtooth").gain(...)
stack(drums, bass, pad, lead)   // ← THIS single output is what plays
```
NEVER output multiple separate stack() calls — only the last would play.
