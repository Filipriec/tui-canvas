#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum KeybindingParadigm {
    #[default]
    Vim,
    Helix,
    Emacs,
}

impl KeybindingParadigm {
    pub(crate) fn is_helix(self) -> bool {
        matches!(self, Self::Helix)
    }
}
