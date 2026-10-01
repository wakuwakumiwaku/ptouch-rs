// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Huang Rui <vowstar@gmail.com>

//! Custom egui widgets and shared helpers.

use std::path::PathBuf;

/// Build an image file dialog with per-format filters.
///
/// The first filter is "All Images" (default), followed by individual
/// format groups so the user can narrow down if needed.
pub fn image_file_dialog() -> rfd::FileDialog {
    rfd::FileDialog::new()
        .add_filter(
            "All Images",
            &[
                "png", "jpg", "jpeg", "gif", "bmp", "tiff", "tif", "webp", "ico", "pnm", "tga",
                "qoi", "svg", "svgz",
            ],
        )
        .add_filter("PNG", &["png"])
        .add_filter("JPEG", &["jpg", "jpeg"])
        .add_filter("SVG", &["svg", "svgz"])
        .add_filter("GIF", &["gif"])
        .add_filter("BMP", &["bmp"])
        .add_filter("TIFF", &["tiff", "tif"])
        .add_filter("WebP", &["webp"])
        .add_filter("ICO", &["ico"])
        .add_filter("QOI", &["qoi"])
        .add_filter("PNM", &["pnm"])
        .add_filter("TGA", &["tga"])
}

/// Pick an image file with fallback to zenity and kdialog.
#[allow(clippy::collapsible_if)]
pub fn pick_image_file() -> Option<PathBuf> {
    if let Some(path) = image_file_dialog().pick_file() {
        return Some(path);
    }
    if let Ok(output) = std::process::Command::new("zenity")
        .args([
            "--file-selection",
            "--title=Choose Image",
            "--file-filter=Images | *.png *.jpg *.jpeg *.svg *.bmp *.gif *.webp",
            "--file-filter=All Files | *",
        ])
        .output()
    {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !s.is_empty() {
                return Some(PathBuf::from(s));
            }
        }
    }
    if let Ok(output) = std::process::Command::new("kdialog")
        .args([
            "--getopenfilename",
            ".",
            "*.png *.jpg *.jpeg *.svg *.bmp *.gif *.webp|Images",
        ])
        .output()
    {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !s.is_empty() {
                return Some(PathBuf::from(s));
            }
        }
    }
    None
}

/// Save an image file with fallback to zenity and kdialog.
#[allow(clippy::collapsible_if)]
pub fn save_image_file() -> Option<PathBuf> {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("PNG", &["png"])
        .add_filter("JPEG", &["jpg", "jpeg"])
        .add_filter("BMP", &["bmp"])
        .add_filter("GIF", &["gif"])
        .add_filter("TIFF", &["tiff", "tif"])
        .add_filter("WebP", &["webp"])
        .set_file_name("label.png")
        .save_file()
    {
        return Some(path);
    }
    if let Ok(output) = std::process::Command::new("zenity")
        .args([
            "--file-selection",
            "--save",
            "--confirm-overwrite",
            "--title=Export Image",
            "--filename=label.png",
            "--file-filter=PNG (*.png) | *.png",
            "--file-filter=JPEG (*.jpg) | *.jpg",
        ])
        .output()
    {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !s.is_empty() {
                let mut p = PathBuf::from(s);
                if p.extension().is_none() {
                    p = p.with_extension("png");
                }
                return Some(p);
            }
        }
    }
    if let Ok(output) = std::process::Command::new("kdialog")
        .args(["--getsavefilename", "label.png", "*.png|PNG Image (*.png)"])
        .output()
    {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !s.is_empty() {
                let mut p = PathBuf::from(s);
                if p.extension().is_none() {
                    p = p.with_extension("png");
                }
                return Some(p);
            }
        }
    }
    None
}

/// Build a file dialog filtered to layout files (`.ptl`).
pub fn layout_file_dialog() -> rfd::FileDialog {
    rfd::FileDialog::new().add_filter("Layout", &["ptl"])
}

/// Pick a layout file (`.ptl`) with fallback to zenity and kdialog.
pub fn pick_layout_file() -> Option<PathBuf> {
    if let Some(path) = layout_file_dialog().pick_file() {
        return Some(path);
    }
    if let Ok(output) = std::process::Command::new("zenity")
        .args([
            "--file-selection",
            "--title=Open Layout",
            "--file-filter=Layout (*.ptl) | *.ptl",
            "--file-filter=All Files | *",
        ])
        .output()
        && output.status.success()
    {
        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !s.is_empty() {
            return Some(PathBuf::from(s));
        }
    }
    if let Ok(output) = std::process::Command::new("kdialog")
        .args(["--getopenfilename", ".", "*.ptl|Layout files (*.ptl)"])
        .output()
        && output.status.success()
    {
        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !s.is_empty() {
            return Some(PathBuf::from(s));
        }
    }
    None
}

/// Save a layout file (`.ptl`) with fallback to zenity and kdialog.
pub fn save_layout_file() -> Option<PathBuf> {
    if let Some(path) = layout_file_dialog().set_file_name("label.ptl").save_file() {
        return Some(path);
    }
    if let Ok(output) = std::process::Command::new("zenity")
        .args([
            "--file-selection",
            "--save",
            "--confirm-overwrite",
            "--title=Save Layout",
            "--filename=label.ptl",
            "--file-filter=Layout (*.ptl) | *.ptl",
        ])
        .output()
        && output.status.success()
    {
        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !s.is_empty() {
            let mut p = PathBuf::from(s);
            if p.extension().is_none() {
                p = p.with_extension("ptl");
            }
            return Some(p);
        }
    }
    if let Ok(output) = std::process::Command::new("kdialog")
        .args([
            "--getsavefilename",
            "label.ptl",
            "*.ptl|Layout files (*.ptl)",
        ])
        .output()
        && output.status.success()
    {
        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !s.is_empty() {
            let mut p = PathBuf::from(s);
            if p.extension().is_none() {
                p = p.with_extension("ptl");
            }
            return Some(p);
        }
    }
    None
}

/// iOS-style toggle switch widget.
///
/// Based on the egui demo toggle_switch example.
#[allow(dead_code)]
pub fn toggle(on: &mut bool) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| toggle_ui(ui, on)
}

#[allow(dead_code)]
fn toggle_ui(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired_size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, "")
    });

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
        let visuals = ui.style().interact_selectable(&response, *on);
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter().rect(
            rect,
            radius,
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter()
            .circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
    }

    response
}
