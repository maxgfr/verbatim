---
name: verbatim
description: A transcript set as the tracklist of a field-recording LP sleeve, on device.
colors:
  accent: "oklch(46% 0.2 266)"
  accent-ink: "oklch(98% 0.01 266)"
  accent-wash: "oklch(93% 0.035 266)"
  cut: "oklch(55% 0.17 28)"
  cut-wash: "oklch(93% 0.03 28)"
  warn: "oklch(52% 0.13 70)"
  paper: "oklch(97.4% 0.004 255)"
  paper-2: "oklch(94.6% 0.006 255)"
  ink: "oklch(23% 0.018 265)"
  ink-2: "oklch(44% 0.016 265)"
  ink-3: "oklch(58% 0.012 265)"
  rule: "oklch(86% 0.008 260)"
  rule-strong: "oklch(70% 0.012 262)"
  accent-dark: "oklch(74% 0.13 266)"
  accent-ink-dark: "oklch(18% 0.03 266)"
  accent-wash-dark: "oklch(30% 0.06 266)"
  cut-dark: "oklch(72% 0.13 28)"
  cut-wash-dark: "oklch(30% 0.05 28)"
  warn-dark: "oklch(80% 0.12 80)"
  paper-dark: "oklch(19% 0.012 265)"
  paper-2-dark: "oklch(23% 0.014 265)"
  ink-dark: "oklch(93% 0.006 260)"
  ink-2-dark: "oklch(76% 0.01 260)"
  ink-3-dark: "oklch(64% 0.012 262)"
  rule-dark: "oklch(32% 0.014 265)"
  rule-strong-dark: "oklch(48% 0.016 265)"
typography:
  display:
    fontFamily: "Hanken Grotesk Variable, Hanken Grotesk, system-ui, sans-serif"
    fontSize: "clamp(1.75rem, 3.4vw, 2.75rem)"
    fontWeight: 600
    lineHeight: 1.15
    letterSpacing: "-0.02em"
  headline:
    fontFamily: "Hanken Grotesk Variable, Hanken Grotesk, system-ui, sans-serif"
    fontSize: "1.75rem"
    fontWeight: 600
    lineHeight: 1.15
    letterSpacing: "-0.02em"
  title:
    fontFamily: "Hanken Grotesk Variable, Hanken Grotesk, system-ui, sans-serif"
    fontSize: "1rem"
    fontWeight: 650
    lineHeight: 1.55
  body:
    fontFamily: "Hanken Grotesk Variable, Hanken Grotesk, system-ui, sans-serif"
    fontSize: "1rem"
    fontWeight: 400
    lineHeight: 1.55
    fontFeature: "ss01"
  label:
    fontFamily: "Hanken Grotesk Variable, Hanken Grotesk, system-ui, sans-serif"
    fontSize: "0.8125rem"
    fontWeight: 700
    lineHeight: 1.55
    letterSpacing: "0.08em"
  small:
    fontFamily: "Hanken Grotesk Variable, Hanken Grotesk, system-ui, sans-serif"
    fontSize: "0.8125rem"
    fontWeight: 400
    lineHeight: 1.55
  control:
    fontFamily: "Hanken Grotesk Variable, Hanken Grotesk, system-ui, sans-serif"
    fontSize: "0.8125rem"
    fontWeight: 600
    letterSpacing: "0.01em"
rounded:
  none: "0px"
spacing:
  hair: "0.25rem"
  xs: "0.4rem"
  sm: "0.6rem"
  md: "1rem"
  lg: "1.5rem"
  xl: "2rem"
  xxl: "2.5rem"
  margin: "clamp(1rem, 4vw, 3.5rem)"
