//! Suggestion categories for the Home and Search pages.
//!
//! Provides a static list of music genre/mood categories with template queries.
//! Templates containing `{YEAR}` are resolved at runtime with the current year.

use serde::{Deserialize, Serialize};

/// A suggestion category shown as a discoverable card/chip in the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionCategory {
    /// Unique identifier (e.g. "lofi", "reggaeton", "ambient").
    pub id: String,
    /// Display label (e.g. "Lo-fi Beats", "Reggaeton 2026").
    pub label: String,
    /// Lucide icon name for the UI (e.g. "moon", "flame", "waves").
    pub icon: String,
    /// Search query template — may contain `{YEAR}` placeholder.
    pub query: String,
    /// Accent color as a CSS-compatible value (e.g. "#8B5CF6").
    pub color: String,
}

/// Template definitions for suggestion categories.
/// `{YEAR}` placeholders are replaced with the current year at runtime.
fn category_templates() -> Vec<SuggestionCategory> {
    vec![
        SuggestionCategory {
            id: "lofi".into(),
            label: "Lo-fi Beats".into(),
            icon: "moon".into(),
            query: "lo-fi beats to relax".into(),
            color: "#8B5CF6".into(),
        },
        SuggestionCategory {
            id: "ambient".into(),
            label: "Ambient".into(),
            icon: "cloud".into(),
            query: "ambient music".into(),
            color: "#06B6D4".into(),
        },
        SuggestionCategory {
            id: "chillout".into(),
            label: "Chillout".into(),
            icon: "coffee".into(),
            query: "chillout music".into(),
            color: "#10B981".into(),
        },
        SuggestionCategory {
            id: "reggaeton".into(),
            label: "Reggaeton {YEAR}".into(),
            icon: "flame".into(),
            query: "reggaeton {YEAR}".into(),
            color: "#F59E0B".into(),
        },
        SuggestionCategory {
            id: "exitos".into(),
            label: "Éxitos {YEAR}".into(),
            icon: "trophy".into(),
            query: "éxitos {YEAR}".into(),
            color: "#EF4444".into(),
        },
        SuggestionCategory {
            id: "verano".into(),
            label: "Verano {YEAR}".into(),
            icon: "sun".into(),
            query: "música verano {YEAR}".into(),
            color: "#F97316".into(),
        },
        SuggestionCategory {
            id: "dance".into(),
            label: "Dance {YEAR}".into(),
            icon: "music".into(),
            query: "dance hits {YEAR}".into(),
            color: "#EC4899".into(),
        },
        SuggestionCategory {
            id: "top-hits".into(),
            label: "Top Hits {YEAR}".into(),
            icon: "trending-up".into(),
            query: "top hits {YEAR}".into(),
            color: "#6366F1".into(),
        },
        SuggestionCategory {
            id: "electronic".into(),
            label: "Electronic".into(),
            icon: "radio".into(),
            query: "electronic music".into(),
            color: "#14B8A6".into(),
        },
        SuggestionCategory {
            id: "hiphop".into(),
            label: "Hip Hop".into(),
            icon: "mic".into(),
            query: "hip hop".into(),
            color: "#A855F7".into(),
        },
        SuggestionCategory {
            id: "rock-clasico".into(),
            label: "Rock Clásico".into(),
            icon: "guitar".into(),
            query: "rock clásico".into(),
            color: "#DC2626".into(),
        },
        SuggestionCategory {
            id: "jazz".into(),
            label: "Jazz".into(),
            icon: "music-2".into(),
            query: "jazz".into(),
            color: "#D97706".into(),
        },
    ]
}

/// Get the current year as a string using only std.
fn current_year() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Seconds in a year (approximate, good enough for year extraction)
    const SECONDS_PER_YEAR: u64 = 31_536_000;
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // 1970 + elapsed years (approximate)
    let year: u64 = 1970 + secs / SECONDS_PER_YEAR;
    year.to_string()
}

/// Return the full list of suggestion categories with `{YEAR}` resolved.
pub fn get_suggestion_categories() -> Vec<SuggestionCategory> {
    let year = current_year();
    category_templates()
        .into_iter()
        .map(|cat| SuggestionCategory {
            label: cat.label.replace("{YEAR}", &year),
            query: cat.query.replace("{YEAR}", &year),
            ..cat
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories_have_valid_ids() {
        let cats = get_suggestion_categories();
        assert!(!cats.is_empty(), "should return at least one category");
        for cat in &cats {
            assert!(!cat.id.is_empty(), "id should not be empty");
            assert!(!cat.label.is_empty(), "label should not be empty");
            assert!(!cat.query.is_empty(), "query should not be empty");
            assert!(!cat.icon.is_empty(), "icon should not be empty");
            assert!(!cat.color.is_empty(), "color should not be empty");
        }
    }

    #[test]
    fn year_placeholders_are_resolved() {
        let cats = get_suggestion_categories();
        let current_year = current_year();

        // Categories that had {YEAR} in their template should now have the real year
        let reggaeton = cats.iter().find(|c| c.id == "reggaeton").unwrap();
        assert!(
            reggaeton.label.contains(&current_year),
            "Reggaeton label should contain the current year: got {}",
            reggaeton.label
        );
        assert!(
            reggaeton.query.contains(&current_year),
            "Reggaeton query should contain the current year: got {}",
            reggaeton.query
        );

        // Categories without {YEAR} should be unchanged
        let lofi = cats.iter().find(|c| c.id == "lofi").unwrap();
        assert_eq!(lofi.label, "Lo-fi Beats");
        assert_eq!(lofi.query, "lo-fi beats to relax");
    }

    #[test]
    fn all_categories_have_unique_ids() {
        let cats = get_suggestion_categories();
        let mut ids: Vec<String> = cats.iter().map(|c| c.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), cats.len(), "all category ids should be unique");
    }

    #[test]
    fn colors_are_valid_hex() {
        let cats = get_suggestion_categories();
        for cat in &cats {
            assert!(
                cat.color.starts_with('#') && cat.color.len() == 7,
                "color should be a 7-char hex string: got {} for {}",
                cat.color,
                cat.id
            );
        }
    }

    #[test]
    fn current_year_is_reasonable() {
        let year: u64 = current_year().parse().unwrap();
        // Should be between 2024 and 2100 for a very long-lived app
        assert!(
            year >= 2024 && year <= 2100,
            "current_year should be reasonable: got {}",
            year
        );
    }
}
