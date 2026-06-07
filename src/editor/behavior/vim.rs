/// The Vim operator awaiting a motion or text object (`d`, `c`, `y`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VimOperator {
    Delete,
    Change,
    Yank,
}

/// A Vim operator captured and waiting for the motion that delimits its range.
///
/// `anchor` is the cursor position at the moment the operator key was pressed;
/// the motion moves the cursor and the span between the two becomes the operated
/// range. `count` is the count entered *before* the operator (`2dw`), which is
/// multiplied with the motion's own count (`d3w`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VimPendingOperator {
    pub operator: VimOperator,
    pub count: usize,
    pub anchor: (usize, usize),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct VimBehaviorState {
    count: Option<usize>,
    pending_operator: Option<VimPendingOperator>,
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

    pub(crate) fn set_pending_operator(&mut self, pending: VimPendingOperator) {
        self.pending_operator = Some(pending);
    }

    pub(crate) fn pending_operator(&self) -> Option<VimPendingOperator> {
        self.pending_operator
    }

    pub(crate) fn has_pending_operator(&self) -> bool {
        self.pending_operator.is_some()
    }

    pub(crate) fn clear_pending_operator(&mut self) {
        self.pending_operator = None;
    }
}
