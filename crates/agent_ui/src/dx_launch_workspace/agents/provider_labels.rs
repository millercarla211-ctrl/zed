#[path = "provider_labels/detail.rs"]
mod detail;
#[path = "provider_labels/state.rs"]
mod state;
#[path = "provider_labels/text.rs"]
mod text;

pub(crate) use self::detail::{model_detail_label, provider_detail_label};
pub(crate) use self::state::{model_state_label, provider_state_label};

#[cfg(test)]
#[path = "provider_labels_tests.rs"]
mod provider_labels_tests;
