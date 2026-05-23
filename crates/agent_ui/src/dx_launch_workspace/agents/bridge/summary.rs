use gpui::AnyElement;

use crate::dx_agent_bridge::DxAgentBridgeSnapshot;

use self::{
    contract::dx_agent_bridge_contract_rows, gate::dx_agent_bridge_gate_rows,
    import::dx_agent_bridge_import_rows, overview::dx_agent_bridge_overview_rows,
};

mod contract;
mod gate;
mod import;
mod overview;

pub(super) fn dx_agent_bridge_summary_rows(snapshot: &DxAgentBridgeSnapshot) -> Vec<AnyElement> {
    let mut rows = dx_agent_bridge_overview_rows(snapshot);
    rows.extend(dx_agent_bridge_contract_rows(snapshot));
    rows.extend(dx_agent_bridge_import_rows(snapshot));
    rows.extend(dx_agent_bridge_gate_rows(snapshot));
    rows
}