components:
  button:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    typography: "{typography.control}"
    rounded: "{rounded.none}"
    padding: "0.4rem 0.9rem"
    height: "2.25rem"
  button-hover:
    backgroundColor: "{colors.paper-2}"
    textColor: "{colors.ink}"
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.accent-ink}"
    typography: "{typography.control}"
    rounded: "{rounded.none}"
    padding: "0.4rem 0.9rem"
    height: "2.25rem"
  button-primary-large:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.accent-ink}"
    rounded: "{rounded.none}"
    padding: "0.6rem 1.4rem"
    height: "3rem"
  button-secondary:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    rounded: "{rounded.none}"
    padding: "0.4rem 0.9rem"
  button-small:
    padding: "0.25rem 0.7rem"
    height: "2rem"
  button-link:
    backgroundColor: "transparent"
    textColor: "{colors.ink-2}"
    padding: "0"
  button-icon:
    backgroundColor: "transparent"
    textColor: "{colors.ink-2}"
    rounded: "{rounded.none}"
    size: "2rem"
  segmented-option:
    backgroundColor: "transparent"
    textColor: "{colors.ink-2}"
    rounded: "{rounded.none}"
    padding: "0.25rem 0.6rem"
    height: "2rem"
  segmented-option-selected:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.paper}"
  input:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    typography: "{typography.body}"
    rounded: "{rounded.none}"
    padding: "0.45rem 0.65rem"
    height: "2.25rem"
  nav-link:
    textColor: "{colors.ink-2}"
    typography: "{typography.label}"
    padding: "0.25rem 0"
  nav-link-current:
    textColor: "{colors.ink}"
  tab:
    textColor: "{colors.ink-2}"
    typography: "{typography.control}"
    padding: "0.4rem 0"
    height: "2.25rem"
  tab-selected:
    textColor: "{colors.ink}"
  track-row:
    textColor: "{colors.ink}"
    typography: "{typography.body}"
    padding: "0.55rem 0"
  proposal:
    backgroundColor: "{colors.paper-2}"
    textColor: "{colors.ink}"
    padding: "1rem"
  sleeve:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    rounded: "{rounded.none}"
    padding: "clamp(1.5rem, 4vw, 3rem)"
  sleeve-over:
    backgroundColor: "{colors.accent-wash}"
  meter:
    backgroundColor: "{colors.rule}"
    height: "3px"
---

# Design System: verbatim

## Overview

**Creative North Star: "The Field-Recording Sleeve"**

verbatim is set like the sleeve and liner notes of a field-recording LP. A transcript is a tracklist: every segment is a numbered track with its timecode, and the segment playing is the track that plays. The page is cool paper with graphite ink, ruled in hairlines, cornered square, with wide margins that let the text breathe. Nothing floats, glows or is tinted for mood; depth is drawn with rules, never cast with light.

The system is quiet so that one colour can mean something. A single cold ultramarine marks exactly three things: what is playing now, where the keyboard focus is, and the primary action. A warm red marks what has been cut. Everything else is ink at three strengths on paper at two. Hierarchy comes from weight and capitals in one grotesk family, not from a ladder of sizes: the interface uses three sizes, plus one cover-scale exception for the drop field's title.

The world refuses the category default for transcription tools: the dark SaaS panel with a glowing waveform and chat bubbles. Its timeline is a graduated hairline ruler, not a waveform; its AI requests are plain fields and indented proposals, not bubbles. Dark mode is a graphite sleeve with the same structure, not a different product.

**Key Characteristics:**
- Cool paper and graphite ink, one ultramarine signal, one red for cuts.
- One grotesk (Hanken Grotesk), three sizes; rank by weight (400 to 800) and small capitals with tracking.
- Hairline rules (1px) carry all structure; a strong ink rule opens each major list.
- Square corners everywhere (0px), no shadows, no gradients, no glass.
- Tabular numerals for every number and time.
- Motion is short, eased and functional (120 to 300ms), and disappears under reduced motion.

## Colors

A near-monochrome cool-graphite palette with one cold blue signal and one warm red for removal; every neutral carries a faint blue-violet bias (hue 255 to 265).

