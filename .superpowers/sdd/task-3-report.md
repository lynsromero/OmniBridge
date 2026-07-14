**Status:** DONE_WITH_CONCERNS

**Commits created:**
- `7ff86e5` - feat(display): create real Win32 overlay window with pixel rendering

**Test summary:** Cargo check passed with warnings (unused `SetWindowPos` function, snake_case naming). No compilation errors.

**Concerns:**
- The provided code in the brief had a type mismatch: `lpfnWndProc: Some(def_window_proc)` where the field expects a raw pointer `*mut c_void`. Fixed by casting the function pointer to `*mut std::ffi::c_void`.
- The brief's code declares `SetWindowPos` inside `render_frame` but never calls it; this results in an unused function warning.
- The brief's code does not include `#[allow(non_snake_case)]` for the inner `WNDCLASSEXA` struct, causing 10 warnings about snake_case field names. This is cosmetic and non‑breaking.
- The brief's code removes several public methods (`set_visible`, `set_alpha`, `update_from_window`, `position`, `size`, `id`) and the `source_device` field. The API change is safe per the brief (no other usage in the codebase), but any external consumers not listed in the grep results would break.