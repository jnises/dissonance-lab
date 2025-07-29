- [x] When developing I need to the frontend logging to be piped back to the backend and displayed in the terminal.

    ### Current Status & Usage:
    - ✅ HTTP log server created in `dev-log-server/` crate
    - ✅ Trunk proxy configured to forward `/logs` requests to port 3001
    - ✅ Frontend log forwarding implemented via pre-build hook
    - ✅ Development utility setup completed

    ### How to start the development setup:
    Test logging: `curl -X POST 'http://localhost:8080/logs' -H 'Content-Type: application/json' -d '{"level":"info","message":"test"}'`

    ### What's left to implement:
    - [x] Create a simple HTTP log server using axum
        - [x] Add axum and tokio dependencies for the log server
        - [x] Create a basic axum server that listens on a configurable port (e.g., 3001)
        - [x] Add a POST endpoint (e.g., `/logs`) to receive log messages from frontend
        - [x] Parse incoming JSON log data and format for terminal display
        - [x] Add CORS headers to allow requests from trunk's dev server
        - [x] Add proper error handling and graceful shutdown
    - [x] Configure Trunk to proxy log requests to the server
        - [x] Update Trunk.toml or use CLI args to proxy `/logs` path to log server
        - [x] Test that frontend can successfully send requests to `/logs` endpoint
        - [x] Verify that trunk serve and log server can run on different ports simultaneously
    - [x] Create development utility setup using pre-build hooks
        - [x] Use trunk pre-build hooks to generate dynamic content
        - [x] Generate `debugutils.js` with log forwarding for debug builds
        - [x] Add `<link data-trunk rel="inline" href="debugutils.js"/>` to index.html template
        - [x] Ensure debug utilities are only active in development builds
        - [x] Keep existing generate-index.sh and build-audio-worklet.sh hooks
        - [x] Update documentation for the simplified development process
    - [x] Add frontend log forwarding functionality (debug mode only) - via JavaScript console interception
        - [x] Create JavaScript code to intercept console methods (log, warn, error, debug, info)
        - [x] Add logic to detect development vs production mode (check for localhost or dev server)
        - [x] Implement HTTP POST to /logs endpoint from JavaScript
        - [x] Add batching/throttling to avoid excessive requests
        - [x] Ensure original console methods still work (preserve existing behavior)
        - [x] Add error handling for when log server is unavailable
        - [x] Only include the log forwarding code in debug/development builds
    - [x] Remove `test.sh` and anything referring to it. we no longer need it.
    - [x] Update docs and instructions to refer to the `cargo check-wasm` aliases.
    - [x] Create an xtask that starts both dev-log-server and `trunk serve`
        - [x] make sure the xtask has aliases set up
        - [x] Update readme with instruction for how to use this
    - [x] Make the dev-log-server write to some log file
        - [x] make sure that file is gitignored
        - [x] Add some code to the forwarding code in js that indicates that a new frontend session has started
    - [x] Make sure the log forwarding code in js is started as soon as possible when the page loads
    - [x] Add command to xtask that dumps the log file and tell the agent how to use that
        - [x] Add a new command to the `xtask` crate to dump the log file
        - [x] Make the new command only dump the latest session from the log file
        - [x] Update the `DEVELOPMENT.md` file with instructions on how to use the new command
    - [x] clean up the log output format
    - [x] dev-log-server should output its logs to tmp/dev-log-server.log and to no other file
    - [x] the xtask dump-logs should read from tmp/dev-log-server.log only
    - [x] rename the dump-logs command to dump-latest-logs to indicate that we only dump the latest session
    - [x] Move any instructions you added to DEVELOPMENT.md to copilot-instructions.md, (or README.md if it is meant for the user). And then remove DEVELOPMENT.md
    - [x] The dump-latest-logs command is meant for agent consumption.
        - [x] Don't add colors
        - [x] Remove anything that isn't directly useful for the agent. we want to conserve context as much as possible. for example `dev_log_server` is not interesting information on every line.
    - [x] make sure the release minification actually strips out the log forwarding code as dead code elimination
    - [x] Change the "New session started" marker to something more unique.
        - Changed to `=== DISSONANCE_LAB_SESSION_START ===` to avoid conflicts with regular log messages
    - [x] Remove the timestamp, module_path and target from the log format
        - Simplified log format in dev-log-server to only include log level and message
        - Removed unused chrono dependency and cleaned up LogMessage struct
        - Updated xtask clean_log_line function to work with the new simplified format
    - [ ] Make the xtask alias not output any of the cargo stuff, only the output from the actual command
    - [ ] dev_log_server does not seem to output debug logs, it should
    - [ ] The current dev_log_server output is something like:
      ```
      2025-07-29T18:14:55.874837Z  INFO dev_log_server: %cDEBUG%c src/webaudio.rs:40 %c
      Loading audio worklet from: ./dissonance_worklet_processor.js color: white; padding: 0 3px; background: blue; font-weight: bold; color: inherit background: inherit; color: inherit
      ```
      It should be:
      ```
      2025-07-29T18:14:55.874837Z  DEBUG dev_log_server: src/webaudio.rs:40: Loading audio worklet from: ./dissonance_worklet_processor.js color: white; padding: 0 3px; background: blue; font-weight: bold; color: inherit background: inherit; color: inherit
      ```
