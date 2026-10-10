# Guitar Pro → Guitar Hero Live: Literal 1:1 Six-Button Charting Ruleset

**Version:** 1.0  
**Target:** Guitar Hero Live-style six-button guitar (three black buttons and three white buttons)  
**Difficulty:** Expert  
**Objective:** Translate a Guitar Pro performance into the closest *playable* six-button performance, preserving all meaningful attacks, rhythm, articulation, and phrasing without arbitrary simplification.

> **Compatibility note:** Guitar Hero Live's six-button format is fundamentally different from classic five-fret Guitar Hero. This document defines the musical charting rules independently of any particular editor or export format. Validate note encodings, HOPO/forced-note support, sustain behavior, open notes, and chord restrictions against the exact game, custom engine, and chart format being targeted. Do not assume five-fret `.chart` note lanes can encode GHL charts unchanged.

## 1. Controller model and notation

The six buttons are arranged as three positions, each with a **black (upper)** and **white (lower)** button:

| Position | Upper row | Lower row |
|---|---|---|
| 1 (left) | B1 | W1 |
| 2 (middle) | B2 | W2 |
| 3 (right) | B3 | W3 |

Use `B1 B2 B3` and `W1 W2 W3` throughout this specification. `OPEN` means strumming with no fret buttons held, *only when supported by the target chart format and game mode*. A single button is a single note; simultaneous buttons form a chord. Black and white are **button rows**, not direct equivalents of low/high guitar strings, sharp/natural notes, or fixed musical pitches.

### Core controller rules

1. Never interpret the six buttons as six independent, linearly ordered guitar frets.
2. Distinguish **horizontal position** (1–3) from **row** (black/white).
3. Model natural finger placement and movement between positions and rows.
4. Preserve consistent physical shapes for repeated motifs and chord types.
5. Use `OPEN` intentionally; do not map every Guitar Pro open string to an open note.
6. Reject button combinations that the selected target engine cannot encode or that are physically unreasonable at the required speed.

## 2. Non-negotiable musical fidelity

1. Preserve every distinct, meaningful **attack onset** on Expert; fast passages do not justify thinning notes.
2. Preserve the exact tempo map, meter changes, rhythmic subdivisions, tuplets, and deliberate syncopation.
3. Preserve simultaneous attacks as one event with multiple held buttons, subject to valid chord encoding.
4. Preserve the distinction between picked, legato, and tapped notes when the target engine supports it.
5. Do not invent extra attacks for bends, slides, vibrato, or let-ring indications.
6. Preserve recurring riffs, motifs, and chord movement consistently.
7. Never randomize button assignments to increase variety.
8. Make any unavoidable loss of detail explicit in a conversion report.
9. Treat playable equivalence as the goal—not a claim that six buttons can preserve every acoustic property of a guitar performance.

## 3. Source extraction and normalization

Extract source information before assigning buttons:

| Field | Meaning |
|---|---|
| Track and instrument | Intended lead/rhythm part and instrument context |
| Measure, beat, tick | Exact rational musical onset |
| Duration | Notated and, if available, effective sounding duration |
| MIDI pitch | Sounding pitch after tuning, capo, and transposition |
| String and fret | Original physical guitar fingering |
| Voice | Independent Guitar Pro voice |
| Chord/event ID | Notes that sound at the same attack |
| Technique | Pick, hammer-on, pull-off, tap, slide, bend, palm mute, etc. |
| Tie and sustain | Whether the note is reattacked or merely continues |
| Phrase and motif ID | Musical grouping and repeated-pattern identity |
| Confidence | Whether the articulation or duration is ambiguous |

- Resolve alternate tunings and capo positions before pitch comparisons.
- Keep tied notes as continuations rather than separate attacks.
- Do not conflate simultaneous notes from multiple voices with sequential notes.
- When multiple guitar tracks exist, choose the intended part explicitly; do not merge unrelated lead and rhythm parts automatically.
- Use audio to resolve ambiguous transcription details when available. An unmarked Guitar Pro note is not proof of a specific picking technique.

## 4. Timing and note-density rules

