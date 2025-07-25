- [x] Don't show the "shift for multi select" label when showing the gui on a phone, or when the screen is too narrow for it to fit comfortably

## Review Comments from PR #7

- [x] Rename `MIN_WIDTH_FOR_LABEL` to a more descriptive name like `MOBILE_SCREEN_WIDTH_THRESHOLD` or `MOBILE_BREAKPOINT_WIDTH` (src/app.rs, line ~186)
- [x] Use `Array.prototype.includes()` instead of `indexOf` for better readability in cache whitelist check (assets/sw.js, line ~38)
- [x] Use `const` instead of `var` for `cacheWhitelist` since it's not reassigned (assets/sw.js, line ~29)
- [x] Extract magic number 480.0 to a module-level constant with descriptive name for better maintainability (src/app.rs, line ~186)

## Review Comments from PR #8

- [x] Extract magic number 0.2 to a named constant to improve code maintainability and readability (src/app.rs)
- [x] Extract magic numbers -1.0 and 6.0 to named constants to improve code maintainability and readability (src/app.rs)
- [x] Extract magic number 2.0 to named constants to improve code maintainability and readability (src/app.rs)
- [x] Extract magic number 10.0 to a named constant to improve code maintainability and readability (src/app.rs)
- [x] Extract magic number 0.8 to a named constant to improve code maintainability and readability (src/app.rs)
- [x] Extract magic number 0.5 to a named constant to improve code maintainability and readability (src/app.rs)
- [x] Extract magic number 1.5 to a named constant to improve code maintainability and readability (src/app.rs)
- [x] Extract magic numbers -4.0 and 4.0 to named constants to improve code maintainability and readability (src/app.rs)
- [x] Update comment to describe 'rotated text' instead of 'vertical text' for accuracy (src/app.rs)

- [x] Add some graphic that hints the user that they should press the "mute" button to enable audio
- [ ] Go through the codebase and make sure the guidelines from the instructions file are applied.
  - [x] Check for and update rust edition to 2024 in all `Cargo.toml` files.
  - [x] Find and refactor any `mod.rs` files to the `module_name.rs` convention.
  - [x] Replace `format!("{}", ...)` with `format!("{...}")` for better readability.
  - [x] Find and replace magic numbers with named constants.
  - [x] Review ignored errors (`let _ = ...`) and add comments if missing.
  - [x] Run `cargo check` on all crates to check for warnings, including non-exhaustive matches.
  - [x] Run 'cargo clippy'
- [x] Only show "click to enable audio" hint when unitialized, not when muted
- [ ] Fix these todos
  - [ ] interval.rs: perhaps this is overcomplicated. better to just use the base_dissonance directly?
    - [ ] Research and document the impact of complexity and error factors on dissonance perception
    - [ ] Create a simplified version that only uses base_dissonance values
    - [ ] Compare outputs between complex and simple versions across all intervals
    - [ ] Run user testing or perceptual studies to determine which approach feels more accurate
    - [ ] Decide whether to keep complex algorithm or switch to simple base values
  - [ ] piano_gui.rs: handle multi touch? is it possible to do it since this is just a single widget?
    - [ ] Research egui's MultiTouchInfo API and how to access it in the current context
    - [ ] Determine the desired multi-touch behavior (simultaneous key presses, gestures, etc.)
    - [ ] Implement pointer tracking to handle multiple simultaneous touches
    - [ ] Test multi-touch functionality on mobile devices and touch screens
    - [ ] Ensure multi-touch doesn't break existing mouse and single-touch interactions
- [ ] Change the order of the interval displays so the bottom row shows the first pressed note when using the mouse, and the actual base when using a midi keyboard.
  - [ ] Modify PianoGui to track the chronological order of mouse key presses
  - [ ] Define what "actual base" means for MIDI keyboard input (e.g., lowest note, root note, etc.)
  - [ ] Add input source tracking to distinguish between mouse and MIDI input for each pressed key
  - [ ] Modify interval_display.rs to use different ordering logic based on input method
  - [ ] Update the pressed_keys data structure to include ordering/priority information
  - [ ] Test the new ordering behavior with both mouse and MIDI input
