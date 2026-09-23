//! A separate, owned desktop child window; the user's wallpaper file is never changed.
use crate::render::Mode;
use anyhow::{Result, bail};

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Backend {
    Auto,
    Plasma,
    Mpvpaper,
    X11,
    Gnome,
}

#[cfg(any(target_os = "linux", test))]
fn select_backend(requested: Backend, desktop: &str, wayland: bool, x11: bool) -> Result<Backend> {
    anyhow::ensure!(
        wayland || x11,
        "Не найден графический сеанс Linux (WAYLAND_DISPLAY / DISPLAY)"
    );
    if requested != Backend::Auto {
        anyhow::ensure!(
            requested != Backend::X11 || !wayland,
            "X11 backend нельзя использовать через XWayland; выберите plasma, gnome или mpvpaper"
        );
        anyhow::ensure!(
            requested != Backend::Mpvpaper || wayland,
            "mpvpaper требует сеанс Wayland с layer-shell"
        );
        return Ok(requested);
    }
    let desktop = desktop.to_ascii_lowercase();
    Ok(if desktop.contains("kde") || desktop.contains("plasma") {
        Backend::Plasma
    } else if wayland && desktop.contains("gnome") {
        Backend::Gnome
    } else if wayland {
        Backend::Mpvpaper
    } else {
        Backend::X11
    })
}

#[cfg(any(target_os = "linux", test))]
mod frame;
#[cfg(any(target_os = "linux", test))]
mod stream;

