// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Huang Rui <vowstar@gmail.com>

//! Top toolbar panel with element addition and action buttons.

use std::path::PathBuf;

use log::{error, info};

use ptouch_render::document::LabelDocument;
use ptouch_render::raster;
use ptouch_render::text::TextAlign;

use crate::state::{AppState, LabelElement, PrinterCommand};

/// Render the top toolbar.
pub fn show_toolbar(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        // -- Element addition buttons --
        if ui.button("Add Text").clicked() {
            state.elements.push(LabelElement::Text {
                content: "Label".to_string(),
                font_size: None,
                align: TextAlign::Left,
                rotation: 0.0,
                flip_h: false,
                flip_v: false,
            });
            state.selected_element = Some(state.elements.len() - 1);
            state.mark_dirty();
            info!("Added text element");
        }

        if ui.button("Add Image").clicked()
            && let Some(path) = crate::widgets::pick_image_file()
        {
            // Read the original source bytes so the image is embedded in the
            // label and stays self-contained when saved to a layout file.
            match std::fs::read(&path) {
                Ok(bytes) => {
                    let element = LabelElement::image_from_bytes(Some(path.clone()), bytes);
                    // Reject files that do not decode, so a saved layout can
                    // always be reopened.
                    if matches!(element, LabelElement::Image { bitmap: None, .. }) {
                        error!("Unsupported or corrupt image: {}", path.display());
                        state.status_message = format!("Image load error: {}", path.display());
                    } else {
                        info!("Loaded image: {}", path.display());
                        state.elements.push(element);
                        state.selected_element = Some(state.elements.len() - 1);
                        state.mark_dirty();
                    }
                }
                Err(e) => {
                    error!("Failed to read image: {}", e);
                    state.status_message = format!("Image read error: {}", e);
                }
            }
        }

        if ui.button("Cut Mark").clicked() {
            state.elements.push(LabelElement::CutMark);
            state.selected_element = Some(state.elements.len() - 1);
            state.mark_dirty();
            info!("Added cut mark");
        }

        if ui.button("Padding").clicked() {
            state.elements.push(LabelElement::Padding { pixels: 20 });
            state.selected_element = Some(state.elements.len() - 1);
            state.mark_dirty();
            info!("Added padding element");
        }

        ui.separator();

        // -- Action buttons --
        let connected = state.printer_connected;
        let busy = state.is_printer_busy();
        let has_bitmap = state.preview_bitmap.is_some() && !state.needs_rerender;

        if ui
            .add_enabled(connected && !busy && has_bitmap, egui::Button::new("Print"))
            .clicked()
            && let Some(ref bitmap) = state.preview_bitmap
        {
            let raw_lines = raster::bitmap_to_raster_lines(bitmap, state.printer_max_px);
            let dpi = if state.printer_dpi > 0 { state.printer_dpi as f32 } else { 180.0 };
            let px_per_mm = dpi / 25.4;
            let copies = state.copies.max(1);

            let blank_line = vec![0u8; (state.printer_max_px as usize).div_ceil(8)];

            let (raster_lines, chain_print, precut) = match state.cut_mode {
                crate::state::CutMarginMode::CenteredFull => {
                    // Symmetrical safety margins: add safe blank lines BEFORE the text
                    // (preventing the first letters from getting cut off)
                    // and matching trailing lines AFTER the text so it's perfectly centered!
                    let margin_lines = (state.centered_margin_mm * px_per_mm).round() as usize;
                    let mut lines = Vec::with_capacity(margin_lines * 2 + raw_lines.len());
                    lines.extend(std::iter::repeat(blank_line.clone()).take(margin_lines));
                    lines.extend(raw_lines);
                    lines.extend(std::iter::repeat(blank_line).take(margin_lines));
                    (lines, false, false)
                }
                crate::state::CutMarginMode::PretrimCut => {
                    // Pre-cut trims the 24.5 mm scrap first.
                    // Add safe margins (default 3mm) so text is never cut off.
                    let margin_lines = (state.small_margin_mm * px_per_mm).round() as usize;
                    let mut lines = Vec::with_capacity(margin_lines * 2 + raw_lines.len());
                    lines.extend(std::iter::repeat(blank_line.clone()).take(margin_lines));
                    lines.extend(raw_lines);
                    lines.extend(std::iter::repeat(blank_line).take(margin_lines));
                    (lines, false, true)
                }
                crate::state::CutMarginMode::ChainPrint => {
                    let margin_lines = (2.0 * px_per_mm).round() as usize;
                    let mut lines = Vec::with_capacity(margin_lines * 2 + raw_lines.len());
                    lines.extend(std::iter::repeat(blank_line.clone()).take(margin_lines));
                    lines.extend(raw_lines);
                    lines.extend(std::iter::repeat(blank_line).take(margin_lines));
                    (lines, true, false)
                }
            };

            // Track cumulative tape consumption
            let total_job_mm = (raster_lines.len() as f32 / px_per_mm) * copies as f32;
            state.tape_printed_meters += total_job_mm / 1000.0;

            if let Some(ref tx) = state.printer_cmd_tx {
                let _ = tx.send(PrinterCommand::Print {
                    raster_lines,
                    chain_print,
                    precut,
                    copies,
                    quality: state.print_quality,
                    target: state.printer_target.clone(),
                });
                state.operation_in_progress = true;
                state.status_message = if copies > 1 {
                    format!("Printing {copies} copies...")
                } else {
                    "Printing...".to_string()
                };
            }
        }

        if ui
            .add_enabled(
                connected && !busy && !state.printer_target.is_bluetooth(),
                egui::Button::new("Feed & Cut"),
            )
            .clicked()
            && let Some(ref tx) = state.printer_cmd_tx
        {
            let _ = tx.send(PrinterCommand::FeedAndCut(state.printer_target.clone()));
            state.operation_in_progress = true;
            state.status_message = "Feeding & cutting...".to_string();
        }

        let tape_btn_label = if let Some(total_m) = state.cartridge_total_length_m {
            let rem = (total_m - state.tape_printed_meters).max(0.0);
            format!("🏷 Tape: {} mm ({:.1}m rem)", state.tape_width_mm, rem)
        } else {
            format!("🏷 Tape: {} mm", state.tape_width_mm)
        };
        if ui.button(tape_btn_label).clicked() {
            state.show_setup_modal = true;
        }

        if ui.button("Export Image").clicked() {
            do_export_image(state);
        }

        ui.separator();

        if ui.button("Save Layout").clicked() {
            do_save_layout(state);
        }

        if ui.button("Open Layout").clicked() {
            do_open_layout(state);
        }
    });
}

