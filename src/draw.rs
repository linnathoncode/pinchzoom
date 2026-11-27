use std::ffi::c_void;
use std::ptr;

use crate::win32::*;
use crate::hotkeys::{format_reset_summary, MOD_ALT, MOD_CTRL, MOD_SHIFT, MOD_WIN};
use crate::state::UiState;

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: u32,
    pub sub: u32,
    hover: u32,
    pub line: u32,
    strong: u32,
    pub ink: u32,
    muted: u32,
    cta: u32,
    accent: u32,
    on_cta: u32,
    error: u32,
    signal: u32,
    dark: bool,
}

impl Palette {
    pub fn new(dark: bool) -> Self {
        if dark {
            Self {
                bg: 0xFF131518,
                sub: 0xFF191C20,
                hover: 0xFF22262C,
                line: 0xFF252930,
                strong: 0xFF343A44,
                ink: 0xFFF0EFEA,
                muted: 0xFF9EA3AD,
                cta: 0xFF2E4A8F,
                accent: 0xFF4B72C4,
                on_cta: 0xFFF6F7F8,
                error: 0xFFDF6B62,
                signal: 0xFFFFC391,
                dark,
            }
        } else {
            Self {
                bg: 0xFFFCFBF9,
                sub: 0xFFF4F2EC,
                hover: 0xFFEAE7DF,
                line: 0xFFE5E1D8,
                strong: 0xFFD4CFC4,
                ink: 0xFF181A1F,
                muted: 0xFF515661,
                cta: 0xFF3757A6,
                accent: 0xFF3757A6,
                on_cta: 0xFFFCFBF9,
                error: 0xFFB8423A,
                signal: 0xFF87502F,
                dark,
            }
        }
    }

    fn button(self, strength: f32) -> u32 {
        mix(
            self.cta,
            if self.dark { self.accent } else { self.ink },
            strength,
        )
    }
}

pub const CLIENT_W: f32 = 388.0;
pub const CLIENT_H: f32 = 471.0;
const SETTINGS_INSET: f32 = 12.0;
const BUTTON_RADIUS: f32 = 9.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Shortcut,
    Action(i32),
    Preset(i32),
    Follow,
    Startup,
    Reset,
    Done,
}

impl Control {
    pub const ORDER: [Self; 9] = [
        Self::Shortcut,
        Self::Action(0),
        Self::Action(1),
        Self::Preset(0),
        Self::Preset(1),
        Self::Follow,
        Self::Startup,
        Self::Reset,
        Self::Done,
    ];
}

pub fn mix(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let aa = ((a >> 24) & 0xFF) as f32;
    let ar = ((a >> 16) & 0xFF) as f32;
    let ag = ((a >> 8) & 0xFF) as f32;
    let ab = (a & 0xFF) as f32;
    let ba = ((b >> 24) & 0xFF) as f32;
    let br = ((b >> 16) & 0xFF) as f32;
    let bg = ((b >> 8) & 0xFF) as f32;
    let bb = (b & 0xFF) as f32;
    let oa = (aa + (ba - aa) * t).round() as u32;
    let or = (ar + (br - ar) * t).round() as u32;
    let og = (ag + (bg - ag) * t).round() as u32;
    let ob = (ab + (bb - ab) * t).round() as u32;
    (oa << 24) | (or << 16) | (og << 8) | ob
}

pub fn with_alpha(color: u32, alpha: u8) -> u32 {
    ((alpha as u32) << 24) | (color & 0x00FF_FFFF)
}

pub struct Layout {
    pub s: f32,
    pub key_stage: RectF,
    pub cap_action: RectF,
    pub seg_action: RectF,
    pub cap_preset: RectF,
    pub seg_preset: RectF,
    pub divider_y: f32,
    pub follow_row: RectF,
    pub startup_row: RectF,
    pub reset_link: RectF,
    pub done_btn: RectF,
}

impl Layout {
    pub fn new(s: f32) -> Self {
        let m = 20.0 * s;
        let w = CLIENT_W * s - 2.0 * m;
        Self {
            s,
            key_stage: RectF {
                x: m,
                y: 16.0 * s,
                w,
                h: 134.0 * s,
            },
            cap_action: RectF {
                x: m,
                y: 163.0 * s,
                w,
                h: 22.0 * s,
            },
            seg_action: RectF {
                x: m,
                y: 188.0 * s,
                w,
                h: 39.0 * s,
            },
            cap_preset: RectF {
                x: m,
                y: 238.0 * s,
                w,
                h: 22.0 * s,
            },
            seg_preset: RectF {
                x: m,
                y: 263.0 * s,
                w,
                h: 39.0 * s,
            },
            divider_y: 319.0 * s,
            follow_row: RectF {
                x: m,
                y: 328.0 * s,
                w,
                h: 37.0 * s,
            },
            startup_row: RectF {
                x: m,
                y: 369.0 * s,
                w,
                h: 37.0 * s,
            },
            reset_link: RectF {
                x: m,
                y: 420.0 * s,
                w: 140.0 * s,
                h: 36.0 * s,
            },
            done_btn: RectF {
                x: m + w - 100.0 * s,
                y: 420.0 * s,
                w: 100.0 * s,
                h: 36.0 * s,
            },
        }
    }

