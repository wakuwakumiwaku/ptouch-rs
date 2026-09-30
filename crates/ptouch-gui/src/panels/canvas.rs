// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Huang Rui <vowstar@gmail.com>

//! Central canvas panel for label preview display, full tape geometry, and cut indicators.

use crate::state::AppState;

/// Render the central preview canvas.
pub fn show_canvas(ui: &mut egui::Ui, state: &mut AppState) {
    let dpi = if state.printer_dpi > 0 { state.printer_dpi as f32 } else { 180.0 };
    let px_per_mm = dpi / 25.4;

    // Zoom controls and tape overview header
    ui.horizontal(|ui| {
        if ui.button("Fit").clicked() {
            state.zoom_fit = true;
        }
        if ui.button("1:1").clicked() {
            state.zoom = 1.0;
            state.zoom_fit = false;
        }
        if ui.button("+").clicked() {
            state.zoom = (state.zoom * 1.25).min(20.0);
            state.zoom_fit = false;
        }
        if ui.button("-").clicked() {
            state.zoom = (state.zoom / 1.25).max(0.1);
            state.zoom_fit = false;
        }
        ui.label(format!("Zoom: {:.0}%", state.zoom * 100.0));

        ui.separator();

        if let Some(ref texture) = state.preview_texture {
            let copies = state.copies.max(1);
            let content_mm = (texture.size_vec2().x / px_per_mm) * copies as f32;
            let lead_mm: f32 = 24.5;
            let trail_mm: f32 = if state.auto_cut { 2.0 } else { 0.0 };
            let total_tape_mm = if state.precut && state.auto_cut {
                lead_mm + 2.0 + content_mm + trail_mm
            } else if state.auto_cut {
                lead_mm + content_mm + trail_mm
            } else {
                lead_mm + content_mm
            };

            ui.label(
                egui::RichText::new(format!("Total Tape: {:.1} mm", total_tape_mm))
                    .strong(),
            );

            ui.separator();

            let (cut_text, cut_color) = if !state.auto_cut {
                ("✂ Cuts: 0 (Chain Print - No Cuts)", egui::Color32::from_rgb(0, 140, 80))
            } else if state.precut {
                ("✂ Cuts: 2 (Pre-Cut Scrap + Final Cut)", egui::Color32::from_rgb(200, 40, 40))
            } else {
                ("✂ Cuts: 1 (Final Cut Only - No Waste Scrap)", egui::Color32::from_rgb(30, 100, 200))
            };

            ui.label(egui::RichText::new(cut_text).color(cut_color));
        }
    });

    ui.separator();

    // Canvas area
    let canvas_rect = ui.available_rect_before_wrap();
    let canvas_size = canvas_rect.size();

    // Fill the background with neutral canvas gray
    ui.painter()
        .rect_filled(canvas_rect, 0.0, egui::Color32::from_gray(215));

    match state.preview_texture {
        Some(ref texture) => {
            let tex_size = texture.size_vec2();
            let copies = state.copies.max(1);

            let content_w_px = tex_size.x;
            let content_h_px = tex_size.y;

            let lead_mm: f32 = 24.5;
            let lead_w_px = (lead_mm * px_per_mm).round();
            let trail_mm: f32 = if state.auto_cut { 2.0 } else { 0.0 };
            let trail_w_px = (trail_mm * px_per_mm).round();
            let margin_precut_mm: f32 = 2.0;
            let margin_precut_px = (margin_precut_mm * px_per_mm).round();

            let tape_physical_h_px = (state.tape_width_mm as f32 * px_per_mm).max(content_h_px);

            // Total strip width in printer pixels (unscaled)
            let total_strip_w_px = if state.precut && state.auto_cut {
                lead_w_px + margin_precut_px + (content_w_px * copies as f32) + trail_w_px
            } else if state.auto_cut {
                lead_w_px + (content_w_px * copies as f32) + trail_w_px
            } else {
                lead_w_px + (content_w_px * copies as f32) + (15.0 * px_per_mm).round()
            };

            // Calculate zoom
            let zoom = if state.zoom_fit {
                let margin_x = 80.0;
                let margin_y = 130.0;
                let zoom_x = (canvas_size.x - margin_x) / total_strip_w_px;
                let zoom_y = (canvas_size.y - margin_y) / (tape_physical_h_px + 60.0);
                let fit_zoom = zoom_x.min(zoom_y).clamp(0.05, 10.0);
                state.zoom = fit_zoom;
                fit_zoom
            } else {
                state.zoom
            };

            let display_strip_w = total_strip_w_px * zoom;
            let display_tape_h = tape_physical_h_px * zoom;

            // Center the entire tape strip in the canvas
            let center = canvas_rect.center();
            let tape_rect = egui::Rect::from_center_size(
                center,
                egui::vec2(display_strip_w, display_tape_h),
            );

            let painter = ui.painter();

            // 1. Draw tape shadow and physical tape body
            let shadow_rect = tape_rect.translate(egui::vec2(2.0, 3.0));
            painter.rect_filled(shadow_rect, 4.0, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 35));
            painter.rect_filled(tape_rect, 4.0, egui::Color32::from_rgb(255, 255, 255));
            painter.rect_stroke(
                tape_rect,
                4.0,
                egui::Stroke::new(1.5, egui::Color32::from_gray(160)),
                egui::StrokeKind::Outside,
            );

            // 2. Draw printable vertical band guidelines (centered vertically)
            let printable_h = content_h_px * zoom;
            let printable_top = tape_rect.center().y - printable_h / 2.0;
            let printable_bottom = printable_top + printable_h;
            let guideline_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(230));
            painter.line_segment(
                [egui::pos2(tape_rect.min.x, printable_top), egui::pos2(tape_rect.max.x, printable_top)],
                guideline_stroke,
            );
            painter.line_segment(
                [egui::pos2(tape_rect.min.x, printable_bottom), egui::pos2(tape_rect.max.x, printable_bottom)],
                guideline_stroke,
            );

            let start_x = tape_rect.min.x;
            let mut cur_x = start_x;

            let y_top = tape_rect.min.y;
            let y_bottom = tape_rect.max.y;
            let ruler_y = y_top - 34.0;

            // 3. Render Leading Zone
            if state.precut && state.auto_cut {
                // Scrap area to be cut off
                let scrap_w = lead_w_px * zoom;
                let scrap_rect = egui::Rect::from_min_max(
                    egui::pos2(cur_x, y_top),
                    egui::pos2(cur_x + scrap_w, y_bottom),
                );
                painter.rect_filled(scrap_rect, 0.0, egui::Color32::from_rgb(255, 235, 235));
                draw_stripes(painter, scrap_rect, egui::Color32::from_rgba_unmultiplied(230, 80, 80, 40));

                painter.text(
                    scrap_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Scrap (~25mm)",
                    egui::FontId::proportional(11.0),
                    egui::Color32::from_rgb(180, 40, 40),
                );

                draw_dimension(painter, cur_x, cur_x + scrap_w, ruler_y, &format!("{:.1} mm (Scrap)", lead_mm), egui::Color32::from_rgb(180, 40, 40));

                cur_x += scrap_w;

                // PRE-CUT line
                draw_cut_marker(painter, cur_x, y_top, y_bottom, "✂ PRE-CUT (Scrap cut here)", true);

                // Margin between precut and content
                let margin_w = margin_precut_px * zoom;
                cur_x += margin_w;
            } else if state.auto_cut {
                // Normal auto-cut: leading unprinted hardware gap
                let lead_w = lead_w_px * zoom;
                let lead_rect = egui::Rect::from_min_max(
                    egui::pos2(cur_x, y_top),
                    egui::pos2(cur_x + lead_w, y_bottom),
                );
                painter.rect_filled(lead_rect, 0.0, egui::Color32::from_gray(245));
                draw_stripes(painter, lead_rect, egui::Color32::from_rgba_unmultiplied(160, 160, 160, 25));

                painter.text(
                    lead_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Hardware Lead (~25mm)",
                    egui::FontId::proportional(11.0),
                    egui::Color32::from_gray(120),
                );

                // Start of tape marker (from previous cut)
                draw_cut_marker(painter, cur_x, y_top, y_bottom, "Previous Cut (Tape Start)", false);
                draw_dimension(painter, cur_x, cur_x + lead_w, ruler_y, &format!("{:.1} mm (Lead)", lead_mm), egui::Color32::from_gray(100));

                cur_x += lead_w;
            } else {
                // Chain mode: show lead/start
                let lead_w = (lead_w_px * 0.5) * zoom;
                let lead_rect = egui::Rect::from_min_max(
                    egui::pos2(cur_x, y_top),
                    egui::pos2(cur_x + lead_w, y_bottom),
                );
                painter.rect_filled(lead_rect, 0.0, egui::Color32::from_gray(248));
                painter.text(
                    lead_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Start",
                    egui::FontId::proportional(10.0),
                    egui::Color32::from_gray(140),
                );
                cur_x += lead_w;
            }

            // 4. Render Copies and Elements
            let single_content_w = content_w_px * zoom;
            let single_content_mm = content_w_px / px_per_mm;

            for copy_idx in 0..copies {
                let copy_start_x = cur_x;
                let copy_rect = egui::Rect::from_min_size(
                    egui::pos2(copy_start_x, printable_top),
                    egui::vec2(single_content_w, printable_h),
                );

                // Draw label texture
                painter.image(
                    texture.id(),
                    copy_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );

                draw_dimension(
                    painter,
                    copy_start_x,
                    copy_start_x + single_content_w,
                    ruler_y,
                    &format!("{:.1} mm (Label)", single_content_mm),
                    egui::Color32::from_rgb(40, 40, 160),
                );

                cur_x += single_content_w;

                // Between copies: explicit NO CUT chained marker
                if copies > 1 && copy_idx + 1 < copies {
                    draw_no_cut_marker(painter, cur_x, y_top, y_bottom, "⛓ Chained (No Cut)");
                }
            }

            // 5. Render Trailing Zone and End Cut
            if state.auto_cut {
                let trail_w = trail_w_px * zoom;
                cur_x += trail_w;

                // FINAL CUT line
                draw_cut_marker(painter, cur_x, y_top, y_bottom, "✂ FINAL CUT", true);

                if trail_w > 2.0 {
                    draw_dimension(painter, cur_x - trail_w, cur_x, ruler_y, &format!("{:.1} mm", trail_mm), egui::Color32::from_gray(100));
                }
            } else {
                // Chain print / No cut indicator
                draw_continuous_indicator(painter, cur_x, y_top, y_bottom);
            }

            // 6. Draw Tape Width Callout
            painter.text(
                egui::pos2(tape_rect.min.x - 8.0, tape_rect.center().y),
                egui::Align2::RIGHT_CENTER,
                format!("{} mm", state.tape_width_mm),
                egui::FontId::proportional(11.0),
                egui::Color32::from_gray(100),
            );

            // 7. Explanatory bottom legend
            let legend_y = y_bottom + 38.0;
            let legend_text = if !state.auto_cut {
                "💡 Chain Mode: Continuous printing with 0 mm waste between labels. Click 'Feed & Cut' in the top toolbar to cut the tape."
            } else if state.precut {
                "💡 Pre-Cut Mode: Cuts ~25 mm leader scrap before printing for symmetrical borders. (2 cuts total: Pre-Cut + Final Cut)"
            } else {
                "💡 Standard Auto-Cut: Prints label with hardware lead and cuts once at the end. No scrap snippet is cut off."
            };
            painter.text(
                egui::pos2(center.x, legend_y),
                egui::Align2::CENTER_TOP,
                legend_text,
                egui::FontId::proportional(11.0),
                egui::Color32::from_gray(90),
            );

            // Allocate the space so the panel is not empty
            ui.allocate_rect(canvas_rect, egui::Sense::hover());
        }
        None => {
            // No preview available
            let center = canvas_rect.center();
            ui.painter().text(
                center,
                egui::Align2::CENTER_CENTER,
                "Add elements to preview",
                egui::FontId::proportional(18.0),
                egui::Color32::from_gray(100),
            );
            ui.allocate_rect(canvas_rect, egui::Sense::hover());
        }
    }
}

