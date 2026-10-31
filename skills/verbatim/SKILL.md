---
name: verbatim
description: Transcribe and edit video or audio on the user's machine with the `verbatim` CLI — speech to text with NVIDIA Parakeet or Canary (TXT, SRT, VTT, Markdown, JSON, CSV) and cuts by time, by transcript segment, by words said, hesitations or long pauses, rendered with ffmpeg. Use when the user wants a transcript or subtitles of a recording, or wants to cut, trim, shorten or clean up a video or audio file, whatever language they ask in.
---

# verbatim

`verbatim` runs on the user's machine; the media never leaves it. **The CLI reads no sentences**: you
understand what the user asks, in whatever language, and turn it into the options below. Never pass the
user's words to the CLI as an instruction. `verbatim --help` and `verbatim <command> --help` are the
reference.

Install: `brew install maxgfr/tap/verbatim` (pulls ffmpeg). Check with `verbatim doctor`.

## Workflow

1. **Look**: `verbatim info talk.mp4` (duration, streams, JSON).
2. **Transcribe** when the request is about content, words, hesitations or pauses (cached per file and
   language, so later calls are free):
   `verbatim transcribe talk.mp4 --lang fr -o talk.srt`, or `-f txt,srt,vtt,md,json,csv -o out/`.
3. **Find** what the user means: `verbatim segments talk.mp4 --lang fr` lists numbered, timed segments
   (`--grep word`, `--from 10:00 --to 12:00`, `--words` for word times, `--json` for JSON lines). Read
   them yourself: speech recognition misspells names ("l'Itos" for Lighthouse), so match by meaning.
4. **Plan**: run the `cut` command with `--dry-run` and show the user the cuts, with the text of the
   segments they remove when the request was about content. Fix the plan if they object.
5. **Apply**: the same command without `--dry-run`. The source is never modified; the result goes to `-o`.

## Name the spoken language

Pass `--lang` whenever you know it (`transcribe`, `segments`, `plan`, `cut`). Without it, Parakeet guesses
among 25 European languages and can take accented or mixed speech for English (a French talk full of
English terms came out as broken English). With `--lang fr|de|es`, Canary transcribes, told the
language (213 MB, downloaded once; `verbatim model download --engine canary` ahead). Other languages
(and English) stay with Parakeet, the stronger model for them. If a transcript is in the wrong language, run it again with `--lang` and `--no-cache`.

## Cut options

All repeatable or combinable; the cuts add up. Times: `15:39`, `1:02:03.5`, `939` (seconds), `90s`,
`5min`, `1h02`. A range is `START-END`; `start` and `end` stand for the media's ends.

| The user means | Options |
| --- | --- |
| A range | `--delete 15:39-15:43`, `--delete start-0:30`, `--delete 12:00-end` |
| The first / last part | `--delete-first 30`, `--delete-last 5:42` |
| Keep only part of it | `--keep 1:00-2:30` (repeatable), `--keep-first 10min`, `--keep-last 2min` |
| Passages by content | read `segments`, then `--segments 12,15-18` (numbers as listed) or `--delete` their times |
| Keep only some passages | `--keep-segments 3-7,20` |
| Every place a word or phrase is said | `--phrase "off the record"` |
| Hesitations | `--fillers` (built in: euh, um, uh, äh, ehm…); in another language pass its own: `--filler-words "eto,ano"` |
| Long pauses | `--silences 1.5` (`--pad 0.15` kept on each side) |
| A saved plan | `--cuts cuts.json` (from `verbatim plan … -o cuts.json` or the site) |
| Cuts a bit wider / tighter | `--expand 0.2` / `--expand=-0.1` (seconds on each side) |
| Fewer, cleaner cuts | `--merge-gap 1` (join cuts closer than 1 s), `--min-cut 0.3` (drop tiny ones) |

Examples of requests and the options they become:

- "supprime de 15:39 à 15:43 et les 5:42 dernières minutes" → `--delete 15:39-15:43 --delete-last 5:42`
- "elimina i primi 30 secondi e le pause lunghe" → `--delete-first 30 --silences 1.5 --lang it`
- "schneide alles nach Minute 40 weg" → `--delete 40:00-end --lang de`
- "quita las muletillas" → `--fillers --filler-words "eh,este,o sea,pues" --lang es`
- "remove the part where they talk about pricing" → read `segments`, then `--segments 41-47`

## Output options (`cut`)

`.mp4`/`.mov`/`.mkv` re-encode exactly at the cut: `--codec h264|h265`, `--crf 18` (lower is better and
bigger), `--preset medium` (`ultrafast` … `veryslow`), `--audio-bitrate 192k`, `--no-audio`,
`--no-video`. `--fast` copies the streams instead (much faster, each cut snaps to the previous keyframe).
`.m4a`/`.mp3`/`.wav` write audio only. `--subtitles` also writes the result's transcript as `.srt`.
`--json` prints `{cuts, keeps, outputDuration, output, command}`.

## Scripts

```bash
verbatim plan talk.mp4 --fillers --silences 2 --lang fr -o cuts.json   # cut list, nothing rendered
verbatim cut talk.mp4 --cuts cuts.json -o clean.mp4 --json            # machine-readable result
```

Every command exits non-zero with a one-line `verbatim: …` message on failure (a bad range, an unknown
segment, a cut that would leave nothing, an output that would overwrite the source); `--quiet` silences
progress. A cut list is `{"version":1,"duration":…,"delete":[[start,end],…]}` in seconds, the same format
the site (maxgfr.github.io/verbatim) imports and exports.
