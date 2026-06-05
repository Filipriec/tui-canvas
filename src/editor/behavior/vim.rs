#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct VimBehaviorState {
    count: Option<usize>,
    yank_register: Option<YankRegister>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum YankRegister {
    Lines(Vec<String>),
    Text(Vec<String>),
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

    pub(crate) fn set_line_yank_register(&mut self, lines: Vec<String>) {
        self.yank_register = Some(YankRegister::Lines(lines));
    }

    pub(crate) fn set_text_yank_register(&mut self, lines: Vec<String>) {
        self.yank_register = Some(YankRegister::Text(lines));
    }

    pub(crate) fn yank_register(&self) -> Option<&YankRegister> {
        self.yank_register.as_ref()
    }
}
