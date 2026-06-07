use crate::{
    canvas::state::SelectionState,
    textarea::{TextAreaDataProvider, TextAreaState},
};

impl<P: TextAreaDataProvider> TextAreaState<P> {
    /// Emacs `M-w` — copy region to kill ring without deleting.
    pub(crate) fn copy_region_emacs(&mut self) {
        match self.selection_state().clone() {
            SelectionState::Characterwise { anchor } => {
                let _ = self.copy_character_region_emacs(anchor);
            }
            SelectionState::Linewise { .. } => self.yank_selection(),
            SelectionState::None => {}
        }
    }

    /// Emacs `C-w` — kill region (yank then delete, deactivate mark).
    pub(crate) fn kill_region_emacs(&mut self) {
        match self.selection_state().clone() {
            SelectionState::Characterwise { anchor } => {
                let _ = self.copy_character_region_emacs(anchor);
                let _ = self.delete_character_region_emacs(anchor);
            }
            SelectionState::Linewise { .. } => {
                self.yank_selection();
                let _ = self.delete_selection_once(false);
            }
            SelectionState::None => {}
        }
        self.exit_highlight_mode_emacs();
    }

    /// Delete active region without pushing to the kill ring.
    pub(crate) fn delete_region_emacs(&mut self) {
        match self.selection_state().clone() {
            SelectionState::Characterwise { anchor } => {
                let _ = self.delete_character_region_emacs(anchor);
            }
            SelectionState::Linewise { .. } => {
                let _ = self.delete_selection_once(false);
            }
            SelectionState::None => {}
        }
        self.exit_highlight_mode_emacs();
    }

    pub(crate) fn yank_primary_selection_emacs(&mut self) {
        self.copy_region_emacs();
    }

    fn copy_character_region_emacs(&mut self, anchor: (usize, usize)) -> bool {
        let Some((start, end)) = self.emacs_region_endpoints(anchor) else {
            return false;
        };
        let lines = self.core.data_provider().capture_content();
        if start.0 >= lines.len() || end.0 >= lines.len() {
            return false;
        }

        let yanked = if start.0 == end.0 {
            vec![lines[start.0]
                .chars()
                .skip(start.1)
                .take(end.1.saturating_sub(start.1))
                .collect()]
        } else {
            let mut yanked = Vec::new();
            yanked.push(lines[start.0].chars().skip(start.1).collect());
            for line in &lines[start.0 + 1..end.0] {
                yanked.push(line.clone());
            }
            yanked.push(lines[end.0].chars().take(end.1).collect());
            yanked
        };

        self.core
            .behavior_state
            .yank_mut()
            .set_text_register(yanked);
        true
    }

    fn delete_character_region_emacs(&mut self, anchor: (usize, usize)) -> bool {
        let Some((start, end)) = self.emacs_region_endpoints(anchor) else {
            return false;
        };
        let lines = self.core.data_provider().capture_content();
        if start.0 >= lines.len() || end.0 >= lines.len() {
            return false;
        }

        self.core
            .record_checkpoint(crate::editor::features::history::EditKind::Delete);

        let mut content = lines;
        if start.0 == end.0 {
            let line = &content[start.0];
            let new_line: String = line
                .chars()
                .enumerate()
                .filter_map(|(idx, ch)| {
                    if idx < start.1 || idx >= end.1 {
                        Some(ch)
                    } else {
                        None
                    }
                })
                .collect();
            content[start.0] = new_line;
        } else {
            let first: String = content[start.0].chars().take(start.1).collect();
            let last: String = content[end.0].chars().skip(end.1).collect();
            content[start.0] = format!("{first}{last}");
            if end.0 > start.0 {
                content.drain(start.0 + 1..=end.0);
            }
        }

        self.core.data_provider_mut().restore_content(&content);
        let _ = self.transition_to_field(start.0);
        self.set_cursor_position(start.1);
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
        true
    }

    fn emacs_region_endpoints(
        &self,
        anchor: (usize, usize),
    ) -> Option<((usize, usize), (usize, usize))> {
        let cursor = (self.current_field(), self.cursor_position());
        if anchor == cursor {
            return None;
        }
        Some((anchor.min(cursor), anchor.max(cursor)))
    }
}
