//! The motion findings (`specs/CAPTURE.md` §Audits, Motion), read from a
//! film's frames: flicker, spinner pace and width jitter, frames a link
//! cannot carry in time, and what still moves under reduced motion — and
//! the heatmap and spinners a film reports.

use serde_json::json;

use super::{Film, FilmRequest, Heat, Spinner};
use crate::audit::{Check, Finding, Severity};
use crate::capture::ColorValue;

/// Every frame's cells, row by row.
type Frames = [Vec<Vec<(String, ColorValue, ColorValue)>>];

/// A spinner's comfortable pace: a glyph shown between these, in ms.
const SPINNER_MS: (u64, u64) = (50, 500);

/// The cells that changed, as runs of a row with one change count, most
/// first; at most 64.
pub(super) fn heatmap(frames: &Frames) -> (Vec<Heat>, usize) {
    let Some(first) = frames.first() else {
        return (Vec::new(), 0);
    };
    let mut out: Vec<Heat> = Vec::new();
    for (y, row) in first.iter().enumerate() {
        let mut run: Option<Heat> = None;
        for x in 0..row.len() {
            let n = frames
                .windows(2)
                .filter(|w| {
                    w[0].get(y).and_then(|r| r.get(x)) != w[1].get(y).and_then(|r| r.get(x))
                })
                .count() as u32;
            match run.as_mut() {
                Some(h) if n > 0 && h.changes == n && usize::from(h.rect[0] + h.rect[2]) == x => {
                    h.rect[2] += 1
                }
                _ => {
                    if let Some(h) = run.take() {
                        out.push(h);
                    }
                    if n > 0 {
                        run = Some(Heat {
                            rect: [x as u16, y as u16, 1, 1],
                            changes: n,
                        });
                    }
                }
            }
        }
        if let Some(h) = run {
            out.push(h);
        }
    }
    out.sort_by(|a, b| {
        b.changes
            .cmp(&a.changes)
            .then((a.rect[1], a.rect[0]).cmp(&(b.rect[1], b.rect[0])))
    });
    let runs = out.len();
    out.truncate(64);
    (out, runs)
}

/// Cells cycling through three or more glyphs with a period that repeats
/// over the film (at least twice).
pub(super) fn spinners(frames: &Frames, step: u64) -> Vec<Spinner> {
    let Some(first) = frames.first() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (y, row) in first.iter().enumerate() {
        for x in 0..row.len() {
            let seq: Vec<&str> = frames
                .iter()
                .filter_map(|f| f.get(y).and_then(|r| r.get(x)).map(|c| c.0.as_str()))
                .collect();
            let mut distinct: Vec<&str> = seq.clone();
            distinct.sort_unstable();
            distinct.dedup();
            if distinct.len() < 3 {
                continue;
            }
            let Some(p) =
                (1..=seq.len() / 2).find(|&p| (p..seq.len()).all(|i| seq[i] == seq[i - p]))
            else {
                continue;
            };
            // One cycle's glyphs, each once, from where the glyph changes:
            // a film that starts part-way through a glyph would otherwise
            // see its first glyph at both ends of the cycle.
            let cycle = &seq[..p];
            let start = (0..p)
                .find(|&k| cycle[k] != cycle[(k + p - 1) % p])
                .unwrap_or(0);
            let mut glyphs: Vec<String> = Vec::new();
            for k in 0..p {
                let g = cycle[(start + k) % p];
                if glyphs.last().is_none_or(|l| l != g) {
                    glyphs.push(g.to_string());
                }
            }
            let period_ms = p as u64 * step;
            out.push(Spinner {
                x: x as u16,
                y: y as u16,
                frame_ms: period_ms / glyphs.len().max(1) as u64,
                period_ms,
                glyphs,
            });
        }
    }
    out
}

fn finding(
    check: Check,
    severity: Severity,
    message: String,
    cells: Vec<[u16; 4]>,
    inputs: serde_json::Value,
    threshold: serde_json::Value,
) -> Finding {
    Finding {
        check,
        severity,
        message,
        nodes: Vec::new(),
        cells,
        inputs,
        threshold,
    }
}