### Primary
- **Sleeve Ultramarine** (accent): the only signal colour. The playing track's number and timecode, the underline under the word being spoken, the playhead, the progress fill, the focus outline, the current nav and tab underline, the primary button, the drop field while a file is held over it. In dark mode it lifts to a pale periwinkle (accent-dark) so it reads on graphite.
- **Ultramarine Ink** (accent-ink): text on an ultramarine fill (primary button, skip link).
- **Ultramarine Wash** (accent-wash): search hits, text selection, and the drop field's held-over state. A wash, never a fill for controls.

### Secondary
- **Splice Red** (cut): the strike-through on cut words and segments, the edges of a cut region on the timeline, the pressed state of a cut toggle, danger links on hover, and error text.
- **Splice Wash** (cut-wash): the body of a cut region on the timeline.

### Tertiary
- **Amber Caution** (warn): inline warnings in the liner column (for example an engine that needs the spoken language). Text only.

### Neutral
- **Cool Paper** (paper): page ground, input ground, the sticky search bar's ground.
- **Sleeve Grey** (paper-2): the one raised tone. Button hover, the player frame, AI proposals, command boxes.
- **Graphite Ink** (ink): body text, headings, default button borders, the strong rules (top bar, list openers, sleeve frame, timeline baseline), the selected segmented option's fill.
- **Pencil** (ink-2): secondary text, labels, notes, unselected nav and tabs, link buttons, major timeline ticks.
- **Faint Pencil** (ink-3): track numbers, timeline labels, placeholders, step numbers, format list, cut text.
- **Hairline** (rule): row dividers, inner sleeve frame, meter track.
- **Strong Hairline** (rule-strong): input borders, secondary button borders, segmented borders, minor ticks, scrollbar thumb.

One-off: the video letterbox is a near-black graphite (oklch(12% 0.01 265)) so footage never sits on paper.

### Named Rules
**The Now-Playing Rule.** Ultramarine means "now, here, or go": playing, focused, primary. If an element is none of those, it is not blue. One primary button per view.

**The Splice Rule.** Red means removed. It appears as a strike-through, a cut region's edge or an error, never as a fill on a control.

**The Two-Paper Rule.** There are only two grounds, paper and paper-2. Anything that needs to stand apart gets a rule, not a third tint.

## Typography

**Display Font:** Hanken Grotesk Variable (with Hanken Grotesk, system-ui, sans-serif), self-hosted, weights 100 to 900.
**Body Font:** the same family.
**Label/Mono Font:** the same family in tracked capitals; a system monospace (ui-monospace, SF Mono, Menlo) only inside command boxes.

**Character:** One sober grotesk, set like liner-note typesetting: a firm medium-weight title, plain reading text, and small tracked capitals for every label. Stylistic set 1 (ss01) is on globally.

### Hierarchy
- **Display** (600, clamp(1.75rem, 3.4vw, 2.75rem), 1.15, -0.02em): the sleeve's title only, the one exception to the three-size rule, at the scale of a record cover. Max 16ch, balanced wrap.
- **Headline** (600, 1.75rem, 1.15, -0.02em): page titles and the transcript's file name.
- **Title** (650, 1rem): subsection heads, file names in the queue and history, emphasised values in the cost line.
- **Body** (400, 1rem, 1.55): transcript text (max 72ch), prose (max 68ch), inputs.
- **Label** (700, 0.8125rem, 0.08em, uppercase): section headings, field labels, disclosure summaries, chapter heads (at 0.06em). Nav links use the same capitals at 600.
- **Small** (400, 0.8125rem): notes, metadata lines, track numbers, timeline labels, cost line.
- **Control** (600, 0.8125rem, 0.01em): button text; large buttons step up to 1rem.
- **Wordmark** (800, 1rem, -0.02em): the word "verbatim" in the top bar.

### Named Rules
**The Three-Size Rule.** The interface uses 0.8125rem, 1rem and 1.75rem. Rank anything new by weight and capitals first; reach for the sleeve's display size only for a cover-scale title.

**The Tabular Rule.** Every number, size, percentage and timecode is set in tabular numerals so columns of time align like a printed tracklist.

## Layout