    pub fn control_rect(&self, control: Control) -> RectF {
        match control {
            Control::Shortcut => self.key_stage,
            Control::Action(index) => self.segment(self.seg_action, index as f32),
            Control::Preset(index) => self.segment(self.seg_preset, index as f32),
            Control::Follow => self.follow_row,
            Control::Startup => self.startup_row,
            Control::Reset => self.reset_link,
            Control::Done => self.done_btn,
        }
    }

    fn button_rect(&self, control: Control) -> RectF {
        let bounds = self.control_rect(control);
        let s = self.s;
        match control {
            Control::Shortcut => RectF {
                x: bounds.x + bounds.w - 88.0 * s,
                y: bounds.y + 10.0 * s,
                w: 76.0 * s,
                h: 25.0 * s,
            },
            Control::Follow | Control::Startup => {
                let w = 40.0 * s;
                let h = 22.0 * s;
                RectF {
                    x: bounds.x + bounds.w - w - SETTINGS_INSET * s,
                    y: bounds.y + (bounds.h - h) * 0.5,
                    w,
                    h,
                }
            }
            _ => bounds,
        }
    }

    fn segment(&self, row: RectF, index: f32) -> RectF {
        let pad = 4.0 * self.s;
        let w = (row.w - 2.0 * pad) * 0.5;
        RectF {
            x: row.x + pad + index * w,
            y: row.y + pad,
            w,
            h: row.h - 2.0 * pad,
        }
    }
}

pub fn rect_contains(r: RectF, x: f32, y: f32) -> bool {
    x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h
}

unsafe fn set_smooth(g: *mut c_void) {
    GdipSetSmoothingMode(g, 4);
    GdipSetPixelOffsetMode(g, 4);
    GdipSetTextRenderingHint(g, 5);
}

unsafe fn round_path(r: RectF, rad: f32) -> *mut c_void {
    let mut path = ptr::null_mut();
    GdipCreatePath(0, &mut path);
    let d = (rad * 2.0).min(r.w.min(r.h));
    if d <= 0.1 {
        GdipAddPathRectangle(path, r.x, r.y, r.w, r.h);
    } else {
        GdipAddPathArc(path, r.x, r.y, d, d, 180.0, 90.0);
        GdipAddPathArc(path, r.x + r.w - d, r.y, d, d, 270.0, 90.0);
        GdipAddPathArc(path, r.x + r.w - d, r.y + r.h - d, d, d, 0.0, 90.0);
        GdipAddPathArc(path, r.x, r.y + r.h - d, d, d, 90.0, 90.0);
        GdipClosePathFigure(path);
    }
    path
}

unsafe fn fill_round(g: *mut c_void, r: RectF, rad: f32, color: u32) {
    let path = round_path(r, rad);
    let mut brush = ptr::null_mut();
    GdipCreateSolidFill(color, &mut brush);
    GdipFillPath(g, brush, path);
    GdipDeleteBrush(brush);
    GdipDeletePath(path);
}

unsafe fn stroke_round(g: *mut c_void, r: RectF, rad: f32, color: u32, width: f32, dashed: bool) {
    let path = round_path(r, rad);
    let mut pen = ptr::null_mut();
    GdipCreatePen1(color, width, 2, &mut pen);
    if dashed {
        GdipSetPenDashStyle(pen, 1);
    }
    GdipDrawPath(g, pen, path);
    GdipDeletePen(pen);
    GdipDeletePath(path);
}

unsafe fn make_font(pt: f32, semi: bool, underline: bool) -> (*mut c_void, *mut c_void) {
    let preferred = if semi { "Segoe UI Variable Text Semibold" } else { "Segoe UI Variable Text" };
    let fallback = if semi { "Segoe UI Semibold" } else { "Segoe UI" };
    let mut fam = ptr::null_mut();
    let w1 = wide(preferred);
    if GdipCreateFontFamilyFromName(w1.as_ptr(), ptr::null_mut(), &mut fam) != 0 || fam.is_null() {
        let w2 = wide(fallback);
        GdipCreateFontFamilyFromName(w2.as_ptr(), ptr::null_mut(), &mut fam);
    }
    let style = if underline { 4 } else { 0 };
    let mut font = ptr::null_mut();
    GdipCreateFont(fam, pt, style, 3, &mut font);
    (fam, font)
}

unsafe fn make_fmt(h_align: i32, v_align: i32) -> *mut c_void {
    let mut fmt = ptr::null_mut();
    GdipStringFormatGetGenericTypographic(&mut fmt);
    GdipSetStringFormatAlign(fmt, h_align);
    GdipSetStringFormatLineAlign(fmt, v_align);
    GdipSetStringFormatFlags(fmt, 0x1000 | 0x0800);
    fmt
}

