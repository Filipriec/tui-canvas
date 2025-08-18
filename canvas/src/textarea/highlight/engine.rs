// src/textarea/highlight/engine.rs
#![allow(dead_code)]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use ratatui::style::{Modifier, Style};
use syntect::{
    highlighting::{
        HighlightIterator, Highlighter, Style as SynStyle, Theme, ThemeSet,
    },
    parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet},
};

use crate::data_provider::DataProvider;
use super::chunks::StyledChunk;

#[derive(Debug)]
pub struct SyntectEngine {
    ps: SyntaxSet,
    ts: ThemeSet,
    theme_name: String,
    syntax_name: Option<String>,
    // Cached parser state (after line i)
    parse_after: Vec<ParseState>,
    // Cached scope stack (after line i)
    stack_after: Vec<ScopeStack>,
    // Hash of line contents to detect edits
    line_hashes: Vec<u64>,
}

impl SyntectEngine {
    pub fn new() -> Self {
        let ps = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        Self {
            ps,
            ts,
            theme_name: "InspiredGitHub".to_string(),
            syntax_name: None,
            parse_after: Vec::new(),
            stack_after: Vec::new(),
            line_hashes: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.parse_after.clear();
        self.stack_after.clear();
        self.line_hashes.clear();
    }

    pub fn set_theme(&mut self, theme_name: &str) -> bool {
        if self.ts.themes.contains_key(theme_name) {
            self.theme_name = theme_name.to_string();
            true
        } else {
            false
        }
    }

    pub fn set_syntax_by_name(&mut self, name: &str) -> bool {
        if self.ps.find_syntax_by_name(name).is_some() {
            self.syntax_name = Some(name.to_string());
            self.clear();
            true
        } else {
            false
        }
    }

    pub fn set_syntax_by_extension(&mut self, ext: &str) -> bool {
        if let Some(s) = self.ps.find_syntax_by_extension(ext) {
            self.syntax_name = Some(s.name.clone());
            self.clear();
            true
        } else {
            false
        }
    }

    pub fn invalidate_from(&mut self, line_idx: usize) {
        if line_idx < self.parse_after.len() {
            self.parse_after.truncate(line_idx);
        }
        if line_idx < self.stack_after.len() {
            self.stack_after.truncate(line_idx);
        }
        if line_idx < self.line_hashes.len() {
            self.line_hashes.truncate(line_idx);
        }
    }

    pub fn on_insert_line(&mut self, at: usize) {
        self.invalidate_from(at);
    }

    pub fn on_delete_line(&mut self, at: usize) {
        self.invalidate_from(at);
    }

    fn theme(&self) -> &Theme {
        self.ts
            .themes
            .get(&self.theme_name)
            .expect("theme exists")
    }

    fn syntax_ref(&self) -> &SyntaxReference {
        if let Some(name) = &self.syntax_name {
            if let Some(s) = self.ps.find_syntax_by_name(name) {
                return s;
            }
        }
        self.ps.find_syntax_plain_text()
    }

    fn map_syntect_style(s: SynStyle) -> Style {
        let fg =
            ratatui::style::Color::Rgb(s.foreground.r, s.foreground.g, s.foreground.b);
        let mut st = Style::default().fg(fg);
        use syntect::highlighting::FontStyle;
        if s.font_style.contains(FontStyle::BOLD) {
            st = st.add_modifier(Modifier::BOLD);
        }
        if s.font_style.contains(FontStyle::UNDERLINE) {
            st = st.add_modifier(Modifier::UNDERLINED);
        }
        if s.font_style.contains(FontStyle::ITALIC) {
            st = st.add_modifier(Modifier::ITALIC);
        }
        st
    }

    fn hash_line(s: &str) -> u64 {
        let mut h = DefaultHasher::new();
        s.hash(&mut h);
        h.finish()
    }

    // Verify cached chain up to the nearest trusted predecessor of line_idx,
    // using the provider to fetch the current lines.
    fn verify_and_truncate_before(&mut self, line_idx: usize, provider: &dyn DataProvider) {
        let mut k = std::cmp::min(line_idx, self.parse_after.len());
        while k > 0 {
            let j = k - 1;
            let curr = Self::hash_line(provider.field_value(j));
            if self.line_hashes.get(j) == Some(&curr) {
                break;
            }
            self.invalidate_from(j);
            k = j;
        }
    }

    // Ensure we have parser + stack for lines [0..line_idx)
    fn ensure_state_before(&mut self, line_idx: usize, provider: &dyn DataProvider) {
        if line_idx == 0 || self.parse_after.len() >= line_idx {
            return;
        }

        let syntax = self.syntax_ref();
        let theme = self.theme();
        let highlighter = Highlighter::new(theme);

        let mut ps = if self.parse_after.is_empty() {
            ParseState::new(syntax)
        } else {
            self.parse_after[self.parse_after.len() - 1].clone()
        };
        let mut stack = if self.stack_after.is_empty() {
            ScopeStack::new()
        } else {
            self.stack_after[self.stack_after.len() - 1].clone()
        };

        let start = self.parse_after.len();
        for i in start..line_idx {
            let s = provider.field_value(i);
            let ops = ps.parse_line(s);
            // Advance stack by applying ops using HighlightIterator
            let mut it = HighlightIterator::new(&highlighter, &ops[..], s, &mut stack);
            while let Some((_style, _text)) = it.next() {
                // Iterate to apply ops; we don't need the tokens here.
            }

            let h = Self::hash_line(s);

            self.parse_after.push(ps.clone());
            self.stack_after.push(stack.clone());
            if i >= self.line_hashes.len() {
                self.line_hashes.push(h);
            } else {
                self.line_hashes[i] = h;
            }
        }
    }

    // Highlight a single line using cached state; update caches for this line.
    pub fn highlight_line_cached(
        &mut self,
        line_idx: usize,
        line: &str,
        provider: &dyn DataProvider,
    ) -> Vec<StyledChunk> {
        // Auto-detect prior changes and truncate cache if needed
        self.verify_and_truncate_before(line_idx, provider);
        // Precompute states up to line_idx
        self.ensure_state_before(line_idx, provider);

        let syntax = self.syntax_ref();
        let theme = self.theme();
        let highlighter = Highlighter::new(theme);

        let mut ps = if line_idx == 0 {
            ParseState::new(syntax)
        } else if self.parse_after.len() >= line_idx {
            self.parse_after[line_idx - 1].clone()
        } else {
            ParseState::new(syntax)
        };

        let mut stack = if line_idx == 0 {
            ScopeStack::new()
        } else if self.stack_after.len() >= line_idx {
            self.stack_after[line_idx - 1].clone()
        } else {
            ScopeStack::new()
        };

        let ops = ps.parse_line(line);
        let mut iter = HighlightIterator::new(&highlighter, &ops[..], line, &mut stack);

        let mut out: Vec<StyledChunk> = Vec::new();
        while let Some((syn_style, slice)) = iter.next() {
            if slice.is_empty() {
                continue;
            }
            let text = slice.trim_end_matches('\n').to_string();
            if text.is_empty() {
                continue;
            }
            out.push(StyledChunk {
                text,
                style: Self::map_syntect_style(syn_style),
            });
        }

        // Update caches for this line (state after this line)
        let h = Self::hash_line(line);
        if line_idx >= self.parse_after.len() {
            self.parse_after.push(ps);
        } else {
            self.parse_after[line_idx] = ps;
        }
        if line_idx >= self.stack_after.len() {
            self.stack_after.push(stack);
        } else {
            self.stack_after[line_idx] = stack;
        }
        if line_idx >= self.line_hashes.len() {
            self.line_hashes.push(h);
        } else {
            self.line_hashes[line_idx] = h;
        }

        out
    }
}
