# FretForge literal Expert implementation

The supplied `5_fret_charting_ruleset.md` and `6_fret_charting_ruleset.md` govern Guitar, Rhythm and Bass in their respective modes. Their literal requirements apply to Expert; lower difficulties remain deliberate subsets.

Expert never thins or retimes attacks for speed. Absolute onsets are rounded once to output ticks; simultaneity and collisions are reported. Five-fret mapping uses compact spans, phrase contour and transposition-invariant repeated motifs. Six-fret mapping uses physical three-column/two-row templates; sequences exceeding six pitches reset the contextual palette rather than fail or become open notes. Every reset or contour compromise is reported. Significant triads retain three buttons even in dense sections. Larger chords keep root, characteristic inner harmony and top voice. Simultaneous unisons and chord reductions are logged.

Unmarked Guitar Pro articulation defaults conservatively to strum and is explicitly reported as inference. Written legato and taps are honored; rapid picking is forced to strum when automatic HOPO rules disagree. Same-button taps are supported. Expert preserves individual whole-tick ringing releases, shortening only same-button/open conflicts; muted, dead, staccato and tremolo attacks have no tail. No automatic audio analysis is performed.

Source events retain sounding pitch, timing, duration, strings/frets and available written technique/track/voice/measure metadata. Track tuning/capo/transposition metadata is retained separately. Playback expansion supplies repeats, tuplets and effect attacks; unmatched metadata is flagged. This is a normalized playback representation, not a lossless archival copy of the entire Guitar Pro document. Preserve the original score.

Every export includes source/target Expert event comparisons and decision logs in `conversion-report.json` and `CHARTING-REVIEW.txt`. Expected and generated onset sets are checked before export. Re-import tests check serialized .chart events. Human audio review and a full-speed Clone Hero controller playtest are still required; this release does not claim automated verification of audible performance or universal game-version compatibility.

Six-fret maximum three buttons and optional simple two-button barres are this app's ergonomic house policy, not an engine-wide chord limit. GHL output uses dedicated track sections and W1/W2/W3=0/1/2, B1/B2/B3=3/4/8. Open=7, forced toggle=5 and tap=6. Opens represent qualified repeated pedals, not every open source string or pitch overflow. Source tempo changes are retained unless the user explicitly enables Constant BPM; the user's BPM and global alignment controls remain authoritative.

## User clarification in v1.4.1

The user specifies GHL pitch order B1, B2, B3, W1, W2, W3 (lowest to highest). This instruction takes precedence over earlier alternating-row scale templates. Melody candidate sequences must follow this order within a position; lower and higher chord voices receive buttons in the same order. Preview lanes follow it too. File note IDs remain B=3/4/8, W=0/1/2. Physical column and row movement are separate ergonomic considerations, rather than replacements for musical order.

## Sustain interpretation in v1.5.0

A Guitar Pro playback duration is not proof of audible ringing. Ordinary quarter-note and shorter durations now have no tail by default; longer or explicitly let-ring/bend notes retain supported releases. Keep short notation sustains explicitly restores the earlier literal-duration behavior. This user-directed interpretation takes precedence over earlier implementation defaults; the source duration and any tail omission remain visible in the fidelity report.
