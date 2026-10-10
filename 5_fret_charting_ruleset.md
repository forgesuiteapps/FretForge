# Clone Hero Expert 5-Fret — Literal Guitar Pro Charting Ruleset

**Version:** 1.0  
**Target:** Clone Hero, Expert, five-fret guitar  
**Input:** Guitar Pro (`.gp3`, `.gp4`, `.gp5`, `.gpx`, `.gp`)  
**Output:** Clone Hero-compatible `.chart` or MIDI  
**Objective:** The most literal *playable* representation of the source guitar performance within the target engine's mechanics.

## 1. Core principles

1. Preserve every distinct, meaningful note attack. Do not simplify for speed or difficulty.
2. Preserve exact rhythmic positions, subdivisions, rests, tempo changes, and time signatures.
3. Preserve source articulation: picked attacks, hammer-ons, pull-offs, tapping, and detached notes.
4. Preserve simultaneous attacks as chord events; preserve staggered attacks as separate events.
5. Preserve melodic contour, riff identity, chord movement, and recurring patterns.
6. Choose human-playable fret patterns only among mappings that preserve higher-priority musical information.
7. Never invent note attacks to imitate bends, vibrato, slides, or sustain.
8. Never assign colors randomly or from a permanent pitch-to-color lookup.
9. Record unavoidable representational compromises instead of silently discarding information.
10. Distinguish what is explicitly encoded in Guitar Pro from what must be inferred.

**Priority order:** timing and attacks → simultaneity → articulation → motif consistency → melodic contour → chord/voice movement → interval relationships → ergonomics → visual variety.

## 2. Input extraction and normalization

Extract for every note or attack event:

- Absolute onset in rational beat units and source ticks, with the complete tempo map.
- Notated duration and, where distinguishable, sounding/ringing duration.
- Sounding pitch (including tuning, capo, and transposition), original string, and original fret.
- Guitar Pro track, voice, measure, and chord membership.
- Attack type: explicitly picked, hammer-on, pull-off, tapped, or unspecified.
- Effects: slide, bend, tremolo, trill, palm mute, vibrato, harmonic, dead note, grace note, let ring, etc.
- Phrase/motif identity and corresponding events in repeated passages.
- Source confidence or ambiguity flags when the transcription does not determine an action.

Do not assign Clone Hero fret colors during extraction. Keep a lossless intermediate representation of the source events, including details that cannot be directly displayed in Clone Hero.

### Source interpretation

- Different guitar strings can produce the same pitch; string changes do not automatically require lane changes.
- A chord comprises simultaneous note attacks, not merely notes whose sustain intervals overlap.
- A rolled chord or arpeggio with distinct onsets must remain sequential.
- Notated durations are not always reliable evidence of audible sustain, especially for palm-muted notes.
- Do not assume unmarked Guitar Pro notes were definitely picked; use a consistent documented inference policy and flag uncertainty.
- If audio is available, use it to resolve ambiguous attacks, timing, and sustain, while keeping Guitar Pro as the primary transcription source.

## 3. Timing and event preservation

1. Map every distinct meaningful source attack to one chart event at the corresponding onset.
2. Preserve separately picked repeated notes as separate events, including same-pitch repetitions.
3. Preserve triplets, tuplets, sixteenths, thirty-seconds, tremolo picking, and irregular subdivisions.
4. Do not merge close notes, stretch intervals, or snap expressive timing to a different rhythm simply to make a pattern easier.
5. Preserve all BPM and time-signature changes in the sync track.
6. Keep onset timing independent from lane assignment and note-type assignment.
7. For simultaneous source notes, create one chord event, subject to the supported maximum chord size.
8. Never create multiple attacks merely to simulate one long note.
9. When source precision exceeds output tick resolution, use a consistent rounding strategy, minimize accumulated drift, and report collisions or irreducible timing loss.

## 4. Five-fret pitch mapping

Available colored lanes, low to high: **Green (G), Red (R), Yellow (Y), Blue (B), Orange (O)**.

### Phrase-level mapping

1. Segment the song into riffs, melodic phrases, arpeggios, chord progressions, solos, and transitions before mapping.
2. Map phrases contextually, not each note independently.
3. Preserve ascending and descending melodic direction wherever possible.
4. Keep repeated pitches on the same lane within a stable fingering context.
5. Reflect larger intervals with larger lane movement where feasible, without sacrificing natural playability.
6. Preserve phrase peaks, low points, anchor notes, and distinctive interval patterns.
7. Prefer compact lane ranges for narrow melodies; expand the range for wider phrases.
8. Never use a universal fixed MIDI-pitch-to-color assignment.
9. Reuse the same fret sequence for identical repeated riffs unless context makes a deliberate shift necessary.
10. Preserve the shape of transposed motifs where possible.

### When a phrase exceeds five distinct pitches

