---
name: verbatim
description: Transcribe and edit video or audio on the user's machine with the `verbatim` CLI — speech to text with NVIDIA Parakeet (25 languages, timed to the word; TXT, SRT, VTT, Markdown, JSON, CSV), and cuts by time ("supprime de 15:39 à 15:43 et les 5:42 dernières minutes"), by words said, hesitations (euh, um) or long pauses, written to a new file with ffmpeg. Use when the user wants a transcript or subtitles of a recording, or wants to cut, trim or clean up a video or audio file, including from a plain-language instruction or in a script.
---

# verbatim

`verbatim` runs on the user's machine: the media never leaves it. Only an instruction that needs a model
(not plain times) sends the instruction and the transcript's text to the AI provider configured in the
environment. `verbatim --help` and `verbatim <command> --help` are the reference for flags.

Install: `brew install maxgfr/tap/verbatim` (pulls ffmpeg). Check the setup with `verbatim doctor`.
The first transcription downloads its model once: Parakeet (~670 MB) by default, Canary (~213 MB) for
`--lang en|fr|de|es` (`verbatim model download [--engine canary]` to do it ahead).

## Transcribe

```bash
verbatim transcribe talk.mp4                         # plain text on stdout
verbatim transcribe talk.mp4 -o talk.srt             # format from the extension
verbatim transcribe talk.mp4 -f srt,vtt,md,json -o out/
verbatim transcribe talk.mp4 --lang fr -o talk.srt   # French: told to the model, never guessed
verbatim export talk.json -f vtt -o talk.vtt         # a transcript JSON (CLI's or the site's) to another format
```

Transcripts are cached per file and language: later `plan`/`cut` runs on the same file reuse them.

**Name the spoken language when you know it** (`--lang fr`, also on `plan` and `cut`). Without it,
Parakeet guesses among 25 European languages, and it can take accented or mixed speech for English: a
French talk full of English terms came out as broken English. With `--lang en|fr|de|es`, Canary
transcribes instead, told the language (213 MB, downloaded once; `verbatim model download --engine
canary` to fetch it ahead). Other languages stay with Parakeet. If a transcript comes out in the wrong
language, run it again with `--lang` and `--no-cache`. Both models keep hesitations, which is what lets
`--fillers` remove them; Canary times phrases, so word times inside a phrase are estimated.

## Edit: plan, check, apply

Always show the plan before writing a video. The original is never modified; the result goes to `-o`.

1. **Plan** (no media written):
   ```bash
   verbatim cut talk.mp4 "supprime de 15:39 à 15:43 et les 5:42 dernières minutes" -o out.mp4 --dry-run
   ```
   It prints every cut and the new duration. Read it back to the user when the request was in words.
2. **Apply** once the plan is right: the same command without `--dry-run`. A plan that came from a model
   asks for confirmation; pass `--yes` only after the user has seen it.

Ways to say what goes:

| Want | Flags |
| --- | --- |
| A range | `--delete 15:39-15:43` (repeatable), `--delete "last 5:42"`, `--delete "first 30"`, `--delete 12:00-end`, `--delete start-0:30` |
| Keep only some parts | `--keep 1:00-2:30` (everything else goes) |
| Hesitations | `--fillers` |
| Long pauses | `--silences 1.5` (`--pad 0.15` kept on each side) |
| Where words are said | `--phrase "off the record"` |
| Plain language | the instruction as an argument, French or English |
| A saved plan | `--cuts cuts.json` (from `verbatim plan … -o cuts.json` or the site's export) |

Times like `15:39` are minutes:seconds; `1:02:03` is hours:minutes:seconds; bare numbers are seconds.
Flags and an instruction combine: `verbatim cut talk.mp4 "les 30 premières secondes" --fillers --lang fr -o out.mp4`.

Instructions made only of times are read without a model. An instruction about content ("cut the part
about pricing", "remove the intro", "garde seulement la démo") needs someone to read the transcript, and
**you are that reader**: do not ask for an API key and do not fall back to asking for times.

1. Transcribe to JSON (cached, so free the second time): `verbatim transcribe talk.mp4 --lang fr -f json -o talk.json`.
2. Read `transcript.segments` (`text`, `start`, `end` in seconds) and find the passages the user means.
   Cut whole segments, from the first one's `start` to the last one's `end`; for a word or a phrase said
   several times, prefer `--phrase "…"`.
3. Show the plan with `--dry-run` and the passages' text, then apply:
   `verbatim cut talk.mp4 --delete 2:02-2:31 --delete 30:05.5-30:12 --lang fr -o short.mp4`.
   Mix with times and flags freely (`--fillers`, `--silences 1.5`, `"les 5:42 dernières minutes"`).

The CLI can also ask a model itself (the instruction as an argument, with `ANTHROPIC_API_KEY`, or
`OPENAI_API_KEY` and `--ai-model`, or `--ai ollama --ai-model <model>`): useful in scripts, where no agent
reads the transcript. `--no-ai` makes it refuse instead.

Output: `.mp4`/`.mov`/`.mkv` re-encode exactly at the cut (H.264/AAC); `--fast` copies streams and snaps
each cut to the previous keyframe (seconds off, much faster). `.m4a`/`.mp3`/`.wav` write audio only.
`--subtitles` also writes the result's transcript as `.srt` next to it.

## Scripts

```bash
verbatim plan talk.mp4 --fillers --silences 2 -o cuts.json   # cut list JSON, nothing rendered
verbatim cut talk.mp4 --cuts cuts.json -o clean.mp4 --json   # machine-readable result
```

Every command exits non-zero with a one-line `verbatim: …` message on failure; `--quiet` silences progress.
A cut list is `{"version":1,"duration":…,"delete":[[start,end],…]}` in seconds, the same format the site
(maxgfr.github.io/verbatim) imports and exports.