#[cfg(test)]
mod selection_tests {
    use super::*;
    #[test]
    fn desktop_protocol_and_override_select_the_right_backend() {
        assert_eq!(
            select_backend(Backend::Auto, "KDE", true, true).unwrap(),
            Backend::Plasma
        );
        assert_eq!(
            select_backend(Backend::Auto, "ubuntu:GNOME", true, true).unwrap(),
            Backend::Gnome
        );
        assert_eq!(
            select_backend(Backend::Auto, "Hyprland", true, true).unwrap(),
            Backend::Mpvpaper
        );
        assert_eq!(
            select_backend(Backend::Auto, "XFCE", false, true).unwrap(),
            Backend::X11
        );
        assert_eq!(
            select_backend(Backend::X11, "KDE", false, true).unwrap(),
            Backend::X11
        );
        assert!(select_backend(Backend::Auto, "", false, false).is_err());
        assert!(select_backend(Backend::X11, "GNOME", true, true).is_err());
        assert!(select_backend(Backend::Mpvpaper, "", false, true).is_err());
    }
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::Wallpaper;

pub fn open(args: &crate::Args) -> Result<Wallpaper> {
    #[cfg(target_os = "linux")]
    {
        linux::Wallpaper::open(args)
    }
    #[cfg(not(target_os = "linux"))]
    {
        anyhow::ensure!(
            args.wallpaper_backend == Backend::Auto,
            "Этот backend обоев доступен только в Linux; используйте auto"
        );
        Wallpaper::open()
    }
}

pub fn configure(args: &crate::Args) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        linux::configure(args)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = args;
        bail!("Установка интеграции требуется только в Linux")
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
pub struct Wallpaper;
#[cfg(not(any(windows, target_os = "linux")))]
impl Wallpaper {
    pub fn open() -> Result<Self> {
        bail!(
            "Живые обои пока поддерживаются только в Windows Explorer; Linux X11/Wayland требуют отдельной интеграции"
        )
    }
    pub fn draw(&mut self, _: &[u8], _: (usize, usize), _: Mode, _: bool) -> Result<()> {
        Ok(())
    }
}

#[cfg(windows)]
mod desktop {
    use super::*;
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        UI::{HiDpi::*, WindowsAndMessaging::*},
    };

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain([0]).collect()
    }

    unsafe extern "system" fn find_worker(window: HWND, output: LPARAM) -> i32 {
        unsafe {
            if !FindWindowExW(
                window,
                null_mut(),
                wide("SHELLDLL_DefView").as_ptr(),
                null(),
            )
            .is_null()
            {
                let next = FindWindowExW(null_mut(), window, wide("WorkerW").as_ptr(), null());
                if !next.is_null() {
                    *(output as *mut HWND) = next;
                    return 0;
                }
            }
        }
        1
    }

    pub struct Wallpaper {
        window: HWND,
        parent: HWND,
        dc: HDC,
        bitmap: HBITMAP,
        old_bitmap: HGDIOBJ,
        size: (i32, i32),
        pixels: Vec<u8>,
    }

    impl Wallpaper {
        pub fn open() -> Result<Self> {
            unsafe {
                let progman = FindWindowW(wide("Progman").as_ptr(), null());
                anyhow::ensure!(
                    !progman.is_null(),
                    "Не найден рабочий стол Windows Explorer"
                );
                let mut result = 0;
                // Explorer's wallpaper host is undocumented; fail visibly if its layout differs.
                for (w, l) in [(0xD, 0), (0xD, 1), (0, 0)] {
                    SendMessageTimeoutW(progman, 0x052C, w, l, SMTO_ABORTIFHUNG, 1000, &mut result);
                }
                let mut parent =
                    FindWindowExW(progman, null_mut(), wide("WorkerW").as_ptr(), null());
                if parent.is_null() {
                    EnumWindows(Some(find_worker), (&mut parent as *mut HWND) as LPARAM);
                }
                anyhow::ensure!(
                    !parent.is_null(),
                    "Explorer не предоставил слой обоев. Обычный режим доступен без --wallpaper"
                );
                let previous_dpi =
                    SetThreadDpiAwarenessContext(GetWindowDpiAwarenessContext(parent));
                let size = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
                let mut origin = POINT { x: 0, y: 0 };
                MapWindowPoints(null_mut(), parent, &mut origin, 1);
                if size.0 <= 0 || size.1 <= 0 {
                    SetThreadDpiAwarenessContext(previous_dpi);
                    bail!("Слой обоев имеет нулевой размер");
                }
                let window = CreateWindowExW(
                    WS_EX_NOACTIVATE | WS_EX_TRANSPARENT,
                    wide("STATIC").as_ptr(),
                    wide("ASIJI Wallpaper").as_ptr(),
                    WS_CHILD | WS_VISIBLE | WS_DISABLED,
                    origin.x,
                    origin.y,
                    size.0,
                    size.1,
                    parent,
                    null_mut(),
                    null_mut(),
                    null(),
                );
                SetThreadDpiAwarenessContext(previous_dpi);
                anyhow::ensure!(
                    !window.is_null(),
                    "Не удалось создать окно обоев: {}",
                    std::io::Error::last_os_error()
                );
                let screen = GetDC(window);
                if screen.is_null() {
                    DestroyWindow(window);
                    bail!("Не удалось получить контекст окна обоев");
                }
                let dc = CreateCompatibleDC(screen);
                let bitmap = CreateCompatibleBitmap(screen, size.0, size.1);
                ReleaseDC(window, screen);
                if dc.is_null() || bitmap.is_null() {
                    if !dc.is_null() {
                        DeleteDC(dc);
                    }
                    if !bitmap.is_null() {
                        DeleteObject(bitmap);
                    }
                    DestroyWindow(window);
                    bail!("Не удалось создать буфер обоев");
                }
                let old_bitmap = SelectObject(dc, bitmap);
                Ok(Self {
                    window,
                    parent,
                    dc,
                    bitmap,
                    old_bitmap,
                    size,
                    pixels: Vec::new(),
                })
            }
        }

        pub fn draw(
            &mut self,
            rgb: &[u8],
            dimensions: (usize, usize),
            mode: Mode,
            color: bool,
        ) -> Result<()> {
            let (columns, rows) = dimensions;
            let pixel_rows = rows * if mode == Mode::Blocks { 2 } else { 1 };
            anyhow::ensure!(
                columns > 0 && rows > 0 && rgb.len() == columns * pixel_rows * 3,
                "Неверный размер кадра обоев"
            );
            unsafe {
                anyhow::ensure!(
                    IsWindow(self.window) != 0 && IsWindow(self.parent) != 0,
                    "Explorer перезапустился; включите трек заново для восстановления обоев"
                );
                let mut msg = MSG::default();
                while PeekMessageW(&mut msg, self.window, 0, 0, PM_REMOVE) != 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                let scale = (self.size.0 as f64 / columns as f64)
                    .min(self.size.1 as f64 / (rows * 2) as f64);
                let width = (columns as f64 * scale) as i32;
                let height = (rows as f64 * scale * 2.0) as i32;
                let x = (self.size.0 - width) / 2;
                let y = (self.size.1 - height) / 2;
                PatBlt(self.dc, 0, 0, self.size.0, self.size.1, BLACKNESS);
                if mode == Mode::Blocks {
                    self.pixels.clear();
                    for p in rgb.chunks_exact(3) {
                        let gray =
                            ((77 * p[0] as u32 + 150 * p[1] as u32 + 29 * p[2] as u32) >> 8) as u8;
                        let pixel = if color {
                            [p[2], p[1], p[0], 0]
                        } else {
                            [gray, gray, gray, 0]
                        };
                        self.pixels.extend_from_slice(&pixel);
                    }
                    let mut info = BITMAPINFO::default();
                    info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                    info.bmiHeader.biWidth = columns as i32;
                    info.bmiHeader.biHeight = -(rows as i32 * 2);
                    info.bmiHeader.biPlanes = 1;
                    info.bmiHeader.biBitCount = 32;
                    SetStretchBltMode(self.dc, COLORONCOLOR);
                    StretchDIBits(
                        self.dc,
                        x,
                        y,
                        width,
                        height,
                        0,
                        0,
                        columns as i32,
                        rows as i32 * 2,
                        self.pixels.as_ptr().cast(),
                        &info,
                        DIB_RGB_COLORS,
                        SRCCOPY,
                    );
                } else {
                    let font = CreateFontW(
                        (scale * 2.0).ceil() as i32,
                        scale.ceil() as i32,
                        0,
                        0,
                        400,
                        0,
                        0,
                        0,
                        DEFAULT_CHARSET as u32,
                        0,
                        0,
                        NONANTIALIASED_QUALITY as u32,
                        FIXED_PITCH as u32,
                        wide("Consolas").as_ptr(),
                    );
                    anyhow::ensure!(!font.is_null(), "Не удалось создать шрифт обоев");
                    let old_font = SelectObject(self.dc, font);
                    SetBkMode(self.dc, TRANSPARENT as i32);
                    let ramp = b" .,:;irsXA253hMHGS#9B&@";
                    for row in 0..rows {
                        for column in 0..columns {
                            let p = &rgb[(row * columns + column) * 3..][..3];
                            let gray = ((77 * p[0] as u32 + 150 * p[1] as u32 + 29 * p[2] as u32)
                                >> 8) as u8;
                            let ch = ramp[((gray as f32 / 255.0).powf(0.85)
                                * (ramp.len() - 1) as f32)
                                as usize] as u16;
                            if ch == 32 {
                                continue;
                            }
                            let rgb = if color {
                                p[0] as u32 | (p[1] as u32) << 8 | (p[2] as u32) << 16
                            } else {
                                gray as u32 * 0x010101
                            };
                            SetTextColor(self.dc, rgb);
                            TextOutW(
                                self.dc,
                                x + (column as f64 * scale) as i32,
                                y + (row as f64 * scale * 2.0) as i32,
                                &ch,
                                1,
                            );
                        }
                    }
                    SelectObject(self.dc, old_font);
                    DeleteObject(font);
                }
                let screen = GetDC(self.window);
                let copied = BitBlt(
                    screen,
                    0,
                    0,
                    self.size.0,
                    self.size.1,
                    self.dc,
                    0,
                    0,
                    SRCCOPY,
                );
                ReleaseDC(self.window, screen);
                anyhow::ensure!(copied != 0, "Не удалось нарисовать обои");
            }
            Ok(())
        }
    }

    impl Drop for Wallpaper {
        fn drop(&mut self) {
            unsafe {
                SelectObject(self.dc, self.old_bitmap);
                DeleteObject(self.bitmap);
                DeleteDC(self.dc);
                if IsWindow(self.window) != 0 {
                    DestroyWindow(self.window);
                }
                InvalidateRect(self.parent, null(), 1);
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        #[ignore = "requires an interactive Windows Explorer desktop"]
        fn desktop_buffer_renders_and_window_is_removed() -> Result<()> {
            let mut wallpaper = Wallpaper::open()?;
            let window = wallpaper.window;
            let rgb = vec![180_u8; 80 * 44 * 3];
            wallpaper.draw(&rgb, (80, 22), Mode::Blocks, true)?;
            unsafe {
                assert_eq!(
                    GetPixel(wallpaper.dc, wallpaper.size.0 / 2, wallpaper.size.1 / 2),
                    0x00b4b4b4
                );
                assert_eq!(GetParent(window), wallpaper.parent);
            }
            wallpaper.draw(&rgb[..80 * 22 * 3], (80, 22), Mode::Ascii, true)?;
            let mut colored = 0;
            unsafe {
                for y in (0..wallpaper.size.1).step_by(7) {
                    for x in (0..wallpaper.size.0).step_by(7) {
                        if GetPixel(wallpaper.dc, x, y) == 0x00b4b4b4 {
                            colored += 1;
                        }
                    }
                }
                assert!(colored > 100, "ASCII wallpaper has no glyph pixels");
            }
            drop(wallpaper);
            unsafe {
                assert_eq!(IsWindow(window), 0);
            }
            Ok(())
        }
    }
}
#[cfg(windows)]
pub use desktop::Wallpaper;
