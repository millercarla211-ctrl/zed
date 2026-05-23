use gpui::AnyElement;

use crate::dx_launch_readiness::DxLaunchReadinessSnapshot;

use super::super::metric_row;

pub(super) fn launch_readiness_example_rows(
    snapshot: &DxLaunchReadinessSnapshot,
) -> Vec<AnyElement> {
    let mut rows = Vec::new();

    for (ix, example) in snapshot.examples.iter().take(3).enumerate() {
        rows.push(metric_row(
            format!("Example {}", ix + 1),
            format!("{}: {} ({})", example.label, example.status, example.detail),
        ));

        if let Some(next_action) = example.next_action.as_ref() {
            rows.push(metric_row(format!("Next {}", ix + 1), next_action.clone()));
        }
    }

    rows
}
