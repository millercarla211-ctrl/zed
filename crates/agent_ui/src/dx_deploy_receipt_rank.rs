use std::{cmp::Ordering, time::SystemTime};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DxDeployReceiptSourceKind {
    Workspace,
    DxHub,
    DxWww,
    DxCli,
}

pub(crate) fn command_receipt_source_rank(kind: DxDeployReceiptSourceKind) -> u8 {
    match kind {
        DxDeployReceiptSourceKind::Workspace => 0,
        DxDeployReceiptSourceKind::DxWww => 1,
        DxDeployReceiptSourceKind::DxHub => 2,
        DxDeployReceiptSourceKind::DxCli => 3,
    }
}

pub(crate) fn provider_gate_receipt_source_rank(kind: DxDeployReceiptSourceKind) -> u8 {
    match kind {
        DxDeployReceiptSourceKind::DxHub => 0,
        DxDeployReceiptSourceKind::Workspace => 1,
        DxDeployReceiptSourceKind::DxWww => 2,
        DxDeployReceiptSourceKind::DxCli => 3,
    }
}

pub(crate) fn compare_rank_then_newest(
    left_rank: u8,
    left_modified: SystemTime,
    right_rank: u8,
    right_modified: SystemTime,
) -> Ordering {
    left_rank.cmp(&right_rank).then_with(|| {
        right_modified
            .partial_cmp(&left_modified)
            .unwrap_or(Ordering::Equal)
    })
}
