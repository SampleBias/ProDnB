use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Style {
    Liquid,
    Jungle,
    Neuro,
}

impl Style {
    pub fn from_int(value: u8) -> Self {
        match value {
            1 => Style::Liquid,
            2 => Style::Jungle,
            3 => Style::Neuro,
            _ => Style::Liquid,
        }
    }

    pub fn to_int(&self) -> u8 {
        match self {
            Style::Liquid => 1,
            Style::Jungle => 2,
            Style::Neuro => 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleConfig {
    pub style: Style,
    pub name: String,
    pub description: String,
}

impl StyleConfig {
    pub fn from_style(style: Style) -> Self {
        match style {
            Style::Liquid => StyleConfig {
                style,
                name: "Liquid".to_string(),
                description: "Smooth, melodic DnB with rolling basslines".to_string(),
            },
            Style::Jungle => StyleConfig {
                style,
                name: "Jungle".to_string(),
                description: "Ragga-influenced breaks and deep bass".to_string(),
            },
            Style::Neuro => StyleConfig {
                style,
                name: "Neurofunk".to_string(),
                description: "Aggressive, tech-forward sound".to_string(),
            },
        }
    }
}

impl Default for StyleConfig {
    fn default() -> Self {
        Self::from_style(Style::Liquid)
    }
}
