//! verbatim-edl for the site. Values cross as JSON strings, the same shapes
//! the CLI reads and writes, so the page and `verbatim` cannot drift apart.

use verbatim_edl::{CutList, Edit, ExportDoc, Format, Piece, Range, Transcript};
use wasm_bindgen::prelude::*;

fn from<T: serde::de::DeserializeOwned>(json: &str) -> Result<T, JsError> {
    serde_json::from_str(json).map_err(|e| JsError::new(&e.to_string()))
}

fn to<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("plain data serializes")
}

/// An instruction in plain French or English → `{ edits, unparsed }`.
#[wasm_bindgen(js_name = parseInstruction)]
pub fn parse_instruction(text: &str) -> String {
    to(&verbatim_edl::parse_instruction(text))
}

/// Edits (`[{ op, span, … }]`) → the cut list for this duration.
#[wasm_bindgen(js_name = resolve)]
pub fn resolve(edits: &str, duration: f64) -> Result<String, JsError> {
    let edits: Vec<Edit> = from(edits)?;
    Ok(to(&CutList::new(
        duration,
        verbatim_edl::resolve(&edits, duration),
    )))
}

/// Any ranges (`[[start, end], …]`) → a normalized cut list.
#[wasm_bindgen(js_name = cutList)]
pub fn cut_list(delete: &str, duration: f64) -> Result<String, JsError> {
    Ok(to(&CutList::new(duration, from::<Vec<Range>>(delete)?)))
}

#[wasm_bindgen(js_name = fillers)]
pub fn fillers(words: &str) -> Result<String, JsError> {
    Ok(to(&verbatim_edl::fillers(&from::<Vec<Piece>>(words)?)))
}

#[wasm_bindgen(js_name = silences)]
pub fn silences(words: &str, duration: f64, min_gap: f64, pad: f64) -> Result<String, JsError> {
    Ok(to(&verbatim_edl::silences(
        &from::<Vec<Piece>>(words)?,
        duration,
        min_gap,
        pad,
    )))
}

#[wasm_bindgen(js_name = findText)]
pub fn find_text(words: &str, query: &str) -> Result<String, JsError> {
    Ok(to(&verbatim_edl::find_text(
        &from::<Vec<Piece>>(words)?,
        query,
    )))
}

/// The transcript on the edited timeline.
#[wasm_bindgen(js_name = remap)]
pub fn remap(transcript: &str, cut_list: &str) -> Result<String, JsError> {
    let t: Transcript = from(transcript)?;
    let c: CutList = from(cut_list)?;
    Ok(to(&verbatim_edl::remap(&t, &c.delete)))
}

/// The `verbatim cut` arguments for a cut list: `--delete 15:39-15:43 …`.
#[wasm_bindgen(js_name = cliArgs)]
pub fn cli_args(cut_list: &str) -> Result<String, JsError> {
    Ok(from::<CutList>(cut_list)?.cli_args().join(" "))
}

/// One export (`txt`, `txt-timed`, `srt`, `vtt`, `md`, `json`, `csv`) of `{ name, engine, transcript, summary?, chapters? }`.
#[wasm_bindgen(js_name = render)]
pub fn render(doc: &str, format: &str) -> Result<String, JsError> {
    let format =
        Format::parse(format).ok_or_else(|| JsError::new(&format!("unknown format {format}")))?;
    Ok(verbatim_edl::render(&from::<ExportDoc>(doc)?, format))
}

/// The system prompt for a model asked to plan an edit.
#[wasm_bindgen(js_name = planSystem)]
pub fn plan_system() -> String {
    verbatim_edl::PLAN_SYSTEM.to_string()
}

/// The user message for that model; `transcript` may be empty.
#[wasm_bindgen(js_name = planPrompt)]
pub fn plan_prompt(instruction: &str, duration: f64, transcript: &str) -> Result<String, JsError> {
    let t: Option<Transcript> = if transcript.is_empty() {
        None
    } else {
        Some(from(transcript)?)
    };
    Ok(verbatim_edl::plan_prompt(instruction, duration, t.as_ref()))
}

/// The plan in a model's answer → `{ edits, fillers, silences, phrases, note }`.
#[wasm_bindgen(js_name = parsePlan)]
pub fn parse_plan(text: &str) -> Result<String, JsError> {
    verbatim_edl::parse_plan(text)
        .map(|p| to(&p))
        .map_err(|e| JsError::new(&e))
}
