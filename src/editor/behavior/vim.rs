#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct VimBehaviorState {
    count: Option<usize>,
}

impl VimBehaviorState {
    pub(crate) fn has_count(&self) -> bool {
        self.count.is_some()
    }

    pub(crate) fn push_count_digit(&mut self, digit: usize) {
        let current = self.count.unwrap_or(0);
        self.count = Some(current.saturating_mul(10).saturating_add(digit));
    }

    pub(crate) fn take_count_or_one(&mut self) -> usize {
        self.count.take().unwrap_or(1).max(1)
    }

    pub(crate) fn reset_count(&mut self) {
        self.count = None;
    }
}
