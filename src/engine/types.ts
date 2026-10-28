import type { Transcript } from "llm-connect";

export type { Transcript, TranscriptPiece } from "llm-connect";

export interface Chapter {
  start: number;
  title: string;
}

/** One transcribed file, as kept in the history. The media itself is never stored. */
export interface Doc {
  id: string;
  name: string;
  mime: string;
  size: number;
  createdAt: number;
  updatedAt: number;
  /** The engine that produced it, as shown to the user ("Parakeet", "Whisper (fast)", "OpenAI"…). */
  engine: string;
  transcript: Transcript;
  summary?: string;
  chapters?: Chapter[];
  /** Ranges to cut, in seconds, sorted and merged (the montage). */
  cuts?: [number, number][];
  /** Segment texts in another language, keyed by language code, aligned with transcript.segments. */
  translations?: Record<string, string[]>;
}
