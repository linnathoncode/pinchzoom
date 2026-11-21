use std::ffi::c_void;
use std::ptr;

use crate::win32::*;
use crate::{format_reset_summary, UiState, MOD_ALT, MOD_CTRL, MOD_SHIFT, MOD_WIN};

pub const PAPER: u32 = 0xFFF4EFE4;
pub const CARD: u32 = 0xFFFCF9F2;
pub const SUBTLE: u32 = 0xFFECE5D5;
pub const LINE: u32 = 0xFFDDD5C3;
pub const LINE_HOVER: u32 = 0xFFBEB49E;
pub const INK: u32 = 0xFF23211D;
pub const MUTED: u32 = 0xFF6B6559;
pub const ACCENT: u32 = 0xFF23211D;
pub const ON_ACCENT: u32 = 0xFFFAF7F0;
pub const KEY_FACE: u32 = 0xFFFFFFFF;
pub const KEY_SHADOW: u32 = 0xFFD4CBB7;
pub const FOCUS_BLUE: u32 = 0xFF2457C5;
pub const FOCUS_BG: u32 = 0xFFEFF4FC;
pub const FOCUS_LINE: u32 = 0xFFA6C0F0;
pub const ERROR_RED: u32 = 0xFFB43E2B;
pub const ERROR_BG: u32 = 0xFFFDF1EE;
pub const ERROR_LINE: u32 = 0xFFE8B4AA;

pub const CLIENT_W: f32 = 368.0;
pub const CLIENT_H: f32 = 478.0;

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
        let w = 328.0 * s;
        Self {
            s,
            key_stage: RectF { x: m, y: 10.0 * s, w, h: 128.0 * s },
            cap_action: RectF { x: m, y: 154.0 * s, w, h: 18.0 * s },
            seg_action: RectF { x: m, y: 178.0 * s, w, h: 42.0 * s },
            cap_preset: RectF { x: m, y: 236.0 * s, w, h: 18.0 * s },
            seg_preset: RectF { x: m, y: 260.0 * s, w, h: 42.0 * s },
            divider_y: 320.0 * s,
            follow_row: RectF { x: m - 4.0 * s, y: 330.0 * s, w: w + 8.0 * s, h: 38.0 * s },
            startup_row: RectF { x: m - 4.0 * s, y: 370.0 * s, w: w + 8.0 * s, h: 38.0 * s },
            reset_link: RectF { x: m, y: 422.0 * s, w: 140.0 * s, h: 36.0 * s },
            done_btn: RectF { x: m + w - 104.0 * s, y: 422.0 * s, w: 104.0 * s, h: 36.0 * s },
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
    let fmt = make_fmt(h_align, 1);
    let mut brush = ptr::null_mut();
    GdipCreateSolidFill(color, &mut brush);
    let wtext: Vec<u16> = text.encode_utf16().collect();
    GdipDrawString(g, wtext.as_ptr(), wtext.len() as i32, font, &rect, fmt, brush);
    GdipDeleteBrush(brush);
    GdipDeleteStringFormat(fmt);
    GdipDeleteFont(font);
    GdipDeleteFontFamily(fam);
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
    GdipSetPathGradientCenterColor(pgb, 0x2A2563EB);
    let sur = [0x007C3AEDu32];
    let mut cnt = 1i32;
    GdipSetPathGradientSurroundColorsWithCount(pgb, sur.as_ptr(), &mut cnt);
    GdipFillPath(g, pgb, glow_path);
    GdipDeleteBrush(pgb);
    GdipDeletePath(glow_path);

    // Sculpted >< gradient chevrons
    let gp1 = PointF { x: 20.0 * k, y: 30.0 * k };
    let gp2 = PointF { x: 108.0 * k, y: 98.0 * k };
    let mut grad = ptr::null_mut();
    GdipCreateLineBrush(&gp1, &gp2, 0xFF0EA5E9, 0xFF7C3AED, 0, &mut grad);
    let colors = [0xFF0EA5E9u32, 0xFF2563EBu32, 0xFF7C3AEDu32];
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
    GdipCreatePen1(0xFFD8D0BE, (1.5 * k).max(1.0), 2, &mut rim);
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
    GdipGraphicsClear(g, PAPER);

    let lay = Layout::new(ui.scale);
    paint_key_stage(g, &lay, ui);

    draw_text(
        g,
        "Zoom with",
        8.5,
        true,
        false,
        MUTED,
        RectF { x: lay.cap_action.x + 2.0 * lay.s, ..lay.cap_action },
        0,
    );
    paint_seg_action(g, &lay, ui);

    draw_text(
        g,
        "Modifier keys",
        8.5,
        true,
        false,
        MUTED,
        RectF { x: lay.cap_preset.x + 2.0 * lay.s, ..lay.cap_preset },
        0,
    );
    paint_seg_preset(g, &lay, ui);

    // Hairline divider
    let mut div_pen = ptr::null_mut();
    GdipCreatePen1(LINE, 1.0, 2, &mut div_pen);
    GdipDrawLine(g, div_pen, 20.0 * lay.s, lay.divider_y, (w as f32) - 20.0 * lay.s, lay.divider_y);
    GdipDeletePen(div_pen);

    paint_toggle_row(g, lay.follow_row, lay.s, "Follow mouse", ui.follow_k, ui.hover_follow);
    paint_toggle_row(g, lay.startup_row, lay.s, "Start with Windows", ui.startup_k, ui.hover_startup);

    // Reset to defaults link
    draw_text(
        g,
        "Reset to defaults",
        9.0,
        true,
        ui.hover_reset,
        if ui.hover_reset { INK } else { MUTED },
        RectF { x: lay.reset_link.x + 2.0 * lay.s, ..lay.reset_link },
        0,
    );

    // Done primary button
    let btn_fill = if ui.press_done {
        0xFF141310
    } else {
        mix(ACCENT, 0xFF3B3730, ui.done_hv)
    };
    fill_round(g, lay.done_btn, 9.0 * lay.s, btn_fill);
    draw_text(g, "Done", 9.5, true, false, ON_ACCENT, lay.done_btn, 1);

    GdipDeleteGraphics(g);
    BitBlt(hdc, 0, 0, w, h, mem_dc, 0, 0, SRCCOPY);
    SelectObject(mem_dc, old);
    DeleteObject(bmp);
    DeleteDC(mem_dc);
}

