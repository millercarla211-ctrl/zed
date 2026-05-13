//! Search categories.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Categories of search supported by the metasearch engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchCategory {
    General,
    Images,
    Videos,
    News,
    Maps,
    Music,
    Science,
    Files,
    #[serde(rename = "social_media", alias = "socialmedia", alias = "social")]
    SocialMedia,
    IT,
}

impl SearchCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Images => "images",
            Self::Videos => "videos",
            Self::News => "news",
            Self::Maps => "maps",
            Self::Music => "music",
            Self::Science => "science",
            Self::Files => "files",
            Self::SocialMedia => "social_media",
            Self::IT => "it",
        }
    }
}

impl std::fmt::Display for SearchCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SearchCategory {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "general" | "web" | "" => Ok(Self::General),
            "images" | "image" => Ok(Self::Images),
            "videos" | "video" => Ok(Self::Videos),
            "news" => Ok(Self::News),
            "maps" | "map" => Ok(Self::Maps),
            "music" => Ok(Self::Music),
            "science" | "academic" => Ok(Self::Science),
            "files" | "file" => Ok(Self::Files),
            "social" | "social_media" | "socialmedia" => Ok(Self::SocialMedia),
            "it" | "code" | "dev" => Ok(Self::IT),
            _ => Err(()),
        }
    }
}
