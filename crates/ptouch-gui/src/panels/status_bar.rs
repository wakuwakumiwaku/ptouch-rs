// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Huang Rui <vowstar@gmail.com>

//! Bottom status bar panel.

use crate::state::AppState;

/// Render the bottom status bar.
pub fn show_status_bar(ui: &mut egui::Ui, state: &AppState) {
    ui.horizontal(|ui| {
        // Printer status
        let printer = state.printer_model.as_deref().unwrap_or("--");
        ui.label(format!("Printer: {}", printer));

        ui.separator();

        // Tape info & cartridge remaining
        if let Some(total_m) = state.cartridge_total_length_m {
            let remaining = (total_m - state.tape_printed_meters).max(0.0);
            ui.label(format!(
                "Tape: {} mm | Cartridge: {:.2}m rem / {:.1}m",
                state.tape_width_mm, remaining, total_m
            ));
        } else {
            ui.label(format!("Tape: {} mm", state.tape_width_mm));
        }

        ui.separator();

        // Label dimensions
        if let Some(ref bmp) = state.preview_bitmap {
            ui.label(format!("{}x{} px", bmp.width(), bmp.height()));
        } else {
            ui.label("-- x -- px");
        }

        ui.separator();

        // Zoom level
        ui.label(format!("Zoom: {:.0}%", state.zoom * 100.0));

        ui.separator();

        // Status message (right-aligned)
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(&state.status_message);
        });
    });
}
