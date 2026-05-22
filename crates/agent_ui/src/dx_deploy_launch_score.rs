use crate::dx_deploy_launch_gate::DxDeployLaunchGateSnapshot;

pub(crate) fn launch_status_score(snapshot: &DxDeployLaunchGateSnapshot) -> Option<usize> {
    let score = snapshot.score?;
    let max_score = snapshot.max_score.filter(|max_score| *max_score > 0)?;

    Some((score.saturating_mul(100) / max_score).min(100))
}

pub(crate) fn launch_status_score_label(snapshot: &DxDeployLaunchGateSnapshot) -> Option<String> {
    launch_status_score(snapshot).map(|score| {
        let mut label = format!("{score}/100");
        if snapshot.score_estimated == Some(true) {
            label.push_str(" estimated");
        }
        label
    })
}
