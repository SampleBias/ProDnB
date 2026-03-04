//! Minimal Strudel→MIDI: parses s("bd sd hh"), stack(...), setcps() and produces drum MIDI.
//!
//! Supports: setcps(x), s("...") / sound("..."), stack(...), mini-notation (~ rest, *N repeat).

use anyhow::Result;
use crate::events::{EventKind, MidiEvent, NoteEvent};

const TICKS_PER_BEAT: u32 = 480;
const TICKS_PER_16TH: u32 = TICKS_PER_BEAT / 4;
const DEFAULT_BPM: u16 = 174;

/// GM drum map (channel 9)
fn drum_name_to_midi(name: &str) -> Option<u8> {
    let n = name.to_lowercase();
    Some(match n.as_str() {
        "bd" | "kick" => 36,
        "sd" | "snare" => 38,
        "hh" | "hat" | "ch" => 42,
        "oh" | "openhat" => 46,
        "cp" | "clap" => 39,
        "rim" => 37,
        "perc" => 45,
        _ => return None,
    })
}

/// Parse Strudel code and return (bpm, MIDI events as flat list).
pub fn strudel_to_midi(code: &str) -> Result<(u16, Vec<MidiEvent>)> {
    let code = strip_comments_and_whitespace(code);
    let mut bpm = DEFAULT_BPM;
    let mut patterns: Vec<Vec<StrudelHit>> = Vec::new();

    // Extract setcps(x)
    if let Some(cps) = extract_setcps(&code) {
        bpm = (cps * 4.0 * 60.0).round() as u16;
        bpm = bpm.clamp(60, 240);
    }

    // Extract s("...") or sound("...") patterns
    for cap in extract_s_patterns(&code) {
        if let Some(hits) = parse_mini_notation(&cap) {
            patterns.push(hits);
        }
    }

    // Extract stack(...) - inner s() calls already captured above; if no stack, use first pattern
    let use_patterns = if patterns.is_empty() {
        // Fallback: try to find any s("...") we might have missed
        vec![]
    } else {
        patterns
    };

    if use_patterns.is_empty() {
        anyhow::bail!("No valid s(\"...\") patterns found in Strudel code");
    }

    let ticks_per_cycle = 4 * TICKS_PER_BEAT; // 1 bar
    let mut all_events: Vec<MidiEvent> = Vec::new();

    for pattern in &use_patterns {
        let events = pattern_to_midi(pattern, ticks_per_cycle);
        all_events.extend(events);
    }

    all_events.sort_by_key(|e| e.start_ticks());
    Ok((bpm, all_events))
}

#[derive(Debug, Clone)]
struct StrudelHit {
    sound: String,
    step: usize,
    total_steps: usize,
}

fn parse_mini_notation(s: &str) -> Option<Vec<StrudelHit>> {
    let tokens: Vec<&str> = s.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }

    let mut hits = Vec::new();
    let mut expanded: Vec<String> = Vec::new();

    for t in tokens {
        let t = t.trim();
        if t == "~" || t.is_empty() {
            expanded.push(String::new()); // rest
        } else if let Some((name, count)) = parse_repeat(t) {
            for _ in 0..count {
                expanded.push(name.to_string());
            }
        } else {
            expanded.push(t.to_string());
        }
    }

    let n = expanded.len();
    for (i, name) in expanded.into_iter().enumerate() {
        if !name.is_empty() && drum_name_to_midi(&name).is_some() {
            hits.push(StrudelHit {
                sound: name,
                step: i,
                total_steps: n,
            });
        }
    }

    if hits.is_empty() {
        return None;
    }
    Some(hits)
}

fn parse_repeat(s: &str) -> Option<(&str, usize)> {
    let s = s.trim();
    if let Some(idx) = s.find('*') {
        let (name, num) = s.split_at(idx);
        let num = num.trim_start_matches('*').parse::<usize>().ok()?;
        if num > 0 && num <= 64 {
            return Some((name, num));
        }
    }
    None
}