unsafe fn paint_key_stage(g: *mut c_void, lay: &Layout, ui: &UiState) {
    let s = lay.s;
    let r = RectF {
        x: lay.key_stage.x + 0.5,
        y: lay.key_stage.y + 0.5,
        w: lay.key_stage.w - 1.0,
        h: lay.key_stage.h - 1.0,
    };
    let rad = 12.0 * s;
    let has_warn = !ui.recording && ui.warning.is_some();

    let bg = if ui.recording { FOCUS_BG } else { CARD };
    let border = if ui.recording {
        FOCUS_BLUE
    } else if has_warn {
        ERROR_LINE
    } else if ui.hover_stage {
        LINE_HOVER
    } else {
        LINE
    };
    let border_w = if ui.recording { 1.5 } else { 1.0 };

    let clip = round_path(r, rad);
    let mut bg_b = ptr::null_mut();
    GdipCreateSolidFill(bg, &mut bg_b);
    GdipFillPath(g, bg_b, clip);
    GdipDeleteBrush(bg_b);

    let mut st = 0u32;
    GdipSaveGraphics(g, &mut st);
    GdipSetClipPath(g, clip, 0);

    let foot_h = 32.0 * s;
    let foot_y = r.y + r.h - foot_h;
    let foot_bg = if ui.recording {
        mix(FOCUS_BG, FOCUS_LINE, 0.22)
    } else if has_warn {
        ERROR_BG
    } else {
        SUBTLE
    };
    let foot_rule = if ui.recording {
        FOCUS_LINE
    } else if has_warn {
        ERROR_LINE
    } else {
        LINE
    };

    let mut fb = ptr::null_mut();
    GdipCreateSolidFill(foot_bg, &mut fb);
    GdipFillRectangle(g, fb, r.x, foot_y, r.w, foot_h);
    GdipDeleteBrush(fb);
    let mut fp = ptr::null_mut();
    GdipCreatePen1(foot_rule, 1.0, 2, &mut fp);
    GdipDrawLine(g, fp, r.x, foot_y, r.x + r.w, foot_y);
    GdipDeletePen(fp);

    // Top bar inside card
    let hdr_x = r.x + 14.0 * s;
    let hdr_y = r.y + 8.0 * s;
    let hdr_h = 20.0 * s;
    if ui.recording {
        let alpha = (255.0 * (0.45 + 0.55 * (0.5 + 0.5 * ui.pulse_phase.sin()))) as u8;
        let dot_d = 7.0 * s;
        let mut db = ptr::null_mut();
        GdipCreateSolidFill(with_alpha(FOCUS_BLUE, alpha), &mut db);
        GdipFillEllipse(g, db, hdr_x, hdr_y + (hdr_h - dot_d) * 0.5, dot_d, dot_d);
        GdipDeleteBrush(db);
        draw_text(
            g,
            "RECORDING",
            7.8,
            true,
            false,
            FOCUS_BLUE,
            RectF { x: hdr_x + dot_d + 6.0 * s, y: hdr_y, w: 140.0 * s, h: hdr_h },
            0,
        );
    } else {
        draw_text(
            g,
            "ACTIVE SHORTCUT",
            7.8,
            true,
            false,
            MUTED,
            RectF { x: hdr_x, y: hdr_y, w: 160.0 * s, h: hdr_h },
            0,
        );
    }

    // Top-right action pill
    let is_custom = ui.use_custom && ui.custom_mods != 0;
    let pill_text = if ui.recording {
        "Cancel"
    } else if is_custom {
        "Edit keys"
    } else {
        "Customize"
    };
    let (pfam, pfont) = make_font(8.0, true, false);
    let tw = measure_width(g, pill_text, pfont);
    GdipDeleteFont(pfont);
    GdipDeleteFontFamily(pfam);
    let pw = tw + 16.0 * s;
    let ph = 20.0 * s;
    let pr = RectF { x: r.x + r.w - 12.0 * s - pw, y: hdr_y, w: pw, h: ph };
    let p_bg = if ui.recording {
        if ui.hover_stage { mix(FOCUS_BLUE, 0xFFFFFFFF, 0.78) } else { 0xFFDCE8FA }
    } else if ui.hover_stage {
        ACCENT
    } else {
        SUBTLE
    };
    let p_fg = if ui.recording {
        FOCUS_BLUE
    } else if ui.hover_stage {
        ON_ACCENT
    } else {
        INK
    };
    fill_round(g, pr, ph * 0.5, p_bg);
    draw_text(g, pill_text, 8.0, true, false, p_fg, pr, 1);

    // Keycaps centered in stage
    let stage_top = r.y + 34.0 * s;
    let stage_bot = foot_y - 4.0 * s;
    let cy = (stage_top + stage_bot) * 0.5;
    paint_keycaps(g, r, cy, s, ui);

    // Bottom strip text
    let foot_rect = RectF { x: r.x + 12.0 * s, y: foot_y, w: r.w - 24.0 * s, h: foot_h };
    if ui.recording {
        let (hint, col) = if ui.rec_peak == MOD_SHIFT {
            ("Shift alone clashes with horizontal scroll. Add another key.", ERROR_RED)
        } else if ui.rec_peak != 0 {
            ("Release keys to save  \u{00b7}  keep holding to add more", FOCUS_BLUE)
        } else {
            ("Hold modifier keys, then release  \u{00b7}  Esc to cancel", FOCUS_BLUE)
        };
        draw_text(g, hint, 8.2, false, false, col, foot_rect, 1);
    } else if let Some(ref warn) = ui.warning {
        draw_text(g, warn, 8.2, true, false, ERROR_RED, foot_rect, 1);
    } else {
        let prefix = "Reset zoom: ";
        let combo = format_reset_summary(ui.active_mods(), ui.action);
        let (f1fam, f1) = make_font(8.5, false, false);
        let (f2fam, f2) = make_font(8.5, true, false);
        let w1 = measure_width(g, prefix, f1);
        let w2 = measure_width(g, &combo, f2);
        GdipDeleteFont(f1);
        GdipDeleteFontFamily(f1fam);
        GdipDeleteFont(f2);
        GdipDeleteFontFamily(f2fam);
        let sx = r.x + (r.w - (w1 + w2)) * 0.5;
        draw_text(g, prefix, 8.5, false, false, MUTED, RectF { x: sx, y: foot_y, w: w1 + 4.0 * s, h: foot_h }, 0);
        draw_text(g, &combo, 8.5, true, false, INK, RectF { x: sx + w1, y: foot_y, w: w2 + 6.0 * s, h: foot_h }, 0);
    }

    GdipRestoreGraphics(g, st);

    let mut bp = ptr::null_mut();
    GdipCreatePen1(border, border_w, 2, &mut bp);
    GdipDrawPath(g, bp, clip);
    GdipDeletePen(bp);
    GdipDeletePath(clip);
}

