/**
 * Greedy decoding can loop on a hesitation ("euh, euh, euh…" seventy times in
 * one phrase): a word repeated more than four times in a row is kept once.
 * The CLI stops the same loops while decoding (crates/cli/src/asr.rs).
 */
export function collapseLoops(text: string): string {
  return text
    .replace(/(\b[\p{L}\p{N}'’-]+[,.;]?\s+)(?:\1){4,}/giu, "$1")
    .replace(/\s{2,}/g, " ")
    .trim();
}