1. **Do not delete notes.** The five-lane limit restricts simultaneous and spatial representation, not the number of sequential events.
2. Reuse lanes or shift the mapping at musically logical reset points.
3. Prefer phrase boundaries, rests, motif boundaries, or articulation breaks for resets.
4. Preserve local contour even when global pitch order cannot fit on five lanes.
5. Avoid disruptive Green–Orange or Orange–Green jumps inside smooth legato passages unless they are the best available representation.
6. Evaluate the previous and following phrases before choosing a reset.
7. Treat each reset as a documented representational decision rather than a pitch change in the source.

## 5. Chord translation

**Default mapping:**

| Source event | Default target |
|---|---|
| Single note | Single gem |
| Two distinct musical voices | Two-gem chord |
| Three distinct musical voices | Three-gem chord |
| Four or more simultaneous voices | Reduced chord, at most three colored gems |

Rules:

1. Preserve the chord's attack time and repeated chord attacks.
2. Prefer two-gem chords for power chords and meaningful double-stops.
3. Use three-gem chords for harmonically or technically significant fuller voicings.
4. When reducing chords, prioritize characteristic harmony, root, moving top voice, and important voice-leading; do not mechanically choose only the highest or lowest pitches.
5. Octave doublings and unisons may be represented with fewer gems when that better conveys the musical event.
6. Preserve stationary voices and moving voices across chord progressions when possible.
7. Use consistent chord-shape vocabulary across repeated riffs.
8. Avoid arbitrary wide stretches and excessive three-note shapes where a more natural equally faithful option exists.
9. Never turn a genuinely simultaneous strum into an arpeggio, or a genuine arpeggio into a simultaneous chord.
10. Never create duplicate gems on the same lane at the same onset.

## 6. Clone Hero articulation and note types

Do not rely solely on the engine's automatic HOPO threshold. Determine intended articulation first, then encode or override the engine's default behavior.

| Source event | Intended target behavior |
|---|---|
| Explicit picked attack | Strum-required gem |
| Hammer-on | HOPO where supported |
| Pull-off | HOPO where supported |
| Two-hand tap | Tap gem when supported |
| Repeated picked pitch | Separate strum-required gem |
| Tremolo-picked passage | One strum-required event per picked attack |
| Trill | All distinct articulated events; HOPO/tap according to source |
| Palm-muted attack | Usually short, strum-required gem |
| Bend / vibrato | One note event with appropriate sustain; no invented attack |
| Slide | Preserve separately articulated or explicitly distinct targets; do not invent intermediate attacks |
| Harmonic | Preserve onset and duration as a normal gem unless supported mechanics offer a faithful distinction |
| Dead note | Short percussive gem if musically significant |
| Grace note | Preserve if it has a distinct meaningful onset and can be encoded |
| Let ring | Sustain where musically and technically appropriate |

Additional rules:

1. Force strum on fast picked notes that the game would otherwise automatically treat as HOPOs.
2. Force HOPO on source legato notes when automatic rules would otherwise require a strum, provided the engine supports the intended mechanic.
3. Mark actual tapping passages as taps rather than labeling all fast solos as tapping.
4. Same-lane repetitions generally require new strums unless the engine's explicitly supported mechanic and source technique justify otherwise.
5. Verify HOPO and tap behavior in the selected Clone Hero version and output format.
6. If a source technique has no faithful five-fret equivalent, preserve its onset/rhythm and log the omitted expressive detail.

## 7. Open-note policy

Clone Hero supports open notes, but a Guitar Pro open string does **not** automatically become a Clone Hero open gem.

1. Use open notes only when they communicate a meaningful repeated open-string or pedal-tone gesture and work naturally in the surrounding chart.
2. Prefer a stable colored-lane anchor when open-note behavior would disrupt the phrase or add unnecessary complexity.
3. Keep open-note use consistent across repeated riffs.
4. Treat an open gem as a distinct input state, not as an additional colored fret.
5. Validate open-note and open-sustain compatibility with the chosen export format and game version.

## 8. Sustain and note-off policy

1. Represent genuinely ringing notes with appropriate sustain tails.
2. Do not infer long audible sustain solely from a long notated duration.
3. Keep palm-muted, staccato, and dead-note events short when their articulation warrants it.
4. Preserve the source's intended note-off time where the game can represent it.
5. Do not add extra gems to represent a sustain or continuous pitch effect.
6. Handle overlapping sustained voices within the target engine's actual support; do not silently truncate important notes.
7. Apply any minimum-tail threshold consistently and document deviations caused by output-format or gameplay limitations.

## 9. Human playability without simplification

1. Prefer ordinary hand positions and natural index-to-pinky movement among musically equivalent mappings.
2. Keep repeated pedal notes or riff anchors on consistent lanes.
3. Use stable alternating fret pairs for trills and back-and-forth motifs.
4. Avoid unnecessary full-range jumps and erratic zigzags.
5. Preserve real technical density; difficulty alone never justifies removing or retiming notes.
6. Choose chord shapes that permit coherent transitions between successive events.
7. Consider entry and exit transitions when optimizing each phrase.
8. A difficult but faithful chart is better than an easy but inaccurate chart; an awkward chart is not better than an equally faithful ergonomic chart.