fn draw_dashed_v_line(
    painter: &egui::Painter,
    x: f32,
    y_min: f32,
    y_max: f32,
    stroke: egui::Stroke,
    dash: f32,
    gap: f32,
) {
    let mut y = y_min;
    while y < y_max {
        let y_end = (y + dash).min(y_max);
        painter.line_segment([egui::pos2(x, y), egui::pos2(x, y_end)], stroke);
        y += dash + gap;
    }
}

fn draw_cut_marker(
    painter: &egui::Painter,
    x: f32,
    y_top: f32,
    y_bottom: f32,
    label: &str,
    is_active_cut: bool,
) {
    let color = if is_active_cut {
        egui::Color32::from_rgb(220, 45, 45) // Vivid Red for CUT
    } else {
        egui::Color32::from_gray(140) // Neutral gray for Previous Cut
    };
    let stroke = egui::Stroke::new(2.0, color);

    draw_dashed_v_line(painter, x, y_top - 20.0, y_bottom + 20.0, stroke, 5.0, 3.0);

    let font_id = egui::FontId::proportional(11.0);
    let badge_pos = egui::pos2(x, y_top - 22.0);
    let text_rect = painter.text(
        badge_pos,
        egui::Align2::CENTER_BOTTOM,
        label,
        font_id.clone(),
        egui::Color32::TRANSPARENT,
    );
    let bg_rect = text_rect.expand2(egui::vec2(6.0, 3.0));
    painter.rect_filled(bg_rect, 3.0, color);
    painter.text(
        badge_pos,
        egui::Align2::CENTER_BOTTOM,
        label,
        font_id,
        egui::Color32::WHITE,
    );
}

