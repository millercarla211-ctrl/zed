use crate::dx_agent_bridge::DxAgentBridgeSnapshot;

pub(super) fn dx_agent_bridge_next_action(snapshot: &DxAgentBridgeSnapshot) -> String {
    if snapshot.release_gate.present {
        snapshot.release_gate.next_action.clone()
    } else if snapshot.import_summary.present {
        snapshot.import_summary.next_action.clone()
    } else if snapshot.contract_summary.present {
        snapshot.contract_summary.next_action.clone()
    } else {
        snapshot.next_action.clone()
    }
}
