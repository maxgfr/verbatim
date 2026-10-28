# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

Bun build (no bundler framework), React 19, TypeScript, plain CSS. Static site on GitHub Pages at `maxgfr.github.io/verbatim/`. Chosen in the approved plan, following the sibling app pixvault.

## Users

Anyone who has a video or audio file and wants its words as text: someone with a lecture, a meeting recording, an interview, a voice memo, a clip to subtitle. General purpose, no account, no prior setup.

## Product Purpose

Drop a video or audio file, get an accurate, timed transcript produced on your own device, read it alongside the media, fix it, and take it away as plain text, subtitles (SRT, VTT) or structured data (JSON, Markdown, CSV). Success: the first transcript arrives without the user having to understand models, keys or formats.

## Positioning

The file never leaves the device: transcription runs in the page (Whisper, NVIDIA Parakeet or Canary through llm-connect, on WebGPU or WebAssembly). No upload, no account, no server. Cloud transcription and AI features exist only as an explicit opt-in with the user's own key.

## Operating Context

- One file at a time is the common case; several files queue.
- The first run downloads a speech model (about 80 MB to 670 MB) once; later runs work offline from the browser cache.
- Transcription takes time (roughly real time down to a few times faster); progress and cancel must be visible.
- Review happens with the media playing: the current word follows playback, clicking a word seeks.

## Capabilities and Constraints

- Engines: Whisper (fast / balanced / accurate), Parakeet (25 European languages, language detected, exact word timings), Canary (EN/FR/DE/ES, smaller). Whisper does not detect the language: a language must be chosen or the browser's is used.
- Exports: TXT (with or without timestamps), SRT, VTT, JSON, Markdown, CSV, and all of them as a zip.
- History of transcripts in IndexedDB (text and metadata, never the media).
- Optional, with the user's own key via llm-connect (or Ollama / LM Studio): summary, timestamped chapters, translation of segments, cleanup; and cloud transcription (OpenAI, Groq, Mistral, Gemini), which sends the file away and must say so.
- Strict CSP; the only external origins are Hugging Face (model download) and the AI providers the user configures.
- Interface in English and French; light and dark.

## Brand Commitments

Name: verbatim. Part of maxgfr's family of local-first browser tools (pixvault, pdfvault, nook…): privacy is stated plainly and proven, not marketed.

## Evidence on Hand

No testimonials, users, benchmarks or press. Do not fabricate any. Speed and size figures come only from llm-connect's documentation.

## Product Principles

1. Nothing leaves the device unless the user explicitly sends it, and the UI says so at that moment.
2. The first transcript needs zero decisions: sensible automatic engine and language, choices available but not required.
3. The transcript is the work surface: reading, correcting and exporting happen in one place, next to the media.
4. Honest about cost: model size, download and time are shown before the user commits.

## Accessibility & Inclusion

WCAG 2.2 AA. Full keyboard use (including the player and word seek), progress announced to screen readers, reduced motion respected. Assumed: no product-specific requirement beyond that was established.
