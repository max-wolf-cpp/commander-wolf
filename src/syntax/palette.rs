use std::collections::HashMap;

use ratatui::style::Color;

use super::SyntaxCategory;
use crate::theme::parse_theme_color;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SyntaxTheme {
    colors: HashMap<SyntaxCategory, Color>,
}

impl SyntaxTheme {
    pub fn from_fields(fields: &HashMap<String, String>) -> Result<Self, String> {
        for key in fields.keys() {
            if !SyntaxCategory::ALL
                .iter()
                .any(|category| category.name() == key)
            {
                return Err(format!("unknown syntax category '{key}'"));
            }
        }

        let text = fields.get("text").ok_or("syntax theme is missing 'text'")?;
        let mut colors = HashMap::new();
        for category in SyntaxCategory::ALL {
            let value = fields.get(category.name()).unwrap_or(text);
            let color = parse_theme_color(value)
                .map_err(|error| format!("syntax {}: {error}", category.name()))?;
            colors.insert(category, color);
        }

        Ok(Self { colors })
    }

    pub fn generated_default() -> Self {
        use SyntaxCategory as C;

        let mut colors = HashMap::new();
        for category in C::ALL {
            let color = match category {
                C::Comment | C::Punctuation => Color::Reset,
                C::Keyword | C::Boolean | C::Preprocessor | C::Strong => Color::Indexed(5),
                C::String | C::Added => Color::Indexed(2),
                C::Escape | C::Number | C::Constant => Color::Indexed(3),
                C::Type | C::Function | C::Link | C::Heading => Color::Indexed(4),
                C::Tag | C::Attribute | C::Property => Color::Indexed(6),
                C::Removed | C::Invalid => Color::Indexed(1),
                _ => Color::Reset,
            };
            colors.insert(category, color);
        }

        Self { colors }
    }

    pub fn with_fields(mut self, fields: &HashMap<String, String>) -> Result<Self, String> {
        for (key, value) in fields {
            let category = SyntaxCategory::ALL
                .iter()
                .find(|category| category.name() == key)
                .ok_or_else(|| format!("unknown syntax category '{key}'"))?;
            self.colors.insert(*category, parse_theme_color(value)?);
        }

        Ok(self)
    }

    pub fn color(&self, category: SyntaxCategory, fallback: impl Into<Color>) -> Color {
        self.colors
            .get(&category)
            .copied()
            .unwrap_or_else(|| fallback.into())
    }
}