/// The film's motion findings.
pub(super) fn findings(film: &Film, request: &FilmRequest) -> Vec<Finding> {
    let frames: Vec<_> = film.grids.iter().map(super::cells_of).collect();
    let mut out = Vec::new();
    // Flicker: a cell that changes and is back at the next frame.
    let mut flicker: Vec<(u16, u16)> = Vec::new();
    if let Some(first) = frames.first() {
        for (y, row) in first.iter().enumerate() {
            for x in 0..row.len() {
                let at = |k: usize| frames[k].get(y).and_then(|r| r.get(x));
                if (1..frames.len().saturating_sub(1))
                    .any(|k| at(k - 1) == at(k + 1) && at(k) != at(k - 1))
                {
                    flicker.push((x as u16, y as u16));
                }
            }
        }
    }
    let spinning: Vec<(u16, u16)> = film.spinners.iter().map(|s| (s.x, s.y)).collect();
    flicker.retain(|c| !spinning.contains(c));
    if !flicker.is_empty() {
        out.push(finding(
            Check::Flicker,
            Severity::Warning,
            format!(
                "{} cells change and change back within one frame",
                flicker.len()
            ),
            crate::audit::rects(&flicker),
            json!({"step_ms": film.step_ms, "cells": flicker.len()}),
            json!({"frames": 1}),
        ));
    }
    for s in &film.spinners {
        let widths: Vec<usize> = s
            .glyphs
            .iter()
            .map(|g| unicode_width::UnicodeWidthStr::width(g.as_str()))
            .collect();
        if widths.iter().any(|w| *w != widths[0]) {
            out.push(finding(
                Check::Spinner,
                Severity::Warning,
                format!(
                    "a spinner's glyphs differ in width ({:?}): what follows it jitters",
                    s.glyphs
                ),
                vec![[s.x, s.y, 1, 1]],
                json!({"glyphs": s.glyphs, "widths": widths}),
                json!({"widths": "equal"}),
            ));
        }
        if s.frame_ms < SPINNER_MS.0 || s.frame_ms > SPINNER_MS.1 {
            out.push(finding(
                Check::Spinner,
                Severity::Info,
                format!(
                    "a spinner shows each glyph {} ms, outside a comfortable {}–{} ms",
                    s.frame_ms, SPINNER_MS.0, SPINNER_MS.1
                ),
                vec![[s.x, s.y, 1, 1]],
                json!({"frame_ms": s.frame_ms, "period_ms": s.period_ms}),
                json!({"frame_ms": [SPINNER_MS.0, SPINNER_MS.1]}),
            ));
        }
    }
    // Cost: a frame whose bytes take longer to send than the step.
    if let Some(bps) = request.link.bps() {
        let ms = |bytes: usize| bytes as u64 * 8 * 1000 / bps;
        // Exact: the bits × 1000 against the step's, no rounding down.
        let late_bytes = |bytes: usize| bytes as u64 * 8 * 1000 > film.step_ms * bps;
        if late_bytes(film.input_bytes) {
            out.push(finding(
                Check::Cost,
                Severity::Warning,
                format!(
                    "the inputs' frame takes {} ms to send on {:?}, longer than the {} ms step",
                    ms(film.input_bytes),
                    request.link,
                    film.step_ms
                ),
                Vec::new(),
                json!({"bytes": film.input_bytes, "bps": bps}),
                json!({"ms": film.step_ms}),
            ));
        }
        let late: Vec<&super::FilmFrame> = film
            .frames
            .iter()
            .skip(1)
            .filter(|f| late_bytes(f.bytes))
            .collect();
        if !late.is_empty() {
            let worst = late.iter().map(|f| f.bytes).max().unwrap_or(0);
            out.push(finding(
                Check::Cost,
                Severity::Warning,
                format!(
                    "{} of {} frames take longer to send than the {} ms step on {:?} (the largest: {worst} bytes, {} ms)",
                    late.len(),
                    film.frames.len() - 1,
                    film.step_ms,
                    request.link,
                    ms(worst)
                ),
                Vec::new(),
                json!({"late": late.iter().map(|f| f.t_ms).collect::<Vec<_>>(), "bps": bps}),
                json!({"ms": film.step_ms}),
            ));
        }
    }
    out
}

/// What still moves in a film taken under `prefers-reduced-motion:
/// reduce`: every cell that changes, not the heatmap's cut.
pub(super) fn reduced_motion(still: &Film) -> Vec<Finding> {
    let frames: Vec<_> = still.grids.iter().map(super::cells_of).collect();
    let mut moving: Vec<(u16, u16)> = Vec::new();
    if let Some(first) = frames.first() {
        for (y, row) in first.iter().enumerate() {
            for x in 0..row.len() {
                let at = |f: &Vec<Vec<(String, ColorValue, ColorValue)>>| {
                    f.get(y).and_then(|r| r.get(x)).cloned()
                };
                if frames.windows(2).any(|w| at(&w[0]) != at(&w[1])) {
                    moving.push((x as u16, y as u16));
                }
            }
        }
    }
    if moving.is_empty() {
        return Vec::new();
    }
    vec![finding(
        Check::ReducedMotion,
        Severity::Warning,
        format!(
            "{} cells still change under prefers-reduced-motion: reduce",
            moving.len()
        ),
        crate::audit::rects(&moving),
        json!({"cells": moving.len()}),
        json!({"cells": 0}),
    )]
}