fn pattern_to_midi(pattern: &[StrudelHit], ticks_per_cycle: u32) -> Vec<MidiEvent> {
    let mut events = Vec::new();
    let steps = pattern.iter().map(|h| h.total_steps).max().unwrap_or(16);
    let step_ticks = ticks_per_cycle / steps as u32;
    let duration = (step_ticks * 3 / 4).max(1); // 75% of step
    let velocity = 100u8;

    for hit in pattern {
        let note = match drum_name_to_midi(&hit.sound) {
            Some(n) => n,
            None => continue,
        };
        let start_ticks = (hit.step as u32 * ticks_per_cycle) / hit.total_steps as u32;

        events.push(MidiEvent::new(
            start_ticks,
            EventKind::NoteOn(NoteEvent::new(note, velocity)),
        ));
        events.push(MidiEvent::new(
            start_ticks + duration,
            EventKind::NoteOff(NoteEvent::new(note, velocity)),
        ));
    }

    events
}

fn strip_comments_and_whitespace(s: &str) -> String {
    s.lines()
        .filter_map(|l| {
            let l = l.trim();
            if l.starts_with("//") {
                None
            } else {
                Some(l)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_setcps(code: &str) -> Option<f64> {
    let code = code.replace(' ', "");
    let start = code.find("setcps(")?;
    let rest = &code[start + 7..];
    let end = rest.find(')')?;
    rest[..end].parse::<f64>().ok()
}

fn extract_s_patterns(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = code.chars().collect();

    while i < chars.len() {
        // Match s(" or sound("
        let rest: String = chars[i..].iter().collect();
        let (prefix, skip) = if rest.starts_with("s(\"") {
            ("s(\"", 3)
        } else if rest.starts_with("sound(\"") {
            ("sound(\"", 7)
        } else {
            i += 1;
            continue;
        };

        i += skip;
        let mut content = String::new();
        let mut escaped = false;

        while i < chars.len() {
            let c = chars[i];
            if escaped {
                content.push(c);
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                i += 1;
                break;
            } else {
                content.push(c);
            }
            i += 1;
        }

        if !content.is_empty() {
            out.push(content);
        }
    }

    out
}

/// Build MIDI events for playback. Repeats pattern for ~32 bars.
pub fn strudel_to_playback_events(code: &str) -> Result<(u16, Vec<MidiEvent>)> {
    let (bpm, mut events) = strudel_to_midi(code)?;

    // Find cycle length (max tick in first cycle)
    let cycle_ticks = events
        .iter()
        .map(|e| e.start_ticks)
        .max()
        .unwrap_or(4 * TICKS_PER_BEAT)
        + TICKS_PER_BEAT;

    // Repeat for 32 bars
    const BARS: u32 = 32;
    let total_cycle_ticks = 4 * TICKS_PER_BEAT * BARS;
    let mut all = Vec::new();
    let mut offset = 0u32;

    while offset < total_cycle_ticks {
        for ev in &events {
            let new_ticks = ev.start_ticks + offset;
            let kind = match &ev.kind {
                EventKind::NoteOn(n) => EventKind::NoteOn(n.clone()),
                EventKind::NoteOff(n) => EventKind::NoteOff(n.clone()),
                _ => continue,
            };
            all.push(MidiEvent::new(new_ticks, kind));
        }
        offset += cycle_ticks;
    }

    all.sort_by_key(|e| e.start_ticks);
    Ok((bpm, all))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let (bpm, evs) = strudel_to_midi(r#"s("bd sd hh")"#).unwrap();
        assert!(bpm >= 60 && bpm <= 240);
        assert!(!evs.is_empty());
    }

    #[test]
    fn test_setcps() {
        let (bpm, _) = strudel_to_midi("setcps(0.7) s(\"bd sd\")").unwrap();
        assert!((168..=175).contains(&bpm));
    }

    #[test]
    fn test_stack() {
        let (_, evs) = strudel_to_midi(r#"stack(s("bd"), s("hh*8"))"#).unwrap();
        assert!(!evs.is_empty());
    }
}
