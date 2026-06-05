mod paradigm;
mod vim;
mod yank;

pub(crate) use paradigm::KeybindingParadigm;
pub(crate) use vim::VimBehaviorState;
pub(crate) use yank::{YankBehaviorState, YankRegister};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EditorBehaviorState {
    vim: VimBehaviorState,
    yank: YankBehaviorState,
    paradigm: KeybindingParadigm,
}

impl EditorBehaviorState {
    pub(crate) fn vim_mut(&mut self) -> &mut VimBehaviorState {
        &mut self.vim
    }

    pub(crate) fn yank(&self) -> &YankBehaviorState {
        &self.yank
    }

    pub(crate) fn yank_mut(&mut self) -> &mut YankBehaviorState {
        &mut self.yank
    }

    pub(crate) fn paradigm(&self) -> KeybindingParadigm {
        self.paradigm
    }

    pub(crate) fn set_paradigm(&mut self, paradigm: KeybindingParadigm) {
        self.paradigm = paradigm;
    }
}