- [ ] piano_gui.rs: handle multi touch? is it possible to do it since this is just a single widget?
  - [ ] Research egui's MultiTouchInfo API and how to access it in the current context
    - [x] Study egui::InputState and egui::MultiTouchInfo documentation
      - Found: input.multi_touch() returns Option<MultiTouchInfo> for gestures (zoom, rotation)
      - Found: input.any_touches() returns bool for active touches
      - Found: input.has_touch_screen() returns bool for touch capability
      - Found: Event::Touch with device_id, id (TouchId), phase, pos, force for individual touches
      - Key insight: MultiTouchInfo is for gestures, but Event::Touch is for individual finger tracking
    - [x] Check if ui.input() provides access to multi-touch data
      - Yes: ui.input(|i| i.multi_touch()) for gestures
      - Yes: ui.input(|i| i.events) contains Event::Touch events for individual touches
    - [x] Investigate if egui::Sense needs to be configured differently for multi-touch
      - No: Sense only defines interaction types (HOVER, CLICK, DRAG, FOCUSABLE)
      - Multi-touch is handled through Event system, not Sense configuration
    - [x] Look at egui examples or source code for multi-touch handling patterns
      - Key finding: Need to process Event::Touch events in input.events
      - Strategy: Track TouchId -> Key mapping for individual finger tracking
      - Current issue: ui.interact() and is_pointer_button_down_on() are single-pointer
      - Solution: Process touch events directly, bypass single-pointer Response methods
  - [ ] Analyze current single-touch implementation to understand what needs to change
    - [ ] Review how is_pointer_button_down_on() works with multiple pointers
    - [ ] Understand the key_id and temp data storage mechanism
    - [ ] Document the current state tracking for mouse_pressed per key
  - [ ] Design multi-touch data structures
    - [ ] Define how to track multiple pointer IDs per key
    - [ ] Decide on data structure to map pointer IDs to pressed keys
    - [ ] Plan how to handle pointer lifecycle (press, hold, release)
  - [ ] Implement pointer tracking to handle multiple simultaneous touches
    - [ ] Replace single boolean mouse_pressed with multi-pointer tracking
    - [ ] Update key press detection to handle multiple active pointers
    - [ ] Implement pointer release detection for multi-touch
    - [ ] Handle edge cases like pointer leaving key area during touch
  - [ ] Test multi-touch functionality on mobile devices and touch screens
    - [ ] Test basic two-finger simultaneous key presses
    - [ ] Test chord playing with multiple fingers
    - [ ] Verify touch responsiveness and accuracy
    - [ ] Test edge cases like sliding fingers between keys
  - [ ] Ensure multi-touch doesn't break existing mouse and single-touch interactions
    - [ ] Verify mouse clicks still work as expected
    - [ ] Test single-touch on mobile devices
    - [ ] Ensure keyboard shortcuts (shift+click) still work
    - [ ] Test mixed input scenarios (mouse + touch simultaneously)
- [ ] Try increasing the reverb to hear how it sounds
- [ ] Could the midi input callback be moved out of the rust code to make it lower latency?
- [ ] Change the order of the interval displays so the bottom row shows the first pressed note when using the mouse, and the actual base when using a midi keyboard.
  - [ ] The `KeySet` type needs to keep track of the order of the keys
  - [ ] Modify PianoGui to track the chronological order of mouse key presses
  - [ ] Define what "actual base" means for MIDI keyboard input (e.g., lowest note, root note, etc.) Just assume that it is the lowest note for now.
  - [ ] Add input source tracking to distinguish between mouse and MIDI input for each pressed key. If the input modes are mixed, just do some best effort solution. No need to spend much code for this case.
  - [ ] Modify interval_display.rs to use different ordering logic based on input method
  - [ ] Update the pressed_keys data structure to include ordering/priority information
  - [ ] Test the new ordering behavior with both mouse and MIDI input
- [ ] model piano string stiffness inharmonicity
- [ ] go through the codebase looking for comments that say what has been changed. as is typical of coding agents. remove those as they are not useful longterm
- [ ] Calculate dissonances using critical bands theory instead.
    - This would allow us to calculate the dissonance of entire chords
    - how do we handle the fact that we only show a single octave? just force the calculation to happen on a single central octave?
    - can critical bands theory be made octave normalized?
    - does critical bands theory care about the root? do we need to know which note is the root? can the overtones be extended downwards?
- [ ] The dissonance of the currently held notes should show somewhere prominent
- [ ] We only need one row of dissonances that shows what dissonance a new note would result in.
    - for the second note we show the same as we currently do
    - for more notes we show what chord they would result in
- [ ] Make `shift` behave like a sustain pedal. We want almost infinite sustain to allow the user to hear chord dissonances.