unsafe fn measure_width(g: *mut c_void, text: &str, font: *mut c_void) -> f32 {
    let fmt = make_fmt(0, 0);
    let wtext: Vec<u16> = text.encode_utf16().collect();
    let layout = RectF { x: 0.0, y: 0.0, w: 1000.0, h: 100.0 };
    let mut bounds = RectF::default();
    GdipMeasureString(
        g,
        wtext.as_ptr(),
        wtext.len() as i32,
        font,
        &layout,
        fmt,
        &mut bounds,
        ptr::null_mut(),
        ptr::null_mut(),
    );
    GdipDeleteStringFormat(fmt);
    bounds.w
}

unsafe fn draw_text(
    g: *mut c_void,
    text: &str,
    pt: f32,
    semi: bool,
    underline: bool,
    color: u32,
    rect: RectF,
    h_align: i32,
) {
    let (fam, font) = make_font(pt, semi, underline);
    paint_text(g, text, font, color, rect, h_align);
    GdipDeleteFont(font);
    GdipDeleteFontFamily(fam);
}

unsafe fn paint_text(
    g: *mut c_void,
    text: &str,
    font: *mut c_void,
    color: u32,
    rect: RectF,
    h_align: i32,
) {
    let fmt = make_fmt(h_align, 1);
    let mut brush = ptr::null_mut();
    GdipCreateSolidFill(color, &mut brush);
    let wtext: Vec<u16> = text.encode_utf16().collect();
    GdipDrawString(
        g,
        wtext.as_ptr(),
        wtext.len() as i32,
        font,
        &rect,
        fmt,
        brush,
    );
    GdipDeleteBrush(brush);
    GdipDeleteStringFormat(fmt);
}

unsafe fn mono_font(pt: f32) -> (*mut c_void, *mut c_void) {
    let mut fam = ptr::null_mut();
    let name = wide("Consolas");
    GdipCreateFontFamilyFromName(name.as_ptr(), ptr::null_mut(), &mut fam);
    if fam.is_null() {
        return make_font(pt, false, false);
    }
    let mut font = ptr::null_mut();
    GdipCreateFont(fam, pt, 0, 3, &mut font);
    (fam, font)
}

unsafe fn hairline(g: *mut c_void, x: f32, right: f32, y: f32, color: u32) {
    let mut pen = ptr::null_mut();
    GdipCreatePen1(color, 1.0, 2, &mut pen);
    GdipDrawLine(g, pen, x, y, right, y);
    GdipDeletePen(pen);
}

unsafe fn draw_mouse(g: *mut c_void, x: f32, y: f32, h: f32, fg: u32, wheel: u32) -> f32 {
    let w = h * 0.58;
    let pen_w = (h * 0.085).max(1.35);
    let body = RectF {
        x: x + pen_w * 0.5,
        y: y + pen_w * 0.5,
        w: w - pen_w,
        h: h - pen_w,
    };
    stroke_round(g, body, w * 0.5, fg, pen_w, false);

    let ww = (w * 0.21).max(2.2);
    let wh = h * 0.26;
    let wr = RectF {
        x: x + (w - ww) * 0.5,
        y: y + h * 0.17,
        w: ww,
        h: wh,
    };
    fill_round(g, wr, ww * 0.5, wheel);

    let mut pen = ptr::null_mut();
    GdipCreatePen1(fg, (pen_w * 0.65).max(1.0), 2, &mut pen);
    GdipDrawLine(g, pen, x + pen_w * 0.8, y + h * 0.5, x + w - pen_w * 0.8, y + h * 0.5);
    GdipDeletePen(pen);

    let gap = h * 0.14;
    let ax = x + w + gap;
    let aw = h * 0.24;
    let ah = h * 0.15;
    let cy = y + h * 0.5;
    let off = h * 0.11;
    let up = [
        PointF { x: ax, y: cy - off },
        PointF { x: ax + aw, y: cy - off },
        PointF { x: ax + aw * 0.5, y: cy - off - ah },
    ];
    let dn = [
        PointF { x: ax, y: cy + off },
        PointF { x: ax + aw, y: cy + off },
        PointF { x: ax + aw * 0.5, y: cy + off + ah },
    ];
    let mut brush = ptr::null_mut();
    GdipCreateSolidFill(fg, &mut brush);
    GdipFillPolygon(g, brush, up.as_ptr(), 3, 0);
    GdipFillPolygon(g, brush, dn.as_ptr(), 3, 0);
    GdipDeleteBrush(brush);

    w + gap + aw
}