Wide margins are a token: the page margin scales from 1rem to 3.5rem with the viewport (4vw), and both the top bar and every page use it. Content pages cap at 72rem (narrow pages 42rem, prose 44rem, page intros 60ch).

- **Home:** a two-column sleeve and liner. The square drop field (aspect 1:1, capped at the viewport height minus 10rem, at least 20rem tall) on the left; a liner column (16 to 22rem) on the right with engine, language and cost, and the file queue beneath so progress stays in the first viewport.
- **Editor:** a deck column (18 to 30rem) on the left, sticky and independently scrolling, holding the player, the hairline timeline, the meta line and the tabs; the tracklist sheet on the right with a sticky search and cut bar.
- **Track row:** a four-column grid: number (2.25rem, right-aligned), timecode (4.25rem), text, actions. Rows are separated by hairlines with 0.55rem vertical padding.
- **Rhythm:** small steps (0.25, 0.4, 0.6rem) inside controls and rows; 1rem between fields and panel blocks; 1.25 to 2.5rem between groups; 2rem above chapter heads.
- **At 960px and below:** single column. The sleeve loses its square aspect (min 22rem). The editor reorders to read along first: player and timeline, then the tracklist, then the tabs and tools.
- **At 640px and below:** the top bar wraps, nav moves to its own full-width row spread edge to edge; track columns tighten to 1.5rem and 3.1rem; track actions are always visible on touch.

## Elevation & Depth

Flat by construction. There are no shadows, gradients, blurs or glass anywhere in the system. Depth is drawn: a strong ink rule opens a region (top bar, sleeve, sheet head, queue, history, chapter), a hairline separates its rows, and a single raised tone (paper-2) marks a contained surface such as the player, a proposal or a command. Sticky elements (deck, search bar) sit on solid paper with a hairline under them, so they occlude without lifting.

### Named Rules
**The Drawn-Not-Cast Rule.** Separation is a 1px line or a change from paper to paper-2. If something seems to need a shadow, give it a rule.

**The Ink-Opens Rule.** A list or region starts with a 1px ink rule and its rows divide with 1px hairlines; strength of line, not size of gap, says where a section begins.

## Shapes

Square corners throughout (0px), including buttons, inputs, words highlighted in the transcript, and the drop field. The signature silhouette is the sleeve: a 1px ink frame with a second 1px hairline frame inset by 0.6rem, like the printed border on a record sleeve; both turn ultramarine while a file is held over it. Underlines are the other recurring shape: a 2px accent bar under the current nav link and the selected tab, a 2px accent underline (offset 0.28em) under the word being spoken, a 1px underline (offset 0.22em) on links. Icons are drawn for the app on a 24-unit grid with a 1.5 stroke, square caps and mitred joins; only the play triangle is filled.

## Components

