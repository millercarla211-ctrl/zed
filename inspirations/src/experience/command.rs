use super::control::{ControlActionPlan, FlowControlPolicy};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowCommandIntent {
    RewriteSelection,
    OpenUrl,
    LaunchApplication,
    SystemSearch,
    FocusOverlay,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowCommandPlan {
    pub intent: FlowCommandIntent,
    pub transcript: String,
    pub control_actions: Vec<ControlActionPlan>,
    pub local_model_hint: &'static str,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlowCommandRouter;

impl FlowCommandRouter {
    pub fn route(
        &self,
        transcript: impl Into<String>,
        control: &FlowControlPolicy,
    ) -> FlowCommandPlan {
        let transcript = transcript.into();
        let lower = transcript.to_ascii_lowercase();

        if let Some(url) = extract_url_target(&transcript, &lower) {
            return FlowCommandPlan {
                intent: FlowCommandIntent::OpenUrl,
                transcript,
                control_actions: vec![control.plan_open_url(url)],
                local_model_hint: "qwen3-0.6b",
            };
        }

        if let Some(app) = extract_app_target(&transcript, &lower) {
            return FlowCommandPlan {
                intent: FlowCommandIntent::LaunchApplication,
                transcript,
                control_actions: vec![control.plan_launch_app(app)],
                local_model_hint: "qwen3-0.6b",
            };
        }

        if let Some(query) = extract_search_target(&transcript, &lower) {
            return FlowCommandPlan {
                intent: FlowCommandIntent::SystemSearch,
                transcript,
                control_actions: vec![control.plan_system_search(query)],
                local_model_hint: "qwen3-0.6b",
            };
        }

        if lower.contains("rewrite this") || lower.contains("fix this") {
            return FlowCommandPlan {
                intent: FlowCommandIntent::RewriteSelection,
                transcript,
                control_actions: Vec::new(),
                local_model_hint: "qwen3-0.6b",
            };
        }

        if lower.contains("open flow") || lower.contains("show flow") {
            return FlowCommandPlan {
                intent: FlowCommandIntent::FocusOverlay,
                transcript,
                control_actions: vec![control.plan_shortcut("Alt+`")],
                local_model_hint: "qwen3-0.6b",
            };
        }

        FlowCommandPlan {
            intent: FlowCommandIntent::Unknown,
            transcript,
            control_actions: Vec::new(),
            local_model_hint: "qwen3-0.6b",
        }
    }
}

fn extract_url_target(original: &str, lower: &str) -> Option<String> {
    if lower.starts_with("open ")
        && (lower.contains("http://") || lower.contains("https://") || lower.contains(".com"))
    {
        return Some(original["open ".len()..].trim().to_string());
    }

    None
}

fn extract_app_target(original: &str, lower: &str) -> Option<String> {
    if lower.starts_with("launch ") {
        return Some(original["launch ".len()..].trim().to_string());
    }

    if lower.starts_with("open app ") {
        return Some(original["open app ".len()..].trim().to_string());
    }

    None
}

fn extract_search_target(original: &str, lower: &str) -> Option<String> {
    if lower.starts_with("search ") {
        return Some(original["search ".len()..].trim().to_string());
    }

    None
}
