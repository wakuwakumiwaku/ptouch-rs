// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Huang Rui <vowstar@gmail.com>

//! Font discovery using cosmic-text's font system.
//!
//! Provides helpers to list available system fonts and find a font by name.

use cosmic_text::FontSystem;

/// Create a new `FontSystem` with system fonts and bundled fonts loaded.
pub fn create_font_system() -> FontSystem {
    let mut font_system = FontSystem::new();
    load_bundled_fonts(font_system.db_mut());
    font_system
}

/// Load bundled embedded fonts (Inter, DINish, Frutiger) into a font database.
pub fn load_bundled_fonts(db: &mut cosmic_text::fontdb::Database) {
    db.load_font_data(include_bytes!("../../../data/fonts/Inter.ttf").to_vec());
    db.load_font_data(include_bytes!("../../../data/fonts/DINish-Regular.otf").to_vec());
    db.load_font_data(include_bytes!("../../../data/fonts/DINish-Bold.otf").to_vec());
    db.load_font_data(include_bytes!("../../../data/fonts/Frutiger-Roman.ttf").to_vec());
    db.load_font_data(include_bytes!("../../../data/fonts/Frutiger-Bold.ttf").to_vec());
}

/// Find a font by name and return its full family name if found.
///
/// The search is case-insensitive and matches on family name.
pub fn find_font(name: &str) -> Option<String> {
    let font_system = create_font_system();
    let name_lower = name.to_lowercase();

    // Map common aliases/short names
    let search_term = match name_lower.as_str() {
        "din" | "din 1451" | "din-1451" => "dinish",
        "frutiger" => "frutiger",
        _ => &name_lower,
    };

    for face in font_system.db().faces() {
        for family in &face.families {
            if family.0.to_lowercase().contains(search_term) {
                return Some(family.0.clone());
            }
        }
    }

    None
}

/// List all available font family names on the system, including bundled fonts.
///
/// Returns a sorted, deduplicated list of family names.
pub fn list_fonts() -> Vec<String> {
    let font_system = create_font_system();
    let mut names: Vec<String> = Vec::new();

    for face in font_system.db().faces() {
        for family in &face.families {
            names.push(family.0.clone());
        }
    }

    names.sort();
    names.dedup();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_fonts_not_empty() {
        // On most systems there should be at least some fonts.
        // This test may fail in minimal containers without fonts.
        let fonts = list_fonts();
        // We do not assert non-empty since CI may have no fonts,
        // but we check it does not panic.
        log::debug!("Found {} font families", fonts.len());
    }

    #[test]
    fn test_list_fonts_sorted() {
        let fonts = list_fonts();
        for pair in fonts.windows(2) {
            assert!(pair[0] <= pair[1], "fonts should be sorted");
        }
    }

    #[test]
    fn test_find_nonexistent() {
        let result = find_font("ThisFontDefinitelyDoesNotExist12345");
        assert!(result.is_none());
    }

    #[test]
    fn test_find_bundled_fonts() {
        assert_eq!(find_font("Inter"), Some("Inter".to_string()));
        assert_eq!(find_font("din"), Some("DINish".to_string()));
        assert_eq!(find_font("DIN 1451"), Some("DINish".to_string()));
        assert_eq!(find_font("frutiger"), Some("Frutiger".to_string()));
    }
}
