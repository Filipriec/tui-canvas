#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum KeybindingParadigm {
    #[default]
    Vim,
    Helix,
    Emacs,
    /// Modeless, VSCode-style editing. Behaves like the non-modal Emacs
    /// paradigm under the hood (always editing, region-style selection); the
    /// distinction exists so VSCode-specific behavior can diverge later.
    Vscode,
}

impl KeybindingParadigm {
    pub(crate) fn is_helix(self) -> bool {
        matches!(self, Self::Helix)
    }
}