1. Build a complete event timeline before button mapping.
2. Keep each independent picked attack, including repeated identical notes and tremolo strokes.
3. Maintain exact triplet, quintuplet, and other tuplet spacing; do not straighten them.
4. Do not change onsets to make finger transitions easier.
5. Keep the source tempo map separate from the rendered button mapping.
6. Avoid collapsing closely spaced arpeggio notes into one chord.
7. Preserve rolled chord onsets when explicitly notated as separate attacks.
8. If the target format's timing resolution cannot express a source event exactly, choose the nearest supported time and report the quantization error.

## 5. Phrase-level six-button mapping

The six-button controller has **three horizontal positions and two rows**, not a six-step linear pitch axis. Assign button sequences based on **relative pitch contour, original string/fret gestures, recurring motifs, and natural 3×2 hand movement**.

### Rules

1. Analyze phrases and their transitions before assigning buttons.
2. Favor consistent horizontal movement for a smooth melodic rise or fall when musically appropriate.
3. Use changes between black and white rows to represent additional distinct pitches or physical movement, but **do not assert that one row is always higher in pitch**.
4. Within a phrase, assign identical pitches to identical buttons unless a deliberate fingering-position shift improves the performance translation.
5. Preserve local pitch ordering wherever possible; document where the 3×2 layout forces an exception.
6. Preserve recognizable interval and contour patterns without creating needless row zigzags.
7. Keep repeated notes and pedal tones on stable buttons or `OPEN` when musically and mechanically justified.
8. Preserve motif identity across sections; transposed motifs should retain similar physical patterns when feasible.
9. Consider previous and next phrases when choosing an assignment.
10. Reuse buttons and reset a mapping at musical boundaries when more than six pitches occur; never drop notes merely because the phrase exceeds six distinct pitches.

### Mapping guidance, not fixed pitch tables

- A short three-note scale fragment can often be represented by movement across positions 1 → 2 → 3 on one row.
- A wider six-note figure may use both rows, but its traversal order must be selected for ergonomic continuity, not by an assumed universal `B1 → B2 → B3 → W1 → W2 → W3` pitch scale.
- A repeated pedal note should keep a stable button while moving notes occupy neighboring positions/rows.
- A descending passage should generally reverse its phrase's chosen physical gesture rather than receive unrelated buttons.

**Never use a global MIDI-pitch-to-button lookup.**

## 6. Translating real guitar fingering

Guitar Pro string and fret positions are useful clues about the original guitarist's motions but are not one-to-one controller instructions.

1. Prefer compact gestures for adjacent real-guitar frets when they also preserve melodic contour.
2. Use row changes when they naturally reflect a change in fingering, string grouping, or voicing.
3. Do not automatically map lower guitar strings to black buttons or higher strings to white buttons.
4. Recognize that the same sounding pitch may be played on different strings; a string change alone does not require a new button.
5. Prefer a stable left-to-right hand position through a phrase; avoid gratuitous full-position resets.
6. Avoid patterns that require rapid, repeated row switches for no musical benefit.
7. When fidelity permits several mappings, choose the one that best resembles the source's alternation, anchoring, stretching, and position shifts.

## 7. Chord translation and voicing

A real guitar may sound two to six strings simultaneously. The six-button controller does not imply that six-button chords are practical or supported. **Use only chord combinations validated for the exact target engine.**

1. Preserve the onset of each strummed chord.
2. Preserve two distinct voices as a two-button chord when supported and playable.
3. Represent larger voicings using a valid, ergonomic subset of buttons that preserves harmonic identity and voice leading.
4. Prefer root, characteristic third/suspension, seventh when essential, melodic top voice, and moving inner voice according to musical context—not a rigid “three highest notes” rule.
5. Reduce octave doubling and unison doubling before discarding independent harmonic information.
6. Keep power-chord shapes consistent throughout a riff.
7. Maintain common tones in stable button positions where possible.
8. Avoid physically conflicting same-position upper/lower combinations when the required hand posture or target engine makes them impractical; do not assume every mathematically possible button set is a good chord.
9. Do not turn a simultaneous chord into an arpeggio merely to fit the controller.
10. Do not collapse a genuine arpeggio into a simultaneous chord.
11. Report all chord reductions and why the original voicing could not be preserved.

