// src/textarea/highlight/state.rs
use std::ops::{Deref, DerefMut};

use super::engine::SyntectEngine;
use crate::widgets::textarea::state::TextAreaState;

#[derive(Default)]
pub struct TextAreaSyntaxState {
    pub textarea: TextAreaState,
    pub engine: SyntectEngine,
}

impl TextAreaSyntaxState {
    pub fn from_text<S: Into<String>>(text: S) -> Self {
        let mut s = Self::default();
        s.textarea.set_text(text);
        s
    }

    pub fn set_syntax_theme(&mut self, theme: &str) -> bool {
        self.engine.set_theme(theme)
    }
    pub fn set_syntax_by_name(&mut self, name: &str) -> bool {
        self.engine.set_syntax_by_name(name)
    }
    pub fn set_syntax_by_extension(&mut self, ext: &str) -> bool {
        self.engine.set_syntax_by_extension(ext)
    }
}

impl Deref for TextAreaSyntaxState {
    type Target = TextAreaState;
    fn deref(&self) -> &Self::Target {
        &self.textarea
    }
}

impl DerefMut for TextAreaSyntaxState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.textarea
    }
}
