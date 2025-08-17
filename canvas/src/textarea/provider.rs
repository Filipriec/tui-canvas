// src/textarea/provider.rs
use crate::DataProvider;

#[derive(Debug, Clone)]
pub struct TextAreaProvider {
    lines: Vec<String>,
    name: String,
}

impl Default for TextAreaProvider {
    fn default() -> Self {
        Self {
            lines: vec![String::new()],
            name: "Text".to_string(),
        }
    }
}

impl TextAreaProvider {
    pub fn from_text<S: Into<String>>(text: S) -> Self {
        let text = text.into();
        let mut lines: Vec<String> =
            text.split('\n').map(|s| s.to_string()).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        Self {
            lines,
            name: "Text".to_string(),
        }
    }

    pub fn to_text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn set_text<S: Into<String>>(&mut self, text: S) {
        let text = text.into();
        self.lines = text.split('\n').map(|s| s.to_string()).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    #[inline]
    fn char_to_byte_index(s: &str, char_idx: usize) -> usize {
        s.char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or_else(|| s.len())
    }

    pub fn split_line_at(&mut self, line_idx: usize, at_char: usize) -> usize {
        if line_idx >= self.lines.len() {
            return self.lines.len().saturating_sub(1);
        }
        let line = &mut self.lines[line_idx];
        let byte_idx = Self::char_to_byte_index(line, at_char);
        let right = line[byte_idx..].to_string();
        line.truncate(byte_idx);
        let insert_at = line_idx + 1;
        self.lines.insert(insert_at, right);
        insert_at
    }

    pub fn join_with_next(&mut self, line_idx: usize) -> Option<usize> {
        if line_idx + 1 >= self.lines.len() {
            return None;
        }
        let left_len = self.lines[line_idx].chars().count();
        let right = self.lines.remove(line_idx + 1);
        self.lines[line_idx].push_str(&right);
        Some(left_len)
    }

    pub fn join_with_prev(
        &mut self,
        line_idx: usize,
    ) -> Option<(usize, usize)> {
        if line_idx == 0 || line_idx >= self.lines.len() {
            return None;
        }
        let prev_idx = line_idx - 1;
        let prev_len = self.lines[prev_idx].chars().count();
        let curr = self.lines.remove(line_idx);
        self.lines[prev_idx].push_str(&curr);
        Some((prev_idx, prev_len))
    }

    pub fn insert_blank_line_after(&mut self, idx: usize) -> usize {
        let clamped = idx.min(self.lines.len());
        let insert_at = if clamped >= self.lines.len() {
            self.lines.len()
        } else {
            clamped + 1
        };
        if insert_at == self.lines.len() {
            self.lines.push(String::new());
        } else {
            self.lines.insert(insert_at, String::new());
        }
        insert_at
    }

    pub fn insert_blank_line_before(&mut self, idx: usize) -> usize {
        let insert_at = idx.min(self.lines.len());
        self.lines.insert(insert_at, String::new());
        insert_at
    }
}

impl DataProvider for TextAreaProvider {
    fn field_count(&self) -> usize {
        self.lines.len()
    }

    fn field_name(&self, _index: usize) -> &str {
        &self.name
    }

    fn field_value(&self, index: usize) -> &str {
        self.lines.get(index).map(|s| s.as_str()).unwrap_or("")
    }

    fn set_field_value(&mut self, index: usize, value: String) {
        if index < self.lines.len() {
            self.lines[index] = value;
        }
    }
}
