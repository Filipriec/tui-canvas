mod paradigm;
mod vim;

pub(crate) use paradigm::KeybindingParadigm;
pub(crate) use vim::{VimBehaviorState, VimRegister};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EditorBehaviorState {
    vim: VimBehaviorState,
    paradigm: KeybindingParadigm,
}

impl EditorBehaviorState {
    pub(crate) fn vim(&self) -> &VimBehaviorState {
        &self.vim
    }

    pub(crate) fn vim_mut(&mut self) -> &mut VimBehaviorState {
        &mut self.vim
    }

    pub(crate) fn paradigm(&self) -> KeybindingParadigm {
        self.paradigm
    }

    pub(crate) fn set_paradigm(&mut self, paradigm: KeybindingParadigm) {
        self.paradigm = paradigm;
    }
}