/// Save the current design to a `.ptl` layout file (TOML with embedded images).
fn do_save_layout(state: &mut AppState) {
    if state.elements.is_empty() {
        state.status_message = "Nothing to save".to_string();
        return;
    }

    let document = LabelDocument {
        version: ptouch_render::document::DOCUMENT_VERSION,
        tape_width_mm: state.tape_width_mm,
        dpi: state.printer_dpi,
        font_name: state.font_name.clone(),
        font_margin: state.font_margin,
        flip_h: state.overall_flip_h,
        flip_v: state.overall_flip_v,
        elements: state.elements.clone(),
    };

    let text = match document.to_toml_string() {
        Ok(text) => text,
        Err(e) => {
            state.status_message = format!("Save error: {}", e);
            error!("Layout serialize error: {}", e);
            return;
        }
    };

    if let Some(path) = crate::widgets::save_layout_file() {
        let save_path: PathBuf = if path.extension().is_none() {
            path.with_extension("ptl")
        } else {
            path
        };
        match std::fs::write(&save_path, text) {
            Ok(()) => {
                state.status_message = format!("Saved to {}", save_path.display());
                info!("Saved layout: {}", save_path.display());
            }
            Err(e) => {
                state.status_message = format!("Save error: {}", e);
                error!("Layout write error: {}", e);
            }
        }
    }
}

