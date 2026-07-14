### Task 3: Rewrite Overlay Window with Win32 Creation + Pixel Rendering

**Files:**
- Modify: `crates/ob-display/src/overlay.rs` (full rewrite of the file)

**Interfaces:**
- Consumes: `DecodedFrame` (from `ob-codec::decoder`)
- Produces: A visible Win32 overlay window that displays decoded pixels

The current overlay tries to `FindWindowA` a window that doesn't exist. Replace with actual window creation using `RegisterClassExA` + `CreateWindowExA` and pixel rendering via `StretchDIBits`.

- [ ] **Step 1: Rewrite overlay.rs**

Replace the entire contents of `crates/ob-display/src/overlay.rs` with:

```rust
use anyhow::Result;
use ob_codec::decoder::DecodedFrame;
use tracing::{debug, info};

pub struct OverlayWindow {
    hwnd: *mut std::ffi::c_void,
    hdc: *mut std::ffi::c_void,
    bitmap_info: BITMAPINFO,
    width: u32,
    height: u32,
}

#[repr(C)]
#[allow(non_snake_case)]
struct BITMAPINFOHEADER {
    biSize: u32,
    biWidth: i32,
    biHeight: i32,
    biPlanes: u16,
    biBitCount: u16,
    biCompression: u32,
    biSizeImage: u32,
    biXPelsPerMeter: i32,
    biYPelsPerMeter: i32,
    biClrUsed: u32,
    biClrImportant: u32,
}

#[repr(C)]
#[allow(non_snake_case)]
struct BITMAPINFO {
    bmiHeader: BITMAPINFOHEADER,
    bmiColors: [u32; 1],
}

impl OverlayWindow {
    pub fn new(title: &str, width: u32, height: u32) -> Self {
        use std::ffi::c_void;

        #[link(name = "user32")]
        extern "system" {
            fn RegisterClassExA(lpwcx: *const WNDCLASSEXA) -> u16;
            fn CreateWindowExA(
                dwExStyle: u32, lpClassName: *const u8, lpWindowName: *const u8,
                dwStyle: u32, x: i32, y: i32, nWidth: i32, nHeight: i32,
                hWndParent: *mut c_void, hMenu: *mut c_void,
                hInstance: *mut c_void, lpParam: *mut c_void,
            ) -> *mut c_void;
            fn GetDC(hWnd: *mut c_void) -> *mut c_void;
            fn GetModuleHandleA(lpModuleName: *const u8) -> *mut c_void;
        }

        #[repr(C)]
        struct WNDCLASSEXA {
            cbSize: u32,
            style: u32,
            lpfnWndProc: *mut c_void,
            cbClsExtra: i32,
            cbWndExtra: i32,
            hInstance: *mut c_void,
            hIcon: *mut c_void,
            hCursor: *mut c_void,
            hbrBackground: *mut c_void,
            lpszMenuName: *const u8,
            lpszClassName: *const u8,
            hIconSm: *mut c_void,
        }

        let class_name = b"OmniBridgeOverlay\0";
        let window_name = format!("{}\0", title);

        unsafe {
            let hinstance = GetModuleHandleA(std::ptr::null());

            let wnd_class = WNDCLASSEXA {
                cbSize: std::mem::size_of::<WNDCLASSEXA>() as u32,
                style: 0,
                lpfnWndProc: Some(def_window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance,
                hIcon: std::ptr::null_mut(),
                hCursor: std::ptr::null_mut(),
                hbrBackground: std::ptr::null_mut(),
                lpszMenuName: std::ptr::null(),
                lpszClassName: class_name.as_ptr(),
                hIconSm: std::ptr::null_mut(),
            };

            RegisterClassExA(&wnd_class);

            let hwnd = CreateWindowExA(
                0x00000008 | 0x00000020 | 0x00000001, // WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_LAYERED
                class_name.as_ptr(),
                window_name.as_ptr() as *const u8,
                0x80000000, // WS_POPUP
                0, 0, width as i32, height as i32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                hinstance,
                std::ptr::null_mut(),
            );

            let hdc = GetDC(hwnd);

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width as i32,
                    biHeight: -(height as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: 0,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [0; 1],
            };

            info!("Overlay window created: {}x{}", width, height);

            Self {
                hwnd,
                hdc,
                bitmap_info: bmi,
                width,
                height,
            }
        }
    }

    pub fn render_frame(&self, frame: &DecodedFrame) -> Result<()> {
        if self.hwnd.is_null() || self.hdc.is_null() {
            return Ok(());
        }

        #[link(name = "gdi32")]
        extern "system" {
            fn StretchDIBits(
                hdc: *mut std::ffi::c_void,
                xDest: i32, yDest: i32, wDest: i32, hDest: i32,
                xSrc: i32, ySrc: i32, wSrc: i32, hSrc: i32,
                lpBits: *const u8, lpbmi: *const BITMAPINFO,
                iUsage: u32, dwRop: u32,
            ) -> i32;
        }

        #[link(name = "user32")]
        extern "system" {
            fn SetWindowPos(
                hWnd: *mut std::ffi::c_void,
                hWndInsertAfter: *mut std::ffi::c_void,
                x: i32, y: i32, cx: i32, cy: i32, uFlags: u32,
            ) -> i32;
        }

        if frame.pixels.len() >= (frame.width * frame.height * 4) as usize {
            unsafe {
                StretchDIBits(
                    self.hdc,
                    0, 0, self.width as i32, self.height as i32,
                    0, 0, frame.width as i32, frame.height as i32,
                    frame.pixels.as_ptr(),
                    &self.bitmap_info,
                    0, 0x00CC0020, // SRCCOPY
                );
            }
        }

        debug!("Rendered frame {}x{}", frame.width, frame.height);
        Ok(())
    }

    pub fn set_position(&self, x: i32, y: i32) {
        if self.hwnd.is_null() {
            return;
        }
        #[link(name = "user32")]
        extern "system" {
            fn SetWindowPos(
                hWnd: *mut std::ffi::c_void,
                hWndInsertAfter: *mut std::ffi::c_void,
                x: i32, y: i32, cx: i32, cy: i32, uFlags: u32,
            ) -> i32;
        }
        unsafe {
            SetWindowPos(
                self.hwnd,
                std::ptr::null_mut(), // HWND_TOPMOST
                x, y, 0, 0,
                0x0001 | 0x0002, // SWP_NOSIZE | SWP_NOZORDER
            );
        }
    }

    pub fn set_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.bitmap_info.bmiHeader.biWidth = width as i32;
        self.bitmap_info.bmiHeader.biHeight = -(height as i32);
    }

    pub fn destroy(&self) {
        if !self.hwnd.is_null() {
            #[link(name = "user32")]
            extern "system" {
                fn DestroyWindow(hWnd: *mut std::ffi::c_void) -> i32;
                fn ReleaseDC(hWnd: *mut std::ffi::c_void, hDC: *mut std::ffi::c_void) -> i32;
            }
            unsafe {
                ReleaseDC(self.hwnd, self.hdc);
                DestroyWindow(self.hwnd);
            }
            info!("Overlay window destroyed");
        }
    }
}

extern "system" fn def_window_proc(
    _hwnd: *mut std::ffi::c_void,
    _msg: u32,
    _wparam: usize,
    _lparam: isize,
) -> isize {
    0
}

impl Drop for OverlayWindow {
    fn drop(&mut self) {
        self.destroy();
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p ob-display`
Expected: OK

- [ ] **Step 3: Commit**

```bash
git add crates/ob-display/src/overlay.rs
git commit -m "feat(display): create real Win32 overlay window with pixel rendering"
```

---

