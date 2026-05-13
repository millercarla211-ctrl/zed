use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
pub enum SearchVertical {
    Web,
    News,
    Code,
    Academic,
    Images,
    Video,
    Models,
    Packages,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
pub enum SearchIntent {
    AgentGrounding,
    ProviderDiscovery,
    ModelDiscovery,
    CodeResearch,
    UserSearch,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
pub struct SearchRequestPlan {
    pub query: String,
    pub intent: SearchIntent,
    pub verticals: Vec<SearchVertical>,
    pub use_adjacent_metasearch: bool,
    pub notes: Vec<String>,
}

pub struct MetasearchBridge;

impl MetasearchBridge {
    pub fn for_agent_grounding(query: impl Into<String>) -> SearchRequestPlan {
        SearchRequestPlan {
            query: query.into(),
            intent: SearchIntent::AgentGrounding,
            verticals: vec![SearchVertical::Web, SearchVertical::Code, SearchVertical::Academic],
            use_adjacent_metasearch: true,
            notes: vec![
                "Prefer the adjacent metasearch project when it is available.".to_string(),
                "Use multiple verticals so agents can ground against docs, code, and general web results."
                    .to_string(),
            ],
        }
    }

    pub fn for_model_discovery(query: impl Into<String>) -> SearchRequestPlan {
        SearchRequestPlan {
            query: query.into(),
            intent: SearchIntent::ModelDiscovery,
            verticals: vec![
                SearchVertical::Models,
                SearchVertical::Web,
                SearchVertical::Code,
            ],
            use_adjacent_metasearch: true,
            notes: vec![
                "Use model and web verticals together for faster provider and runtime discovery."
                    .to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_grounding_plan_uses_multiple_verticals() {
        let plan = MetasearchBridge::for_agent_grounding("best local stt");
        assert!(plan.verticals.contains(&SearchVertical::Web));
        assert!(plan.verticals.contains(&SearchVertical::Code));
    }
}
