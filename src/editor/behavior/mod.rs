mod vim;

pub(crate) use vim::{VimBehaviorState, VimRegister};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EditorBehaviorState {
    vim: VimBehaviorState,
}

impl EditorBehaviorState {
    pub(crate) fn vim(&self) -> &VimBehaviorState {
        &self.vim
    }

    pub(crate) fn vim_mut(&mut self) -> &mut VimBehaviorState {
        &mut self.vim
    }
}
