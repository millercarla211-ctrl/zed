use super::DxLaunchReadinessStatusCounts;

impl DxLaunchReadinessStatusCounts {
    pub(super) fn record(&mut self, status: &str) {
        match status {
            "ready" => self.ready += 1,
            "warning" => self.warning += 1,
            "blocked" => self.blocked += 1,
            _ => self.unknown += 1,
        }
    }

    pub(crate) fn summary(&self) -> String {
        format!(
            "{} ready / {} warning / {} blocked",
            self.ready, self.warning, self.blocked
        )
    }
}
