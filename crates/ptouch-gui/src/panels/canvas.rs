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
            let single_mm = texture.size_vec2().x / px_per_mm;
            let total_mm = single_mm * copies as f32;

            if copies > 1 {
                ui.label(
                    egui::RichText::new(format!(
                        "Label: {:.1} mm (Total: {:.1} mm for {} copies)",
                        single_mm, total_mm, copies
                    ))
                    .strong(),
                );
            } else {
                ui.label(
                    egui::RichText::new(format!("Label Length: {:.1} mm", single_mm)).strong(),
                );
            }

            ui.separator();

            let (cut_text, cut_color) = if !state.auto_cut {
                ("✂ Cuts: 0 (Continuous Chain)", egui::Color32::from_rgb(0, 160, 90))
            } else if state.precut {
                ("✂ Cuts: 2 (Pre-Cut + Final Cut)", egui::Color32::from_rgb(220, 50, 50))
            } else {
                ("✂ Cuts: 1 (Final Cut)", egui::Color32::from_rgb(60, 140, 240))
            };

            ui.label(egui::RichText::new(cut_text).color(cut_color));
        }
    });

    ui.separator();

    // Canvas area
    let canvas_rect = ui.available_rect_before_wrap();
    let canvas_size = canvas_rect.size();

    // Natural background matching the application theme
    let bg_color = ui.visuals().extreme_bg_color;
    ui.painter().rect_filled(canvas_rect, 0.0, bg_color);

    match state.preview_texture {
        Some(ref texture) => {
            let tex_size = texture.size_vec2();
            let copies = state.copies.max(1);

            let content_w_px = tex_size.x;
            let content_h_px = tex_size.y;
            let tape_physical_h_px = (state.tape_width_mm as f32 * px_per_mm).max(content_h_px);

            // Total printed strip width (across all copies)
            let total_strip_w_px = content_w_px * copies as f32;

            // Calculate zoom
            let zoom = if state.zoom_fit {
                let margin_x = 80.0;
                let margin_y = 110.0;
                let zoom_x = (canvas_size.x - margin_x) / total_strip_w_px.max(1.0);
                let zoom_y = (canvas_size.y - margin_y) / (tape_physical_h_px + 50.0).max(1.0);
                let fit_zoom = zoom_x.min(zoom_y).clamp(0.05, 10.0);
                state.zoom = fit_zoom;
                fit_zoom
            } else {
                state.zoom
            };

            let display_strip_w = total_strip_w_px * zoom;
            let display_tape_h = tape_physical_h_px * zoom;

            // Center the entire label tape in the canvas
            let center = canvas_rect.center();
            let tape_rect = egui::Rect::from_center_size(
                center,
                egui::vec2(display_strip_w, display_tape_h),
            );

            let painter = ui.painter();

            // 1. Draw tape shadow and physical tape body
            let shadow_rect = tape_rect.translate(egui::vec2(2.0, 3.0));
            painter.rect_filled(shadow_rect, 3.0, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 45));
            painter.rect_filled(tape_rect, 3.0, egui::Color32::WHITE);
            painter.rect_stroke(
                tape_rect,
                3.0,
                egui::Stroke::new(1.0, egui::Color32::from_gray(180)),
                egui::StrokeKind::Outside,
            );

            // 2. Printable vertical guideline margins if tape is taller than content
            let printable_h = content_h_px * zoom;
            let printable_top = tape_rect.center().y - printable_h / 2.0;
            let printable_bottom = printable_top + printable_h;
            if tape_physical_h_px > content_h_px {
                let guideline_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(230));
                painter.line_segment(
                    [egui::pos2(tape_rect.min.x, printable_top), egui::pos2(tape_rect.max.x, printable_top)],
                    guideline_stroke,
                );
                painter.line_segment(
                    [egui::pos2(tape_rect.min.x, printable_bottom), egui::pos2(tape_rect.max.x, printable_bottom)],
                    guideline_stroke,
                );
            }

            let y_top = tape_rect.min.y;
            let y_bottom = tape_rect.max.y;
            let ruler_y = y_top - 28.0;

            // 3. Pre-cut indicator at start (only if precut is active)
            if state.precut && state.auto_cut {
                draw_cut_marker(painter, tape_rect.min.x, y_top, y_bottom, "✂ PRE-CUT", true);
            }

            // 4. Render copies of label
            let single_content_w = content_w_px * zoom;
            let single_content_mm = content_w_px / px_per_mm;
            let mut cur_x = tape_rect.min.x;

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

                if copies == 1 {
                    draw_dimension(
                        painter,
                        copy_start_x,
                        copy_start_x + single_content_w,
                        ruler_y,
                        &format!("{:.1} mm", single_content_mm),
                        egui::Color32::from_rgb(80, 150, 240),
                    );
                }

                cur_x += single_content_w;

                // Between copies: explicit NO CUT chained marker
                if copies > 1 && copy_idx + 1 < copies {
                    draw_no_cut_marker(painter, cur_x, y_top, y_bottom, "⛓ Chained (0 mm)");
                }
            }

            if copies > 1 {
                draw_dimension(
                    painter,
                    tape_rect.min.x,
                    tape_rect.max.x,
                    ruler_y,
                    &format!("{:.1} mm ({} copies x {:.1} mm)", single_content_mm * copies as f32, copies, single_content_mm),
                    egui::Color32::from_rgb(80, 150, 240),
                );
            }

            // 5. End Cut Indicator
            if state.auto_cut {
                draw_cut_marker(painter, tape_rect.max.x, y_top, y_bottom, "✂ CUT", true);
            } else {
                draw_continuous_indicator(painter, tape_rect.max.x, y_top, y_bottom);
            }

            // 6. Draw Tape Width Callout
            painter.text(
                egui::pos2(tape_rect.min.x - 10.0, tape_rect.center().y),
                egui::Align2::RIGHT_CENTER,
                format!("{} mm", state.tape_width_mm),
                egui::FontId::proportional(11.0),
                ui.visuals().weak_text_color(),
            );

            // 7. Explanatory bottom legend
            let legend_y = y_bottom + 30.0;
            let legend_text = if !state.auto_cut {
                "💡 Chain Mode: Continuous printing with 0 mm gap between labels. Click 'Feed & Cut' to cut."
            } else if state.precut {
                "💡 Pre-Cut Mode: Cuts before and after label for symmetrical tape margins."
            } else {
                "💡 Standard Auto-Cut: Prints label and cuts once at the end. Zero scrap snippets."
            };
            painter.text(
                egui::pos2(center.x, legend_y),
                egui::Align2::CENTER_TOP,
                legend_text,
                egui::FontId::proportional(11.0),
                ui.visuals().weak_text_color(),
            );

            ui.allocate_rect(canvas_rect, egui::Sense::hover());
        }
        None => {
            let center = canvas_rect.center();
            ui.painter().text(
                center,
                egui::Align2::CENTER_CENTER,
                "Add elements to design your label",
                egui::FontId::proportional(16.0),
                ui.visuals().weak_text_color(),
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
        egui::Color32::from_rgb(230, 50, 50) // Vivid Red for CUT
    } else {
        egui::Color32::from_gray(140)
    };
    let stroke = egui::Stroke::new(2.0, color);

    draw_dashed_v_line(painter, x, y_top - 16.0, y_bottom + 16.0, stroke, 5.0, 3.0);

    let font_id = egui::FontId::proportional(11.0);
    let badge_pos = egui::pos2(x, y_top - 18.0);
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
    let color = egui::Color32::from_rgb(50, 130, 220); // Blue
    let stroke = egui::Stroke::new(1.0, color);

    draw_dashed_v_line(painter, x, y_top - 10.0, y_bottom + 10.0, stroke, 3.0, 3.0);

    let font_id = egui::FontId::proportional(10.0);
    let badge_pos = egui::pos2(x, y_top - 14.0);
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
    let color = egui::Color32::from_rgb(0, 160, 90); // Green
    let font_id = egui::FontId::proportional(11.0);
    let label = "➔ CONTINUOUS (NO CUT)";

    let stroke = egui::Stroke::new(2.0, color);
    draw_dashed_v_line(painter, x, y_top - 16.0, y_bottom + 16.0, stroke, 4.0, 3.0);

    let badge_pos = egui::pos2(x + 8.0, y_top - 18.0);
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
        egui::FontId::proportional(11.0),
        color,
    );
}