fn draw_no_cut_marker(
    painter: &egui::Painter,
    x: f32,
    y_top: f32,
    y_bottom: f32,
    label: &str,
) {
    let color = egui::Color32::from_rgb(40, 120, 200); // Blue
    let stroke = egui::Stroke::new(1.0, color);

    draw_dashed_v_line(painter, x, y_top - 12.0, y_bottom + 12.0, stroke, 3.0, 3.0);

    let font_id = egui::FontId::proportional(10.0);
    let badge_pos = egui::pos2(x, y_top - 16.0);
    let text_rect = painter.text(
        badge_pos,
        egui::Align2::CENTER_BOTTOM,
        label,
        font_id.clone(),
        egui::Color32::TRANSPARENT,
    );
    let bg_rect = text_rect.expand2(egui::vec2(4.0, 2.0));
    painter.rect_filled(bg_rect, 3.0, color);
    painter.text(
        badge_pos,
        egui::Align2::CENTER_BOTTOM,
        label,
        font_id,
        egui::Color32::WHITE,
    );
}

fn draw_continuous_indicator(
    painter: &egui::Painter,
    x: f32,
    y_top: f32,
    y_bottom: f32,
) {
    let color = egui::Color32::from_rgb(0, 150, 80); // Green
    let font_id = egui::FontId::proportional(11.0);
    let label = "➔ CONTINUOUS (NO CUT) ➔";

    let stroke = egui::Stroke::new(2.0, color);
    draw_dashed_v_line(painter, x, y_top - 16.0, y_bottom + 16.0, stroke, 4.0, 3.0);

    let badge_pos = egui::pos2(x + 10.0, y_top - 20.0);
    let text_rect = painter.text(
        badge_pos,
        egui::Align2::LEFT_BOTTOM,
        label,
        font_id.clone(),
        egui::Color32::TRANSPARENT,
    );
    let bg_rect = text_rect.expand2(egui::vec2(6.0, 3.0));
    painter.rect_filled(bg_rect, 3.0, color);
    painter.text(
        badge_pos,
        egui::Align2::LEFT_BOTTOM,
        label,
        font_id,
        egui::Color32::WHITE,
    );
}