/// Open a `.ptl` layout file, replacing the current design.
fn do_open_layout(state: &mut AppState) {
    let Some(path) = crate::widgets::pick_layout_file() else {
        return;
    };

    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) => {
            state.status_message = format!("Open error: {}", e);
            error!("Layout read error: {}", e);
            return;
        }
    };

    match LabelDocument::from_toml_str(&text) {
        Ok(document) => {
            apply_layout(state, document);
            state.status_message = format!("Opened {}", path.display());
            info!("Opened layout: {}", path.display());
        }
        Err(e) => {
            state.status_message = format!("Open error: {}", e);
            error!("Layout parse error: {}", e);
        }
    }
}

fn apply_layout(state: &mut AppState, document: LabelDocument) {
    if !state.printer_target.is_bluetooth() {
        state.tape_width_mm = document.tape_width_mm;
        state.update_tape_pixels();
    }
    state.font_name = document.font_name;
    state.font_margin = document.font_margin;
    state.overall_flip_h = document.flip_h;
    state.overall_flip_v = document.flip_v;
    state.elements = document.elements;
    state.selected_element = None;
    state.mark_dirty();
}

/// Export the current label preview as an image file.
fn do_export_image(state: &mut AppState) {
    let bitmap = match state.preview_bitmap {
        Some(ref bmp) => bmp,
        None => {
            state.status_message = "Nothing to export".to_string();
            return;
        }
    };

    if let Some(path) = crate::widgets::save_image_file() {
        let save_path: PathBuf = if path.extension().is_none() {
            path.with_extension("png")
        } else {
            path
        };
        match bitmap.save(&save_path) {
            Ok(()) => {
                state.status_message = format!("Saved to {}", save_path.display());
                info!("Exported image: {}", save_path.display());
            }
            Err(e) => {
                state.status_message = format!("Save error: {}", e);
                error!("Image save error: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::PrinterTarget;
    use ptouch_render::text::TextRenderer;

    fn layout(tape_width_mm: u8) -> LabelDocument {
        let document = LabelDocument {
            version: ptouch_render::document::DOCUMENT_VERSION,
            tape_width_mm,
            dpi: 180,
            font_name: "DejaVuSans".into(),
            font_margin: 0,
            flip_h: false,
            flip_v: false,
            elements: vec![LabelElement::CutMark],
        };
        LabelDocument::from_toml_str(&document.to_toml_string().unwrap()).unwrap()
    }

    #[test]
    fn opening_layout_keeps_bluetooth_tape_geometry() {
        for width in [12, 24] {
            let mut state = AppState {
                printer_target: PrinterTarget::Bluetooth {
                    name: "PT-P300BT".into(),
                    address: "AA:BB:CC:DD:EE:FF".into(),
                },
                printer_connected: true,
                printer_max_px: 128,
                tape_width_mm: 12,
                tape_width_px: 64,
                ..AppState::default()
            };
            apply_layout(&mut state, layout(width));
            assert_eq!((state.tape_width_mm, state.tape_width_px), (12, 64));
            let bitmap = ptouch_render::document::render_elements(
                &state.elements,
                state.tape_width_px,
                &state.font_name,
                state.font_margin,
                &mut TextRenderer::new(),
            )
            .unwrap()
            .unwrap();
            let lines = raster::bitmap_to_raster_lines(&bitmap, state.printer_max_px);
            assert!(
                lines
                    .iter()
                    .all(|line| line[..4].iter().chain(&line[12..]).all(|&byte| byte == 0))
            );
        }
    }

    #[test]
    fn opening_usb_layout_uses_saved_tape_width() {
        let mut state = AppState {
            tape_width_mm: 24,
            tape_width_px: 128,
            ..AppState::default()
        };
        apply_layout(&mut state, layout(12));
        assert_eq!((state.tape_width_mm, state.tape_width_px), (12, 76));
    }
}
