//! Text-only block flow. Windows COM interfaces own their references (RAII).
//! All methods run on the window's single UI thread, inside its COM apartment.

use crate::document::{Block, Document};
use windows::{
    core::{w, Result},
    Win32::{
        Foundation::{D2DERR_RECREATE_TARGET, HWND},
        Graphics::{
            Direct2D::{Common::*, *},
            DirectWrite::*,
        },
    },
};

struct DeviceResources {
    brush: ID2D1SolidColorBrush,
    target: ID2D1HwndRenderTarget,
}

pub struct Renderer {
    device: Option<DeviceResources>,
    heading: IDWriteTextFormat,
    paragraph: IDWriteTextFormat,
    write: IDWriteFactory,
    factory: ID2D1Factory,
}

impl Renderer {
    pub fn new() -> Result<Self> {
        // SAFETY: called on the initialized UI thread. All COM outputs are owned
        // by windows-rs; string literals are valid, NUL-terminated UTF-16.
        unsafe {
            let factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let write: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let heading = write.CreateTextFormat(w!("Segoe UI"), None, DWRITE_FONT_WEIGHT_BOLD,
                DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, 32.0, w!("pt-BR"))?;
            let paragraph = write.CreateTextFormat(w!("Segoe UI"), None, DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, 18.0, w!("pt-BR"))?;
            Ok(Self { device: None, heading, paragraph, write, factory })
        }
    }

    fn ensure_device(&mut self, hwnd: HWND, width: u32, height: u32) -> Result<()> {
        if self.device.is_none() {
            // SAFETY: hwnd is the live UI window; property pointers refer to
            // initialized stack values for the duration of these synchronous calls.
            unsafe {
                let target = self.factory.CreateHwndRenderTarget(
                    &D2D1_RENDER_TARGET_PROPERTIES { dpiX: 96.0, dpiY: 96.0, ..Default::default() },
                    &D2D1_HWND_RENDER_TARGET_PROPERTIES {
                        hwnd, pixelSize: D2D_SIZE_U { width, height }, ..Default::default()
                    },
                )?;
                let brush = target.CreateSolidColorBrush(
                    &D2D1_COLOR_F { r: 0.10, g: 0.12, b: 0.14, a: 1.0 }, None)?;
                self.device = Some(DeviceResources { brush, target });
            }
        }
        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        if width == 0 || height == 0 { return Ok(()); }
        if let Some(device) = &self.device {
            // SAFETY: the target is owned, alive, used on its creating thread;
            // the initialized size is nonzero and borrowed only for this call.
            if let Err(error) = unsafe { device.target.Resize(&D2D_SIZE_U { width, height }) } {
                if error.code() == D2DERR_RECREATE_TARGET { self.device = None; }
                else { return Err(error); }
            }
        }
        Ok(())
    }

    /// Returns true only when the target was lost and another paint is needed.
    pub fn paint(&mut self, hwnd: HWND, width: u32, height: u32, document: &Document) -> Result<bool> {
        if width == 0 || height == 0 { return Ok(false); }
        self.ensure_device(hwnd, width, height)?;
        let Some(device) = &self.device else { return Ok(false); };
        // SAFETY: the target/brush/formats are owned and thread-confined. BeginDraw
        // is paired with EndDraw even if text layout fails. UTF-16 buffers and all
        // rectangle/metric pointers remain live through their synchronous calls.
        let (draw, end) = unsafe {
            device.target.BeginDraw();
            device.target.Clear(Some(&D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }));
            let draw = (|| -> Result<()> {
                let size = device.target.GetSize();
                let available = (size.width - 48.0).max(1.0);
                let mut y = 24.0;
                for block in &document.blocks {
                    if y >= size.height { break; }
                    let format = match block { Block::Heading(_) => &self.heading, Block::Paragraph(_) => &self.paragraph };
                    let text: Vec<u16> = block.text().encode_utf16().collect();
                    let layout = self.write.CreateTextLayout(&text, format, available, 1_000_000.0)?;
                    let mut metrics = DWRITE_TEXT_METRICS::default();
                    layout.GetMetrics(&mut metrics)?;
                    let height = metrics.height.max(22.0);
                    device.target.DrawText(&text, format, &D2D_RECT_F {
                        left: 24.0, top: y, right: 24.0 + available, bottom: y + height + 1.0,
                    }, &device.brush, D2D1_DRAW_TEXT_OPTIONS_CLIP, DWRITE_MEASURING_MODE_NATURAL);
                    y += height + 16.0;
                }
                Ok(())
            })();
            (draw, device.target.EndDraw(None, None))
        };
        draw?;
        match end {
            Ok(()) => Ok(false),
            Err(error) if error.code() == D2DERR_RECREATE_TARGET => { self.device = None; Ok(true) },
            Err(error) => Err(error),
        }
    }
}