## 10. Repetition and global consistency

1. Detect identical and near-identical motifs before final lane assignment.
2. Map identical motifs identically unless there is a concrete musical or transition-related reason to differ.
3. Reflect small source variations with corresponding local changes in the chart.
4. Keep recurring chord shapes, pedal tones, and scale figures consistent.
5. Optimize repeated motifs jointly, not as unrelated passages.
6. Avoid arbitrary color variety; consistency is more valuable than using every lane frequently.

## 11. Deterministic conversion pipeline

1. **Parse:** Load the Guitar Pro file and select the intended guitar track(s).
2. **Normalize:** Resolve tuning, transposition, tempo, beats, voices, and techniques into a lossless event model.
3. **Group:** Identify simultaneous chord attacks and distinct sequential attacks.
4. **Segment:** Detect phrases, repeated motifs, chord progressions, and transitions.
5. **Generate candidates:** Produce several valid lane mappings per phrase, including possible position resets and chord reductions.
6. **Reject invalid candidates:** Eliminate mappings with illegal lane combinations, impossible simultaneous events, or unsupported mechanics.
7. **Optimize:** Choose mappings that preserve musical contour, motif consistency, voice-leading, and natural movement.
8. **Assign articulation:** Encode strums, HOPOs, taps, and forced overrides from the source's technique information.
9. **Assign sustains:** Set note lengths according to musical duration and supported game behavior.
10. **Export:** Write a Clone Hero-compatible `.chart` or MIDI with correct sync and Expert guitar note data.
11. **Validate:** Check the exported file against the source event model and playtest in Clone Hero.
12. **Report:** List every unavoidable reduction, inference, collision, or unrepresented technique.

### Optimization model

Minimize a weighted mapping cost over valid candidates:

\[
C = w_p C_{\text{contour}} + w_m C_{\text{motifs}} + w_v C_{\text{voice-leading}} + w_f C_{\text{fingering}} + w_j C_{\text{jumps}} + w_s C_{\text{resets}}
\]

- **Contour cost:** Loss of local pitch direction, phrase peaks, or interval relationships.
- **Motif cost:** Inconsistent mappings of repeated passages.
- **Voice-leading cost:** Loss of stationary and moving chord voices.
- **Fingering cost:** Unnatural hand shapes or transitions.
- **Jump cost:** Unnecessary large fret movements.
- **Reset cost:** Abrupt or musically illogical position shifts.

**Hard constraints:** preserve required attack count and onset timing, preserve simultaneous event grouping, respect the supported lane/chord limits, and encode the intended articulation wherever the engine permits. Do not let a weighted score trade away these requirements.

Use deterministic tie-breaking so the same source and settings produce the same chart.

## 12. Validation checklist

- [ ] Correct source guitar track selected; no accidental merging of unrelated parts.
- [ ] All meaningful source attacks represented, except documented unavoidable reductions.
- [ ] Onsets, subdivisions, tempo map, and time signatures preserved.
- [ ] Simultaneous attacks grouped correctly; staggered attacks remain separate.
- [ ] No illegal chord sizes or duplicate simultaneous lanes.
- [ ] Picked notes, HOPOs, and taps intentionally encoded.
- [ ] Automatic HOPO behavior checked and overridden when necessary.
- [ ] Repeated riffs and chord shapes mapped consistently.
- [ ] Melodic contour and distinctive phrase shapes preserved as closely as possible.
- [ ] Sustain lengths reviewed against source notation and articulation.
- [ ] No invented attacks for bends, slides, vibrato, or ringing notes.
- [ ] Open-note usage consistent and technically supported.
- [ ] Exported `.chart` or MIDI re-imported and inspected for timing/note-type errors.
- [ ] Chart playtested in Clone Hero at full speed.
- [ ] All ambiguity, rounding, reduction, and unsupported-technique cases logged.

## 13. Non-negotiable prohibitions

- Never retime music to make fret placement easier.
- Never remove fast notes solely to reduce difficulty.
- Never convert explicit picked attacks into HOPOs merely because they are close together.
- Never invent notes to simulate continuous pitch changes.
- Never map each pitch independently without phrase context.
- Never use fixed pitch-to-color mapping across the entire song.
- Never change repeated riffs arbitrarily.
- Never exceed supported simultaneous gem limits.
- Never turn a rolled/arpeggiated event into a block chord, or vice versa.
- Never hide a representational loss or unsupported technique.

## 14. Final acceptance standard

A successful conversion is **Clone Hero Expert, five-fret, rhythmically 1:1, articulation-aware, phrase-consistent, and physically playable**. Every meaningful Guitar Pro performance action should have the closest feasible corresponding action in Clone Hero. The only permitted departures are those required by the five-fret interface, the target chart format, or genuine source ambiguity; these departures must be explicit and auditable.
