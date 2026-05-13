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
pub enum LongContextTask {
    SummarizeLargeDocument,
    AnalyzeCodebase,
    BuildAgentContext,
    RecursiveQuestionAnswering,
    MultiFileReasoning,
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
pub struct LongContextExecutionPlan {
    pub task: LongContextTask,
    pub prefer_adjacent_rlm: bool,
    pub use_serializer: bool,
    pub use_prompt_cache: bool,
    pub notes: Vec<String>,
}

pub struct RlmBridge;

impl RlmBridge {
    pub fn for_codebase_analysis() -> LongContextExecutionPlan {
        LongContextExecutionPlan {
            task: LongContextTask::AnalyzeCodebase,
            prefer_adjacent_rlm: true,
            use_serializer: true,
            use_prompt_cache: true,
            notes: vec![
                "Use RLM when the codebase exceeds the working context of the selected model."
                    .to_string(),
                "Pack intermediate context with the serializer and reuse cached prompt fragments."
                    .to_string(),
            ],
        }
    }

    pub fn for_large_document_summary() -> LongContextExecutionPlan {
        LongContextExecutionPlan {
            task: LongContextTask::SummarizeLargeDocument,
            prefer_adjacent_rlm: true,
            use_serializer: true,
            use_prompt_cache: true,
            notes: vec![
                "Split oversized documents recursively instead of truncating them.".to_string(),
                "Reuse summaries and chunk-level prompt envelopes across retries.".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codebase_analysis_prefers_rlm_and_caches() {
        let plan = RlmBridge::for_codebase_analysis();
        assert!(plan.prefer_adjacent_rlm);
        assert!(plan.use_prompt_cache);
    }
}