unsafe fn draw_numpad_icon(g: *mut c_void, x: f32, y: f32, h: f32, fg: u32) -> f32 {
    let w = h * 0.92;
    let pen_w = (h * 0.085).max(1.3);
    let box_r = RectF {
        x: x + pen_w * 0.5,
        y: y + pen_w * 0.5,
        w: w - pen_w,
        h: h - pen_w,
    };
    stroke_round(g, box_r, h * 0.22, fg, pen_w, false);

    let cx = x + w * 0.5;
    let cy = y + h * 0.36;
    let arm = h * 0.14;
    let mut pen = ptr::null_mut();
    GdipCreatePen1(fg, (pen_w * 0.9).max(1.2), 2, &mut pen);
    GdipSetPenStartCap(pen, 2);
    GdipSetPenEndCap(pen, 2);
    GdipDrawLine(g, pen, cx - arm, cy, cx + arm, cy);
    GdipDrawLine(g, pen, cx, cy - arm, cx, cy + arm);
    let my = y + h * 0.68;
    GdipDrawLine(g, pen, cx - arm, my, cx + arm, my);
    GdipDeletePen(pen);
    w
}

/// Sculpted "><" pinch mark on a crisp white tile (cyan -> cobalt -> violet).
pub unsafe fn make_icon(size: i32) -> HICON {
    let mut bmp = ptr::null_mut();
    GdipCreateBitmapFromScan0(size, size, 0, 0x0026_200A, ptr::null_mut(), &mut bmp);
    let mut g = ptr::null_mut();
    GdipGetImageGraphicsContext(bmp, &mut g);
    set_smooth(g);
    GdipGraphicsClear(g, 0x0000_0000);

    let sz = size as f32;
    let k = sz / 128.0;
    let tile = RectF { x: 0.5, y: 0.5, w: sz - 1.0, h: sz - 1.0 };
    let clip = round_path(tile, sz * 0.27);

    let p1 = PointF { x: 0.0, y: 0.0 };
    let p2 = PointF { x: sz, y: sz };
    let mut bg = ptr::null_mut();
    GdipCreateLineBrush(&p1, &p2, 0xFFFFFFFF, 0xFFF3EFE6, 0, &mut bg);
    GdipFillPath(g, bg, clip);
    GdipDeleteBrush(bg);

    let mut state = 0u32;
    GdipSaveGraphics(g, &mut state);
    GdipSetClipPath(g, clip, 0);

    // Subtle center halo
    let mut glow_path = ptr::null_mut();
    GdipCreatePath(0, &mut glow_path);
    let gr = sz * 0.44;
    GdipAddPathEllipse(glow_path, sz * 0.5 - gr, sz * 0.5 - gr, gr * 2.0, gr * 2.0);
    let mut pgb = ptr::null_mut();
    GdipCreatePathGradientFromPath(glow_path, &mut pgb);
    GdipSetPathGradientCenterColor(pgb, 0x293757A6);
    let sur = [0x003757A6u32];
    let mut cnt = 1i32;
    GdipSetPathGradientSurroundColorsWithCount(pgb, sur.as_ptr(), &mut cnt);
    GdipFillPath(g, pgb, glow_path);
    GdipDeleteBrush(pgb);
    GdipDeletePath(glow_path);

    // Sculpted >< gradient chevrons
    let gp1 = PointF { x: 20.0 * k, y: 30.0 * k };
    let gp2 = PointF { x: 108.0 * k, y: 98.0 * k };
    let mut grad = ptr::null_mut();
    GdipCreateLineBrush(&gp1, &gp2, 0xFF3B82A6, 0xFF5D4B9E, 0, &mut grad);
    let colors = [0xFF3B82A6u32, 0xFF3757A6u32, 0xFF5D4B9Eu32];
    let pos = [0.0f32, 0.52f32, 1.0f32];
    GdipSetLinePresetBlend(grad, colors.as_ptr(), pos.as_ptr(), 3);

    let mut pen = ptr::null_mut();
    GdipCreatePen2(grad, (17.5 * k).max(2.2), 2, &mut pen);
    GdipSetPenStartCap(pen, 2);
    GdipSetPenEndCap(pen, 2);
    GdipSetPenLineJoin(pen, 2);

    let mut left = ptr::null_mut();
    GdipCreatePath(0, &mut left);
    GdipAddPathBezier(left, 28.0 * k, 36.0 * k, 46.0 * k, 52.0 * k, 52.0 * k, 58.0 * k, 52.0 * k, 64.0 * k);
    GdipAddPathBezier(left, 52.0 * k, 64.0 * k, 52.0 * k, 70.0 * k, 46.0 * k, 76.0 * k, 28.0 * k, 92.0 * k);
    GdipDrawPath(g, pen, left);
    GdipDeletePath(left);

    let mut right = ptr::null_mut();
    GdipCreatePath(0, &mut right);
    GdipAddPathBezier(right, 100.0 * k, 36.0 * k, 82.0 * k, 52.0 * k, 76.0 * k, 58.0 * k, 76.0 * k, 64.0 * k);
    GdipAddPathBezier(right, 76.0 * k, 64.0 * k, 76.0 * k, 70.0 * k, 82.0 * k, 76.0 * k, 100.0 * k, 92.0 * k);
    GdipDrawPath(g, pen, right);
    GdipDeletePath(right);

    GdipDeletePen(pen);
    GdipDeleteBrush(grad);
    GdipRestoreGraphics(g, state);

    let mut rim = ptr::null_mut();
    GdipCreatePen1(0xFFD8D0BE, (2.2 * k).max(0.6), 2, &mut rim);
    GdipDrawPath(g, rim, clip);
    GdipDeletePen(rim);
    GdipDeletePath(clip);

    GdipDeleteGraphics(g);
    let mut hicon: HICON = 0;
    GdipCreateHICONFromBitmap(bmp, &mut hicon);
    GdipDisposeImage(bmp);
    hicon
}

