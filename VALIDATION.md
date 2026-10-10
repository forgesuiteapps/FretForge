# FretForge v1.6.0 validation

- 74 automated tests passed (one optional native audio-device test skipped).
- Real exported/imported Guitar Pro drum fixture: 20 ghost tags and 40 normal/heavy accent tags recovered. Expert markers persist with inference on/off; explicit ghosts at velocity 127 and accents at velocity 1 retain their written meaning.
- Synthetic six-source audio verifies equal-duration validation, atomic four-stem assignment, residual backing sum, stale-source/modified-stem rejection, clear semantics and saved-project round trips.
- Actual FFmpeg exports verify backing replaces the original full mix, and main audio plus four stems share zero/positive/negative alignment.
- Windows release launched successfully, Extra media layout inspected, and four real Guitar Pro conversion smoke checks passed.
- CPU Demucs model inference and dependency installation were not run in this environment (Python has no Torch/Demucs). Those runtime operations remain dependent on the selected Python, internet downloads and available disk/memory. Separation quality has not been assessed by ear.

# FretForge v1.5.0 validation

Windows validation, 10 October 2026.

- 71 automated checks passed, with the optional native playback test executed separately.
- Quarter-note tails are absent by default on all six pitched roles; longer notes, explicit ringing and the short-tail override are verified.
- Lyric snapping is checked across tempo changes, recording/lyric offsets and optional 1/16-beat resolution.
- Rendered UI tests verify row-to-playhead snapping, beat-grid labels, wide scrubber geometry, unchanged fractional clocks during idle redraws, and red/yellow drum labels changing after reassignment.
- A real synthetic AVI with audio is decoded for preview and exported with zero, positive and negative alignment. Audio sample counts, original video bytes, linked video offsets, .chart audio references and export reports are checked.
- Existing GHL order, charting, drums, Star Power, BPM, audio, lyric repairs and migration checks pass.

Final Windows release, four Guitar Pro demo conversions, screenshots and ZIP/source/executable checks are performed during packaging. Clone Hero controller/video playback testing and an audible assessment of user recordings are not claimed.

# FretForge v1.4.1 validation

Windows validation, 10 October 2026.

- 66 standard automated checks passed.
- The optional native audio-device test was also explicitly run and passed: playback clock advancement, pause stability, seeking, stop, and silence/trim offsets.
- Ascending, descending and narrow six-fret phrases preserve B1/B2/B3/W1/W2/W3 musical order, with the proper export codes 3/4/8/0/1/2.
- Repeated idle redraws of Pattern and Lyrics previews leave fractional playback positions unchanged and do not trigger seeks. The preview-window slider also preserves programmatic follow positions.
- Corrected chord sustain tests inspect source voices in musical order rather than numeric file-code order.
- Existing charting, drums, Star Power, BPM, alignment, lyrics and audio-export checks pass.

The final Windows release and four real Guitar Pro demo exports are checked during packaging, along with preview screenshots and ZIP contents. Native audio-device checks use zero volume; an audible assessment with the user's recording and Clone Hero controller playtest are not claimed.

# FretForge v1.4.0 validation

Windows validation, 10 October 2026.

- 64 automated checks passed; one optional native audio-device check is excluded.
- All six pitched roles preserve long irregular Expert runs and exact rounded onsets; six-fret overflow resets rather than stopping export.
- Dense triads retain three voices; individual disjoint releases, short Expert tails, explicit strums, legato and repeated same-button taps are checked.
- Compact five-fret narrow melodies and transposed motif consistency are checked.
- Rounding collisions, chord losses and unspecified-articulation inference appear in reports.
- Serialized .chart notes are parsed back and compared against generated events for all six pitched roles and four difficulties.
- Existing drums/cymbals/collisions, BPM override, synchronized offsets, Star Power, LRC repairs/alignment and audio silence/trim checks pass.

Final Windows release, actual Guitar Pro demo conversions, rendered screenshots and ZIP contents are checked during packaging. The icon is embedded in the executable and window.

No Clone Hero controller playtest or automatic musical analysis of audio is claimed. Expert is literal within documented representation limits; lower difficulties remain intentional reductions. Preserve the source score and review reported decisions against the recording.