### Buttons
Outlined, square and compact; they read like stamped labels rather than pills.
- **Shape:** square (0px), 1px border, minimum 2.25rem tall.
- **Default:** transparent with an ink border and ink text, control type (600, 0.8125rem).
- **Primary:** ultramarine fill and border, ultramarine-ink text. Hover mixes 18% ink into the accent for both fill and border.
- **Secondary:** the strong-hairline border instead of ink, for tools that should recede (the cut-selection button in the search bar).
- **Large / Small:** large is 3rem tall at body size with 0.6rem 1.4rem padding (the sleeve's file button); small is 2rem tall with 0.25rem 0.7rem.
- **Link:** no border or padding, pencil text with a 1px underline; hover turns ultramarine, or splice red when destructive.
- **Icon:** a 2rem square with a transparent border, pencil glyph; hover inks the glyph and lays paper-2 behind it; a pressed cut toggle turns red.
- **Hover / Focus:** background, colour and border transition over 160ms on the house ease. Focus is a 2px ultramarine outline offset 2px. Disabled drops to 45% opacity.

### Segmented control
Joined square options sharing 1px strong-hairline borders (overlapped by -1px). The selected option is a solid ink block with paper text. Used for language (EN / FR) and other two-to-few-way switches.

### Inputs / Fields
- **Style:** paper ground, 1px strong-hairline border, square, 0.45rem 0.65rem padding, 2.25rem tall, body type. Placeholders in faint pencil.
- **Label:** tracked small capitals (700, 0.8125rem, 0.08em) in pencil above the field, 0.3rem gap.
- **Focus:** the border turns ultramarine and a 2px ultramarine outline sits inset by 1px.
- **Error:** message text in splice red beside or under the field; warnings in amber.

### Navigation
A top bar ruled off by a 1px ink line: the wordmark (800) left, nav links in tracked capitals (600, 0.8125rem, 0.08em) in pencil, tools (segmented language, theme icon) right. Hover inks the link; the current page gets ink text and a 2px ultramarine underline. On phones the nav wraps to its own row, justified edge to edge, with tracking eased to 0.03em.

### Tabs
The deck's Edit / AI / Export tabs: borderless buttons in tracked capitals over a hairline, the selected tab inked with a 2px ultramarine bar that overlaps the hairline.

### Progress meter
A 3px hairline track with an ultramarine fill that widens over 300ms. When the ratio is unknown, a 30% segment sweeps across in 1.4s. Paired with a small-type status line in tabular numerals (percent, bytes, elapsed, time left).

### Tracklist (signature)
The transcript. Each segment is a row: faint track number (replaced by a filled play glyph while playing), a timecode button in tabular 600 pencil, the text in ink, and edit / cut icon actions that appear on hover or focus (always on touch). The playing track's number and time turn ultramarine; the spoken word is underlined 2px in ultramarine. Search hits get the ultramarine wash; filtered-out rows dim to 35%. Cut words and segments go faint pencil with a 1px splice-red strike-through. Chapter heads are tracked capitals over a 1px ink rule.

### Timeline (signature)
A graduated hairline ruler under the player, not a waveform: a 1.6rem track on a 1px ink baseline, minor ticks (0.35rem, strong hairline), major ticks (0.7rem, pencil), full-height ink ticks for chapters, cut regions as splice wash with red edges, and a 2px ultramarine playhead. Time labels in small faint pencil beneath.

### Sleeve (signature)
The home drop field: the double-framed square described in Shapes, holding the three numbered steps, the cover-scale title, the on-device statement, the large primary file button and the format list in tracked faint capitals. While a file is dragged over it, the ground turns ultramarine wash and both frames turn ultramarine (200ms).

### Proposal
An AI suggestion awaiting acceptance: a paper-2 block with 1rem padding and a 1px ultramarine rule on its leading edge, so it reads as an annotation in the margin rather than a chat bubble.

## Do's and Don'ts

### Do:
- **Do** keep ultramarine to playing, focus and the one primary action per view (The Now-Playing Rule).
- **Do** separate with 1px rules: ink to open a region, hairline between rows (The Ink-Opens Rule).
- **Do** set every time, size and count in tabular numerals.
- **Do** rank new text by weight and tracked capitals within the three sizes (0.8125rem, 1rem, 1.75rem).
- **Do** keep corners square (0px) on every new control and container.
- **Do** draw new icons on the 24-unit grid with a 1.5 stroke, square caps and mitred joins, in currentColor.
- **Do** use the house ease (cubic-bezier(0.16, 1, 0.3, 1)) at 120 to 300ms, and let reduced motion zero it.
- **Do** define both light and dark values for any new colour token; dark is a graphite sleeve, same roles.

### Don't:
- **Don't** add shadows, gradients, blur or glass; give it a rule or paper-2 instead (The Drawn-Not-Cast Rule).
- **Don't** draw a waveform, a glow or chat bubbles; the timeline is a graduated ruler and AI output is a proposal block.
- **Don't** use splice red for anything but cuts, destructive hover and errors.
- **Don't** introduce a third ground tint or a second accent hue.
- **Don't** put icons in tinted tiles or circles; icons sit bare beside their label or in a square icon button.
- **Don't** add a new type size to create hierarchy.