fn draw_dimension(
    painter: &egui::Painter,
    x_start: f32,
    x_end: f32,
    y: f32,
    text: &str,
    color: egui::Color32,
) {
    let w = (x_end - x_start).abs();
    if w < 12.0 {
        return;
    }
    let stroke = egui::Stroke::new(1.0, color);
    painter.line_segment([egui::pos2(x_start, y), egui::pos2(x_end, y)], stroke);
    painter.line_segment([egui::pos2(x_start, y - 3.0), egui::pos2(x_start, y + 3.0)], stroke);
    painter.line_segment([egui::pos2(x_end, y - 3.0), egui::pos2(x_end, y + 3.0)], stroke);
    painter.text(
        egui::pos2((x_start + x_end) / 2.0, y - 4.0),
        egui::Align2::CENTER_BOTTOM,
        text,
        egui::FontId::proportional(10.0),
        color,
    );
}

fn draw_stripes(
    painter: &egui::Painter,
    rect: egui::Rect,
    color: egui::Color32,
) {
    let stroke = egui::Stroke::new(1.0, color);
    let step = 10.0;
    let mut x = rect.min.x - rect.height();
    while x < rect.max.x {
        let p1 = egui::pos2(x.max(rect.min.x), rect.max.y);
        let p2 = egui::pos2((x + rect.height()).min(rect.max.x), rect.min.y);
        if p1.x < rect.max.x && p2.x > rect.min.x {
            painter.line_segment([p1, p2], stroke);
        }
        x += step;
    }
}