pub unsafe fn paint_window(hdc: HDC, w: i32, h: i32, ui: &UiState) {
    let mem_dc = CreateCompatibleDC(hdc);
    let bmp = CreateCompatibleBitmap(hdc, w, h);
    let old = SelectObject(mem_dc, bmp);

    let mut g = ptr::null_mut();
    GdipCreateFromHDC(mem_dc, &mut g);
    set_smooth(g);
    let p = Palette::new(ui.dark);
    GdipGraphicsClear(g, p.bg);

    let lay = Layout::new(ui.scale);
    paint_key_stage(g, &lay, ui, p);
    draw_text(g, "Zoom with", 9.2, true, false, p.ink, lay.cap_action, 0);
    paint_seg_action(g, &lay, ui, p);
    draw_text(
        g,
        "Modifier keys",
        9.2,
        true,
        false,
        p.ink,
        lay.cap_preset,
        0,
    );
    paint_seg_preset(g, &lay, ui, p);

    hairline(
        g,
        lay.key_stage.x,
        lay.key_stage.x + lay.key_stage.w,
        lay.divider_y,
        p.line,
    );
    paint_toggle_row(
        g,
        &lay,
        ui,
        Control::Follow,
        "Follow mouse",
        ui.follow_k,
        ui.hover_follow,
        p,
    );
    paint_toggle_row(
        g,
        &lay,
        ui,
        Control::Startup,
        "Start with Windows",
        ui.startup_k,
        ui.hover_startup,
        p,
    );

    paint_control_signal(g, &lay, ui, Control::Reset, p);
    if ui.hover_reset {
        fill_round(g, lay.reset_link, BUTTON_RADIUS * lay.s, p.hover);
    }
    draw_text(
        g,
        "Reset to defaults",
        9.0,
        true,
        false,
        p.ink,
        RectF {
            x: lay.reset_link.x + SETTINGS_INSET * lay.s,
            w: lay.reset_link.w - SETTINGS_INSET * lay.s,
            ..lay.reset_link
        },
        0,
    );

    let strength = if ui.press_done {
        0.35
    } else {
        0.24 * ui.done_hv
    };
    let button = if ui.press_done {
        inset(lay.done_btn, 1.1 * lay.s)
    } else {
        lay.done_btn
    };
    paint_control_signal(g, &lay, ui, Control::Done, p);
    fill_round(g, button, BUTTON_RADIUS * lay.s, p.button(strength));
    draw_text(g, "Done", 9.5, true, false, p.on_cta, button, 1);

    if let Some(control) = ui.keyboard_focus {
        let rect = inset(lay.button_rect(control), 2.0 * lay.s);
        let color = if matches!(control, Control::Done)
            || matches!(control, Control::Action(i) if i == ui.action)
            || matches!(control, Control::Preset(i) if i == ui.preset())
            || (control == Control::Follow && ui.follow_mouse)
            || (control == Control::Startup && ui.startup_on)
        {
            p.on_cta
        } else {
            p.accent
        };
        stroke_round(
            g, rect, (BUTTON_RADIUS - 2.0) * lay.s, color, 1.2 * lay.s, false,
        );
    }

    GdipDeleteGraphics(g);
    BitBlt(hdc, 0, 0, w, h, mem_dc, 0, 0, SRCCOPY);
    SelectObject(mem_dc, old);
    DeleteObject(bmp);
    DeleteDC(mem_dc);
}

fn inset(r: RectF, amount: f32) -> RectF {
    RectF {
        x: r.x + amount,
        y: r.y + amount,
        w: r.w - 2.0 * amount,
        h: r.h - 2.0 * amount,
    }
}

