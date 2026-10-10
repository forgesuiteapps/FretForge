# v1.6.0

- Written drum ghost/accent tags take priority over velocity inference; real Guitar Pro round-trip checks added.
- Extra media: local six-source Demucs setup/separation, progress logs, automatic vocal/guitar/bass/drum assignment and residual backing mix. Shared alignment and video offsets remain intact.
- Stale or modified separated sets block export until cleared or regenerated.

# v1.5.0

- Ordinary quarter-note notation no longer becomes a sustain automatically; short-tail override and written ringing/bends supported.
- Wider pattern and lyric scrubbers preserve continuous playback.
- Lyric beat/subdivision lines, musical snapping and per-row playhead alignment.
- Hover descriptions on every checkbox option.
- Red ignored and yellow colliding drum rows update after reassignment.
- Music-video audio preview/extraction with linked video offsets and untouched original video export.

# v1.4.1

- GHL ascending/descending pitch maps and preview lanes now follow B1, B2, B3, W1, W2, W3. Correct .chart codes remain unchanged.
- GHL chord voice releases and shared-voice comparisons follow this musical order.
- Fixed preview audio being repeatedly stopped and restarted when idle slider rounding changed the live clock. Applies to pattern and lyric previews and automatic window following.
- Regression checks cover pitch direction, short contextual phrases, and repeated preview redraws with a fractional audio clock.

# v1.4.0 — FretForge

- Application, portable executable and source package renamed; supplied icon embedded and purple brand accent applied.
- Literal Expert policies now apply to all five- and six-fret Guitar/Rhythm/Bass roles.
- Six-fret phrases exceeding six pitches reset positions without dropping events or requiring octave folding.
- Dense significant triads retain their voices; larger voicings prioritize characteristic harmony.
- Compact five-fret melodies and transposed motif consistency; explicit picking/legato/taps and independent Expert releases.
- Source provenance, uncertainty, chord reductions, rounding and resets are auditable in export reports.
- Existing drum, audio alignment, lyrics, BPM and Star Power features retained.

# v1.2.1

- Replaced the pitched YARG vocal screen with a Clone Hero LRC manager; removed vocal MIDI export.
- Added timestamp/text editing, waveform playback, seek controls and aligned LRC copies.
- Extended normal LRC phrase ends to one tick before the next phrase start.
- Repaired enhanced backward timestamps by evenly spacing affected words between surrounding valid lyrics, preserving written order.
- Added persistent repair notices, original/repaired times and LYRIC-REVIEW.txt export records.
- Added first-lyric alignment to the playhead, an exact starting-time control and ±100 ms nudges.
- Kept optional Guitar Pro lyric import as editable LRC, without pitched vocal gameplay.

# v1.2.0

- Added a Vocals screen with Guitar Pro melody/verse selection, pitch and lyric preview, shared recording playback, transposition, syllable edits and phrase breaks.
- Exported Rock Band-style solo vocals for YARG in a complete-band notes.mid, preserving the selected instruments and difficulties.
- Imported beat-attached Guitar Pro lyrics, including repeated sections and separate verses, for Clone Hero chart lyric events and synchronized LRC output.
- Kept vocals, lyrics and phrase markers on the shared BPM and bar-shift timeline; audio alignment remains baked into the exported recordings.
- Added validation for polyphonic melodies, missing syllables, unsupported pitches and lyrics without note attacks.
- Included a repeated vocal demo with two verses and a synthesized melody recording.

Windows x64 portable release and GitHub source ZIP. Solo vocals only; phrase boundaries and Star Power require musical review.