**Default policy:** Favor two- and three-button chords when they express the source well, but **do not enforce an invented universal maximum**. Validate permitted chord combinations and practical limits for the chosen GHL-compatible implementation.

## 8. Open notes

`OPEN` represents a strum with no buttons pressed when supported.

- Do **not** translate every real open string to `OPEN`.
- Consider `OPEN` for an exposed repeated open-string pedal tone or a phrase where a no-fret strum provides a faithful physical gesture.
- Preserve separate repeated `OPEN` attacks as separate events.
- Do not combine `OPEN` with pressed-button notes unless the target engine explicitly supports that representation.
- Avoid using `OPEN` as a generic lowest pitch or an automatic overflow lane.
- Verify open-note support and encoding in the intended chart format.

## 9. Strums, HOPOs, taps, and forced notes

1. Classify each event's articulation independently of its button assignment.
2. Explicit picked notes should require a strum, even when fast timing would otherwise trigger an automatic HOPO.
3. Hammer-ons and pull-offs should use supported HOPO mechanics where valid.
4. Tapped notes should use tap mechanics **only if the target engine supports them**; otherwise choose the closest supported articulation and report the limitation.
5. Repeated same-button notes ordinarily require separate strums unless a supported mechanic explicitly permits otherwise.
6. Preserve the difference between a fast alternate-picked run and a legato run.
7. Use forced HOPO/strum flags when supported and necessary to override automatic behavior.
8. Do not assume the HOPO thresholds or forced-note encodings from classic five-fret Guitar Hero apply unchanged to GHL.
9. Test rapid black/white alternations and chords for correct engine recognition.
10. For ambiguous Guitar Pro articulation, use a consistent conservative default and mark the event for review.

## 10. Special guitar techniques

| Guitar Pro technique | Six-button treatment |
|---|---|
| Hammer-on | HOPO if supported and valid |
| Pull-off | HOPO if supported and valid |
| Tapping | Tap mechanic if supported; otherwise documented fallback |
| Alternate/tremolo picking | Every independent picked attack is a separate strum event |
| Trill | Every distinct articulated note, with stable alternating buttons |
| Palm mute | Preserve attacks; use short/detached notes when appropriate |
| Slide | Preserve distinct endpoints only if they represent meaningful chart events; avoid invented attacks |
| Bend / pre-bend / release | Usually one sustained event unless separately reattacked |
| Vibrato | Sustain where musically appropriate; no artificial extra gems |
| Natural/artificial harmonic | Preserve the note attack; no arbitrary special gem |
| Dead note | Short percussive event for each meaningful attack |
| Grace note | Preserve if separately audible/notated and timing permits |
| Let ring | Sustain when appropriate and representable |
| Tied note | Continue the original note, do not create a new attack |

## 11. Sustain and note-off behavior

1. Keep exact source durations internally, even if the target format has sustain limitations.
2. Use sustain tails for clearly ringing or held notes.
3. Do not create long tails for detached palm-muted or staccato events just because the notation uses a long beat value.
4. Avoid unsupported or misleading overlapping sustains.
5. Do not create repeated gems to approximate a long note.
6. Preserve note-on timing as the higher priority if a target format forces duration quantization.
7. Record sustain shortening, overlap resolution, and duration approximation in the conversion report.

## 12. Repetition and consistency

- Identical source riffs should use identical button patterns in comparable contexts.
- One-note variations should produce localized changes, not complete remappings.
- Maintain a consistent vocabulary for power chords, arpeggios, pedal tones, and repeated melodic cells.
- Favor continuity across verse, chorus, bridge, and solo repetitions.
- Preserve deliberate differences in articulation even when pitches are identical.
- Cache motif-level candidate mappings and evaluate them jointly across the song.

## 13. Playability: literal but human

**Never remove notes or move their timing to make a phrase easier.** Instead, optimize the six-button arrangement among musically acceptable alternatives.

Prioritize:

1. Stable hand position and natural access to the three horizontal positions.
2. Economical black/white row changes.
3. Consistent pedal-tone anchors and repeated chord shapes.
4. Natural direction changes and phrase resets.
5. Minimal awkward simultaneous-button grips.
6. Avoidance of abrupt transitions that are difficult for reasons unrelated to the music.
7. Correct articulation through fast passages.

