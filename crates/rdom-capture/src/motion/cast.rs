//! A film as an asciicast v2 recording (`specs/CAPTURE.md` §Views,
//! `cast`): a header line, then one `[seconds, "o", text]` event per
//! frame — the bytes rdom sent the terminal. The first is the whole first
//! frame, repainted on a cleared terminal (`App::repaint`), so a player
//! starting from nothing shows what the app showed; every later one is
//! what rdom sent next, against the same terminal state. No timestamp in
//! the header: the same film is the same file.

use super::Film;

/// `film` as an asciicast v2 file, at the film's terminal size.
pub fn asciicast(film: &Film) -> String {
    let [width, height] = film.size;
    let mut out = serde_json::json!({"version": 2, "width": width, "height": height}).to_string();
    out.push('\n');
    for (frame, bytes) in film.frames.iter().zip(&film.sent) {
        if bytes.is_empty() {
            continue;
        }
        let event = serde_json::json!([
            frame.t_ms as f64 / 1000.0,
            "o",
            String::from_utf8_lossy(bytes)
        ]);
        out.push_str(&event.to_string());
        out.push('\n');
    }
    out
}
