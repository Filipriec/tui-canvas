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
        // Linewise yanks carry a trailing newline, matching how a full line
        // lands in the system clipboard from a real editor.
        #[cfg(all(feature = "clipboard", feature = "keybindings"))]
        {
            let mut joined = lines.join("\n");
            joined.push('\n');
            crate::clipboard::set_system_clipboard(&joined);
        }
        self.register = Some(YankRegister::Lines(lines));
    }

    pub(crate) fn set_text_register(&mut self, lines: Vec<String>) {
        #[cfg(all(feature = "clipboard", feature = "keybindings"))]
        crate::clipboard::set_system_clipboard(&lines.join("\n"));
        self.register = Some(YankRegister::Text(lines));
    }

    pub(crate) fn register(&self) -> Option<&YankRegister> {
        self.register.as_ref()
    }
}