A hard but musically justified passage is acceptable. Arbitrary difficulty is not.

## 14. Suggested automated conversion algorithm

### Stage A — Parse

Read Guitar Pro tracks, tuning, tempo map, notes, ties, chords, voices, and techniques. Normalize to exact beat positions and sounding pitches.

### Stage B — Group events

Group truly simultaneous attacks; distinguish continuing/tied notes from new attacks. Identify phrases, riffs, repeated motifs, and chord progressions.

### Stage C — Enumerate candidates

Generate several valid six-button patterns for each phrase. Candidate generation must account for both **row** and **position**, legal chord combinations, and `OPEN` availability.

### Stage D — Hard validation

Reject candidates that:

- Change meaningful onset times or omit attacks without an explicitly documented format limitation.
- Use unsupported buttons, open-note combinations, chords, or articulation flags.
- Merge sequential attacks into chords or split simultaneous attacks arbitrarily.
- Contradict required picking/legato distinctions when the engine can represent them.

### Stage E — Score remaining candidates

Minimize a weighted cost such as:

`Cost = wp*ContourLoss + wm*MotifInconsistency + we*ErgonomicDifficulty + wc*ChordVoiceLoss + wt*TransitionCost + wr*UnnecessaryRowSwitches`

Use weights with **musical fidelity and motif consistency ahead of cosmetic variety**. Do not put timing or attack-count errors in the soft cost: they are hard constraints.

Suggested additional scoring considerations:

- Penalty for reversing a local ascending/descending figure without good reason.
- Penalty for inconsistent repeated-pitch button identity.
- Penalty for unnecessary jumps across positions 1–3.
- Penalty for repeated black/white switches that do not express a musical gesture.
- Penalty for changing stable chord tones between adjacent chords.
- Penalty for a phrase ending in a posture that makes the next phrase awkward.

### Stage F — Articulation

Apply strum/HOPO/tap/forced-note markers *after* button mapping, checking that the resulting notes actually behave as intended in the target engine.

### Stage G — Export

Export to a **verified GHL-compatible chart format and track encoding**. Include the correct tempo and time-signature map, six-button note events, valid chords, sustains, and supported articulation markers. Do not silently substitute classic five-fret lane encodings.

### Stage H — Verify and playtest

Compare exported events against the source event list and test playback in the exact intended engine.

## 15. Required conversion report

For each chart, provide:

- Source file, selected guitar track, tuning, and target engine/format.
- Source attack count and output attack count, including reasons for any differences.
- Timing discrepancies and quantization errors.
- Chords reduced or altered due to controller or format constraints.
- Uncertain source articulations and any fallbacks.
- Unsupported taps, forced notes, opens, or sustains.
- Phrase mappings that necessarily break melodic contour.
- Any manual review items.

## 16. Validation checklist

- [ ] Correct target engine and GHL six-button format are specified.
- [ ] Button rows and positions are encoded correctly.
- [ ] Tempo map, meter, and note onsets match Guitar Pro.
- [ ] All meaningful attacks are preserved or explicitly documented as exceptions.
- [ ] Ties and sustained notes are not mistaken for new attacks.
- [ ] Simultaneous chords and sequential arpeggios are distinguished.
- [ ] All chord button combinations are supported and playable.
- [ ] Open notes are intentional and valid.
- [ ] Strum, HOPO, tap, and forced-note behavior has been verified.
- [ ] Repeated motifs use consistent button mappings.
- [ ] Melodic contour is preserved as closely as the 3×2 layout allows.
- [ ] Row switching and hand-position changes are musically justified.
- [ ] Sustains and overlaps have been reviewed.
- [ ] No extra attacks were invented for continuous pitch effects.
- [ ] All unavoidable compromises appear in the conversion report.
- [ ] The exported chart has been playtested in the target engine.

## 17. Final governing principle

> **Chart the original guitarist's attacks, rhythm, articulation, voicings, and physical gestures as literally as the Guitar Hero Live six-button controller permits. Optimize the 3×2 button pattern for human hands; never sacrifice musical events for convenience or mistake the two rows for a simple six-fret pitch ladder.**

