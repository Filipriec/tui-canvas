#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct YankBehaviorState {
    register: Option<YankRegister>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum YankRegister {
    Lines(Vec<String>),
    Text(Vec<String>),
}

impl YankBehaviorState {
    pub(crate) fn set_line_register(&mut self, lines: Vec<String>) {
        self.register = Some(YankRegister::Lines(lines));
    }

    pub(crate) fn set_text_register(&mut self, lines: Vec<String>) {
        self.register = Some(YankRegister::Text(lines));
    }

    pub(crate) fn register(&self) -> Option<&YankRegister> {
        self.register.as_ref()
    }
}
