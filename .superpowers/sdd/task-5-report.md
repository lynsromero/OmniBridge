## Task 5: Add Frame Receive + Decode + Display to Client

**Status:** DONE

**Commits:**
- `917ea00` feat(client): add frame receive, decode, and overlay display

**Test Summary:** `cargo check` passed with expected warnings (unused `decoder` variable, to be wired in Task 6).

**Changes:**
- Added `VideoDecoder` import and instance
- Added `OverlayWindow` import and optional instance
- Added overlay creation logic when peers connect
- Added Ctrl+C cleanup to destroy overlay
- Message channel created for future UDP receive wiring

**Concerns:** None. All warnings are expected and will be resolved when Task 6 wires up the UDP receive path and decoder usage.