unsafe fn paint_control_signal(
    g: *mut c_void,
    lay: &Layout,
    ui: &UiState,
    control: Control,
    p: Palette,
) {
    let Some((source, began)) = ui.signal else {
        return;
    };
    if source != control {
        return;
    }
    let t = began.elapsed().as_secs_f32() / 0.53;
    if t >= 1.0 {
        return;
    }
    let s = lay.s;
    let r = lay.button_rect(control);
    let travel = (2.0 + 9.0 * (1.0 - (1.0 - t).powi(3))) * s;
    let alpha = ((t * 16.0).min(1.0) * (1.0 - t) * 225.0) as u8;
    let (fam, font) = mono_font(8.0);
    for (i, (x, y)) in [
        (0.16, -1.0),
        (0.62, -1.0),
        (1.0, -0.5),
        (1.0, 0.65),
        (-0.16, 1.0),
        (-0.62, 1.0),
        (-1.0, 0.5),
        (-1.0, -0.65),
    ]
    .into_iter()
    .enumerate()
    {
        let point = PointF {
            x: r.x + r.w * 0.5 + x * (r.w * 0.5 + travel),
            y: r.y + r.h * 0.5 + y * (r.h * 0.5 + travel),
        };
        let mark = if i % 2 == 0 { "+" } else { "." };
        paint_text(
            g,
            mark,
            font,
            with_alpha(p.signal, alpha),
            RectF {
                x: point.x - 6.0 * s,
                y: point.y - 7.0 * s,
                w: 12.0 * s,
                h: 14.0 * s,
            },
            1,
        );
    }
    GdipDeleteFont(font);
    GdipDeleteFontFamily(fam);
}

pub fn recording_wave(tick: i32) -> Option<f32> {
    const WAVE_TICKS: i32 = 32;
    const REST_TICKS: i32 = 22;
    let phase = tick % (WAVE_TICKS + REST_TICKS);
    (phase < WAVE_TICKS).then(|| phase as f32 / (WAVE_TICKS - 1) as f32)
}

unsafe fn paint_recording_wave(g: *mut c_void, lane: RectF, progress: f32, color: u32) {
    const MARKS: [&str; 11] = [".", "+", ".", "*", "+", ".", "*", ".", "+", "*", "."];
    const RADIUS: f32 = 2.6;
    let width = lane.w / MARKS.len() as f32;
    let travel = (MARKS.len() - 1) as f32 + 2.0 * RADIUS;
    let crest = progress * travel - RADIUS;
    let (family, font) = mono_font(8.0);

    for (column, mark) in MARKS.into_iter().enumerate() {
        let strength = (1.0 - (column as f32 - crest).abs() / RADIUS).clamp(0.0, 1.0);
        let fade = strength * strength * (3.0 - 2.0 * strength);
        let alpha = (fade * 255.0).round() as u8;
        if alpha == 0 {
            continue;
        }

        paint_text(
            g,
            mark,
            font,
            with_alpha(color, alpha),
            RectF {
                x: lane.x + column as f32 * width,
                w: width,
                ..lane
            },
            1,
        );
    }

    GdipDeleteFont(font);
    GdipDeleteFontFamily(family);
}

unsafe fn paint_key_stage(g: *mut c_void, lay: &Layout, ui: &UiState, p: Palette) {
    let s = lay.s;
    let r = lay.key_stage;
    fill_round(g, r, 11.0 * s, p.sub);
    let border = if ui.recording {
        p.accent
    } else if ui.hover_stage {
        p.strong
    } else {
        p.line
    };
    stroke_round(g, inset(r, 0.5), 11.0 * s, border, s, false);

    let label = RectF {
        x: r.x + 13.0 * s,
        y: r.y + 7.0 * s,
        w: 198.0 * s,
        h: 30.0 * s,
    };
    draw_text(
        g,
        if ui.recording {
            "Record modifier keys"
        } else {
            "Zoom shortcut"
        },
        10.5,
        true,
        false,
        p.ink,
        label,
        0,
    );
    if ui.recording && ui.motion {
        if let Some(progress) = recording_wave(ui.rec_ticks) {
            let lane = RectF {
                x: r.x + r.w - 168.0 * s,
                y: label.y,
                w: 70.0 * s,
                h: label.h,
            };
            paint_recording_wave(g, lane, progress, p.signal);
        }
    }

    let button = lay.button_rect(Control::Shortcut);
    paint_control_signal(g, lay, ui, Control::Shortcut, p);
    if ui.hover_stage {
        fill_round(g, button, BUTTON_RADIUS * s, p.hover);
    }
    draw_text(
        g,
        if ui.recording { "Cancel" } else { "Edit keys" },
        9.0,
        true,
        false,
        p.ink,
        button,
        1,
    );

    paint_keycaps(g, r, r.y + 68.0 * s, s, ui, p);
    let foot_y = r.y + r.h - 32.0 * s;
    hairline(g, r.x + 13.0 * s, r.x + r.w - 13.0 * s, foot_y, p.line);
    let foot = RectF {
        x: r.x + 13.0 * s,
        y: foot_y,
        w: r.w - 26.0 * s,
        h: 31.0 * s,
    };
    let (hint, color) = if ui.recording && ui.rec_peak == MOD_SHIFT {
        ("Add another modifier to Shift.".into(), p.error)
    } else if ui.recording {
        ("Release the keys to save. Esc cancels.".into(), p.muted)
    } else if let Some(warn) = &ui.warning {
        (warn.clone(), p.error)
    } else {
        (
            format!(
                "Reset zoom: {}",
                format_reset_summary(ui.active_mods(), ui.action)
            ),
            p.muted,
        )
    };
    draw_text(g, &hint, 8.8, false, false, color, foot, 1);
}