enum KeyItem {
    Cap(&'static str),
    Ghost(&'static str),
    Plus,
    Slash,
    Mouse,
}

unsafe fn paint_keycaps(g: *mut c_void, card: RectF, cy: f32, s: f32, ui: &UiState) {
    let kh = 33.0 * s;
    let gap = 8.0 * s;
    let mods = if ui.recording { ui.rec_peak } else { ui.active_mods() };

    let mut items: Vec<(KeyItem, f32)> = Vec::new();
    let names = [("Ctrl", MOD_CTRL), ("Alt", MOD_ALT), ("Shift", MOD_SHIFT), ("Win", MOD_WIN)];
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
            KeyItem::Cap(t) | KeyItem::Ghost(t) => (measure_width(g, t, kfont) + 22.0 * s).max(kh * 1.25),
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
            KeyItem::Cap(t) => draw_keycap(g, t, x, top, w, kh, s, false, ui.recording, mods == MOD_SHIFT),
            KeyItem::Ghost(t) => draw_keycap(g, t, x, top, w, kh, s, true, ui.recording, mods == MOD_SHIFT),
            KeyItem::Mouse => {
                draw_mouse(g, x, top + 1.0 * s, kh - 2.0 * s, INK, FOCUS_BLUE);
            }
            KeyItem::Plus => {
                draw_text(g, "+", 10.5, false, false, MUTED, RectF { x, y: top, w, h: kh }, 1);
            }
            KeyItem::Slash => {
                draw_text(g, "/", 10.5, false, false, MUTED, RectF { x, y: top, w, h: kh }, 1);
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
) {
    let rad = 7.0 * s;
    let skirt = 3.0 * s;
    if ghost {
        let gr = RectF { x, y, w, h };
        let col = if shift_only { ERROR_RED } else { FOCUS_BLUE };
        stroke_round(g, gr, rad, col, 1.3, true);
        draw_text(g, text, 9.5, true, false, col, gr, 1);
        return;
    }

    let base_r = RectF { x, y: y + skirt, w, h: h - skirt };
    let face_r = RectF { x, y, w, h: h - skirt };
    fill_round(g, base_r, rad, if recording { FOCUS_LINE } else { KEY_SHADOW });
    fill_round(g, face_r, rad, KEY_FACE);
    stroke_round(g, face_r, rad, if recording { FOCUS_BLUE } else { LINE }, 1.0, false);
    draw_text(g, text, 9.5, true, false, if recording { FOCUS_BLUE } else { INK }, face_r, 1);
}

unsafe fn paint_seg_track(g: *mut c_void, r: RectF, s: f32, cur: f32, sel: i32, hover: i32) -> (f32, f32) {
    let track = RectF { x: r.x + 0.5, y: r.y + 0.5, w: r.w - 1.0, h: r.h - 1.0 };
    fill_round(g, track, 10.0 * s, CARD);
    stroke_round(g, track, 10.0 * s, LINE, 1.0, false);

    let pad = 4.0 * s;
    let sw = (r.w - pad * 2.0) * 0.5;
    if hover >= 0 && hover != sel {
        let hr = RectF { x: r.x + pad + (hover as f32) * sw, y: r.y + pad, w: sw, h: r.h - pad * 2.0 };
        fill_round(g, hr, 7.0 * s, SUBTLE);
    }
    let pill = RectF { x: r.x + pad + cur * sw, y: r.y + pad, w: sw, h: r.h - pad * 2.0 };
    fill_round(g, pill, 7.0 * s, ACCENT);
    (pad, sw)
}

unsafe fn paint_seg_action(g: *mut c_void, lay: &Layout, ui: &UiState) {
    let r = lay.seg_action;
    let s = lay.s;
    let (pad, sw) = paint_seg_track(g, r, s, ui.action_cur, ui.action, ui.hover_action);
    let items = [("Scroll wheel", true), ("Numpad + / \u{2212}", false)];
    for (i, &(label, is_mouse)) in items.iter().enumerate() {
        let ir = RectF { x: r.x + pad + (i as f32) * sw, y: r.y + pad, w: sw, h: r.h - pad * 2.0 };
        let near = (1.0 - (i as f32 - ui.action_cur).abs()).max(0.0);
        let fg = mix(INK, ON_ACCENT, near);

        let ih = 17.0 * s;
        let gap = 7.0 * s;
        let iw = if is_mouse { ih * 0.58 + ih * 0.14 + ih * 0.24 } else { ih * 0.92 };
        let (fam, font) = make_font(9.2, true, false);
        let tw = measure_width(g, label, font);
        GdipDeleteFont(font);
        GdipDeleteFontFamily(fam);

        let total = iw + gap + tw;
        let sx = ir.x + (ir.w - total) * 0.5;
        let iy = ir.y + (ir.h - ih) * 0.5;
        if is_mouse {
            draw_mouse(g, sx, iy, ih, fg, fg);
        } else {
            draw_numpad_icon(g, sx, iy, ih, fg);
        }
        draw_text(
            g,
            label,
            9.2,
            true,
            false,
            fg,
            RectF { x: sx + iw + gap, y: ir.y, w: tw + 6.0 * s, h: ir.h },
            0,
        );
    }
}

unsafe fn paint_seg_preset(g: *mut c_void, lay: &Layout, ui: &UiState) {
    let r = lay.seg_preset;
    let s = lay.s;
    let sel = if (ui.use_custom && ui.custom_mods != 0) || ui.recording { 1 } else { 0 };
    let (pad, sw) = paint_seg_track(g, r, s, ui.preset_cur, sel, ui.hover_preset);
    for (i, &label) in ["Default", "Custom"].iter().enumerate() {
        let ir = RectF { x: r.x + pad + (i as f32) * sw, y: r.y + pad, w: sw, h: r.h - pad * 2.0 };
        let near = (1.0 - (i as f32 - ui.preset_cur).abs()).max(0.0);
        let fg = mix(INK, ON_ACCENT, near);
        draw_text(g, label, 9.5, true, false, fg, ir, 1);
    }
}

unsafe fn paint_toggle_row(g: *mut c_void, r: RectF, s: f32, label: &str, k: f32, hover: bool) {
    if hover {
        fill_round(g, r, 9.0 * s, SUBTLE);
    }
    draw_text(
        g,
        label,
        9.5,
        true,
        false,
        INK,
        RectF { x: r.x + 6.0 * s, y: r.y, w: r.w - 64.0 * s, h: r.h },
        0,
    );

    let tw = 44.0 * s;
    let th = 24.0 * s;
    let tx = r.x + r.w - tw - 4.0 * s;
    let ty = r.y + (r.h - th) * 0.5;
    let tr = RectF { x: tx, y: ty, w: tw, h: th };
    let track_fill = mix(CARD, ACCENT, k);
    let track_edge = mix(if hover { LINE_HOVER } else { LINE }, ACCENT, k);
    fill_round(g, tr, th * 0.5, track_fill);
    stroke_round(g, tr, th * 0.5, track_edge, 1.0, false);

    let inset = 3.0 * s;
    let d = th - inset * 2.0;
    let kx = tx + inset + (tw - d - inset * 2.0) * k;
    let ky = ty + inset;
    let mut kb = ptr::null_mut();
    GdipCreateSolidFill(mix(MUTED, ON_ACCENT, k), &mut kb);
    GdipFillEllipse(g, kb, kx, ky, d, d);
    GdipDeleteBrush(kb);
}
