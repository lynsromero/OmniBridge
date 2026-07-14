### Task 7: Build Release and End-to-End Test

**Files:**
- No code changes. Verification only.

- [ ] **Step 1: Build release binary**

Run:
```bash
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo build --release
```
Expected: Build succeeds, binary at `target/release/omnibridge.exe`

- [ ] **Step 2: Test help output**

Run: `target\release\omnibridge.exe --help`
Expected: Shows CLI usage with start, connect, status, layout, config subcommands

- [ ] **Step 3: Test primary node startup (Terminal 1)**

Run: `target\release\omnibridge.exe start --name "MainPC" --primary --port 19810`
Expected: Prints "Server started", "Input capture active", "Video streaming started: WxH"

- [ ] **Step 4: Test secondary node (Terminal 2)**

Run: `target\release\omnibridge.exe start --name "Laptop" --port 19810`
Expected: Prints "Client started", then after discovery: "Discovered server", "Connected to server", "Overlay window created - receiving video"

- [ ] **Step 5: Test standalone connect mode**

Run: `target\release\omnibridge.exe connect --address 127.0.0.1 --port 19810`
Expected: Prints "Connected" without errors

- [ ] **Step 6: Test status command**

Run: `target\release\omnibridge.exe status`
Expected: Prints config directory and layout config status

- [ ] **Step 7: Commit final state**

```bash
git add -A
git commit -m "feat: complete end-to-end pipeline wiring

- Screen capture streams from primary to secondary at 30fps
- Overlay window created on secondary with real-time pixel rendering
- Input events forwarded from secondary back to primary
- Frame decode/encode uses raw BGRA pass-through (stub codec)"
```