enum KeyItem {
    Cap(&'static str),
    Ghost(&'static str),
    Plus,
    Slash,
    Mouse,
}

unsafe fn paint_keycaps(g: *mut c_void, card: RectF, cy: f32, s: f32, ui: &UiState, p: Palette) {
    let kh = 34.0 * s;
    let gap = 6.0 * s;
    let mods = if ui.recording {
        ui.rec_peak
    } else {
        ui.active_mods()
    };

    let mut items: Vec<(KeyItem, f32)> = Vec::new();
    let names = [
        ("Ctrl", MOD_CTRL),
        ("Alt", MOD_ALT),
        ("Shift", MOD_SHIFT),
        ("Win", MOD_WIN),
    ];
    let mut first = true;
    for &(name, bit) in &names {
        if (mods & bit) == 0 {
            continue;
        }
        if !first {
            items.push((KeyItem::Plus, 0.0));
        }
        items.push((KeyItem::Cap(name), 0.0));
        first = false;
    }
    let need_trailing_plus = !ui.recording || mods == MOD_SHIFT;
    if !first && need_trailing_plus {
        items.push((KeyItem::Plus, 0.0));
    }

    if ui.recording {
        if mods == 0 {
            items.push((KeyItem::Ghost("Hold modifier keys"), 0.0));
        } else if mods == MOD_SHIFT {
            items.push((KeyItem::Ghost("Add another key"), 0.0));
        }
    } else if ui.action == 0 {
        items.push((KeyItem::Mouse, 0.0));
    } else {
        items.push((KeyItem::Cap("Num +"), 0.0));
        items.push((KeyItem::Slash, 0.0));
        items.push((KeyItem::Cap("Num \u{2212}"), 0.0));
    }

    let (kfam, kfont) = make_font(9.5, true, false);
    let (pfam, pfont) = make_font(10.5, false, false);
    let mut total = 0.0f32;
    for i in 0..items.len() {
        let w = match items[i].0 {
            KeyItem::Cap(t) | KeyItem::Ghost(t) => {
                (measure_width(g, t, kfont) + 16.0 * s).max(kh * 1.15)
            }
            KeyItem::Mouse => kh * 0.58 + kh * 0.14 + kh * 0.24,
            KeyItem::Plus => measure_width(g, "+", pfont) + 4.0 * s,
            KeyItem::Slash => measure_width(g, "/", pfont) + 4.0 * s,
        };
        items[i].1 = w;
        total += w;
        if i + 1 < items.len() {
            total += gap;
        }
    }
    GdipDeleteFont(kfont);
    GdipDeleteFontFamily(kfam);
    GdipDeleteFont(pfont);
    GdipDeleteFontFamily(pfam);

    let avail = card.w - 28.0 * s;
    let cx = card.x + card.w * 0.5;
    let mut st = 0u32;
    GdipSaveGraphics(g, &mut st);
    if total > avail && total > 1.0 {
        let sc = avail / total;
        GdipTranslateWorldTransform(g, cx, cy, 0);
        GdipScaleWorldTransform(g, sc, sc, 0);
        GdipTranslateWorldTransform(g, -cx, -cy, 0);
    }

    let mut x = cx - total * 0.5;
    let top = cy - kh * 0.5;
    for (item, w) in items {
        match item {
            KeyItem::Cap(t) => draw_keycap(
                g,
                t,
                x,
                top,
                w,
                kh,
                s,
                false,
                ui.recording,
                mods == MOD_SHIFT,
                p,
            ),
            KeyItem::Ghost(t) => draw_keycap(
                g,
                t,
                x,
                top,
                w,
                kh,
                s,
                true,
                ui.recording,
                mods == MOD_SHIFT,
                p,
            ),
            KeyItem::Mouse => {
                draw_mouse(g, x, top + 1.0 * s, kh - 2.0 * s, p.ink, p.accent);
            }
            KeyItem::Plus => {
                draw_text(
                    g,
                    "+",
                    10.5,
                    false,
                    false,
                    p.muted,
                    RectF {
                        x,
                        y: top,
                        w,
                        h: kh,
                    },
                    1,
                );
            }
            KeyItem::Slash => {
                draw_text(
                    g,
                    "/",
                    10.5,
                    false,
                    false,
                    p.muted,
                    RectF {
                        x,
                        y: top,
                        w,
                        h: kh,
                    },
                    1,
                );
            }
        }
        x += w + gap;
    }

    GdipRestoreGraphics(g, st);
}

unsafe fn draw_keycap(
    g: *mut c_void,
    text: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    s: f32,
    ghost: bool,
    recording: bool,
    shift_only: bool,
    p: Palette,
) {
    let rect = RectF { x, y, w, h };
    if ghost {
        let col = if shift_only { p.error } else { p.muted };
        stroke_round(g, rect, 9.0 * s, p.strong, s, true);
        draw_text(g, text, 9.5, true, false, col, rect, 1);
        return;
    }
    fill_round(g, rect, 9.0 * s, p.bg);
    stroke_round(
        g,
        rect,
        9.0 * s,
        if recording { p.accent } else { p.strong },
        s,
        false,
    );
    draw_text(g, text, 9.5, true, false, p.ink, rect, 1);
}

unsafe fn paint_seg_track(
    g: *mut c_void,
    lay: &Layout,
    ui: &UiState,
    r: RectF,
    control: fn(i32) -> Control,
    cur: f32,
    sel: i32,
    hover: i32,
    p: Palette,
) {
    fill_round(g, r, 11.0 * lay.s, p.sub);
    for index in 0..2 {
        paint_control_signal(g, lay, ui, control(index), p);
    }
    if hover >= 0 && hover != sel {
        fill_round(g, lay.segment(r, hover as f32), 9.0 * lay.s, p.hover);
    }
    fill_round(
        g,
        lay.segment(r, cur),
        9.0 * lay.s,
        p.button(if hover == sel { 0.22 } else { 0.0 }),
    );
}

unsafe fn paint_seg_action(g: *mut c_void, lay: &Layout, ui: &UiState, p: Palette) {
    let r = lay.seg_action;
    let s = lay.s;
    paint_seg_track(
        g, lay, ui, r, Control::Action, ui.action_cur, ui.action, ui.hover_action, p,
    );
    let items = [("Scroll wheel", true), ("Numpad + / \u{2212}", false)];
    for (i, &(label, is_mouse)) in items.iter().enumerate() {
        let ir = lay.segment(r, i as f32);
        let near = (1.0 - (i as f32 - ui.action_cur).abs()).max(0.0);
        let fg = mix(p.muted, p.on_cta, near);
        let ih = 16.0 * s;
        let gap = 7.0 * s;
        let iw = if is_mouse { ih * 0.96 } else { ih * 0.92 };
        let (fam, font) = make_font(9.2, true, false);
        let tw = measure_width(g, label, font);
        let sx = ir.x + (ir.w - (iw + gap + tw)) * 0.5;
        let iy = ir.y + (ir.h - ih) * 0.5;
        if is_mouse {
            draw_mouse(g, sx, iy, ih, fg, fg);
        } else {
            draw_numpad_icon(g, sx, iy, ih, fg);
        }
        paint_text(
            g,
            label,
            font,
            fg,
            RectF {
                x: sx + iw + gap,
                w: tw + 6.0 * s,
                ..ir
            },
            0,
        );
        GdipDeleteFont(font);
        GdipDeleteFontFamily(fam);
    }
}

unsafe fn paint_seg_preset(g: *mut c_void, lay: &Layout, ui: &UiState, p: Palette) {
    let r = lay.seg_preset;
    paint_seg_track(
        g, lay, ui, r, Control::Preset, ui.preset_cur, ui.preset(), ui.hover_preset, p,
    );
    for (i, &label) in ["Default", "Custom"].iter().enumerate() {
        let near = (1.0 - (i as f32 - ui.preset_cur).abs()).max(0.0);
        draw_text(
            g,
            label,
            9.5,
            true,
            false,
            mix(p.muted, p.on_cta, near),
            lay.segment(r, i as f32),
            1,
        );
    }
}

unsafe fn paint_toggle_row(
    g: *mut c_void,
    lay: &Layout,
    ui: &UiState,
    control: Control,
    label: &str,
    k: f32,
    hover: bool,
    p: Palette,
) {
    let s = lay.s;
    let r = lay.control_rect(control);
    let track = lay.button_rect(control);
    let inset = SETTINGS_INSET * s;
    if hover {
        fill_round(g, r, BUTTON_RADIUS * s, p.hover);
    }
    draw_text(
        g,
        label,
        9.5,
        false,
        false,
        p.ink,
        RectF {
            x: r.x + inset,
            w: track.x - r.x - 2.0 * inset,
            ..r
        },
        0,
    );

    paint_control_signal(g, lay, ui, control, p);
    let track_fill = mix(p.sub, p.cta, k);
    let track_edge = mix(p.strong, p.cta, k);
    fill_round(g, track, BUTTON_RADIUS * s, track_fill);
    stroke_round(g, track, BUTTON_RADIUS * s, track_edge, 1.0, false);

    let inset = 3.0 * s;
    let d = track.h - inset * 2.0;
    let kx = track.x + inset + (track.w - d - inset * 2.0) * k;
    let ky = track.y + inset;
    let mut kb = ptr::null_mut();
    GdipCreateSolidFill(mix(p.muted, p.on_cta, k), &mut kb);
    GdipFillEllipse(g, kb, kx, ky, d, d);
    GdipDeleteBrush(kb);
}
