// src/textarea/provider.rs
use crate::DataProvider;
use once_cell::unsync::OnceCell;
use ropey::Rope;
use std::io::{self, BufReader, Read};
use std::path::Path;

pub trait TextAreaDataProvider: DataProvider {
    fn from_text(text: String) -> Self
    where
        Self: Sized;

    fn to_text(&self) -> String;
    fn set_text(&mut self, text: String);
    fn line_count(&self) -> usize;
    fn split_line_at(&mut self, line_idx: usize, at_char: usize) -> usize;
    fn join_with_next(&mut self, line_idx: usize) -> Option<usize>;
    fn join_with_prev(&mut self, line_idx: usize) -> Option<(usize, usize)>;
    fn insert_blank_line_after(&mut self, line_idx: usize) -> usize;
    fn insert_blank_line_before(&mut self, line_idx: usize) -> usize;
}

#[derive(Debug)]
pub struct TextAreaProvider {
    rope: Rope,
    name: String,
    // Lazy per-line cache; only lines that are actually used get materialized.
    // This keeps memory low even for very large files.
    line_cache: Vec<OnceCell<String>>,
}

impl Default for TextAreaProvider {
    fn default() -> Self {
        let rope = Rope::from_str("");
        Self {
            rope,
            name: "Text".to_string(),
            line_cache: vec![OnceCell::new()], // at least 1 logical line
        }
    }
}

impl TextAreaProvider {
    pub fn from_text<S: Into<String>>(text: S) -> Self {
        let s = text.into();
        let rope = Rope::from_str(&s);
        let lines = rope.len_lines().max(1);
        Self {
            rope,
            name: "Text".to_string(),
            line_cache: vec![(); lines]
                .into_iter()
                .map(|_| OnceCell::new())
                .collect(),
        }
    }

    pub fn to_text(&self) -> String {
        self.rope.to_string()
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let f = std::fs::File::open(path)?;
        let mut reader = BufReader::new(f);
        Self::from_reader(&mut reader)
    }

    pub fn from_reader<R: Read>(reader: &mut R) -> io::Result<Self> {
        let rope = Rope::from_reader(reader)?;
        let lines = rope.len_lines().max(1);
        Ok(Self {
            rope,
            name: "Text".to_string(),
            line_cache: vec![(); lines]
                .into_iter()
                .map(|_| OnceCell::new())
                .collect(),
        })
    }

    pub fn set_text<S: Into<String>>(&mut self, text: S) {
        let s = text.into();
        self.rope = Rope::from_str(&s);
        self.resize_cache();
        self.invalidate_cache_from(0);
    }

    pub fn line_count(&self) -> usize {
        self.rope.len_lines().max(1)
    }

    fn resize_cache(&mut self) {
        let want = self.line_count();
        if self.line_cache.len() < want {
            self.line_cache
                .extend((0..(want - self.line_cache.len())).map(|_| OnceCell::new()));
        } else if self.line_cache.len() > want {
            self.line_cache.truncate(want);
        }
    }

    fn invalidate_cache_from(&mut self, line_idx: usize) {
        self.resize_cache();
        if line_idx < self.line_cache.len() {
            for cell in &mut self.line_cache[line_idx..] {
                let _ = cell.take();
            }
        }
    }

    #[inline]
    fn line_bounds_chars(&self, line_idx: usize) -> (usize, usize) {
        // Returns [start, end) in char indices for content only (excluding newline).
        let total_lines = self.line_count();
        let start = self.rope.line_to_char(line_idx);
        let end_exclusive = if line_idx + 1 < total_lines {
            // Next line start is at the char index right after the newline.
            // Exclude the newline itself by not including it in the range.
            self.rope.line_to_char(line_idx + 1) - 1
        } else {
            self.rope.len_chars()
        };
        (start, end_exclusive)
    }

    fn line_content_len_chars(&self, line_idx: usize) -> usize {
        let slice = self.rope.line(line_idx);
        let mut len = slice.len_chars();
        if line_idx + 1 < self.line_count() && len > 0 {
            // Non-final lines include a trailing '\n' char in rope; exclude it.
            len -= 1;
        }
        len
    }

    fn compute_line_string(&self, index: usize) -> String {
        let mut s = self.rope.line(index).to_string();
        // Trim trailing newline/CR if present (for non-final lines)
        if s.ends_with('\n') {
            s.pop();
            if s.ends_with('\r') {
                s.pop();
            }
        }
        s
    }

    /// Split line at a character offset (within that line).
    /// Returns the index of the newly created line (line_idx + 1).
    pub fn split_line_at(&mut self, line_idx: usize, at_char: usize) -> usize {
        let lines = self.line_count();
        let clamped_line = line_idx.min(lines.saturating_sub(1));
        let (start, end) = self.line_bounds_chars(clamped_line);
        let line_len = end.saturating_sub(start);
        let at = at_char.min(line_len);

        let insert_at = start + at;
        self.rope.insert(insert_at, "\n");

        self.resize_cache();
        self.invalidate_cache_from(clamped_line);
        clamped_line + 1
    }

    /// Join current line with the next by removing the newline.
    /// Returns Some(new_cursor_col_on_merged_line) or None if no next line.
    pub fn join_with_next(&mut self, line_idx: usize) -> Option<usize> {
        if line_idx + 1 >= self.line_count() {
            return None;
        }
        let newline_pos = self.rope.line_to_char(line_idx + 1) - 1;
        let left_len = self.line_content_len_chars(line_idx);
        self.rope.remove(newline_pos..newline_pos + 1);

        self.resize_cache();
        self.invalidate_cache_from(line_idx);
        Some(left_len)
    }

    /// Join current line with the previous by removing the previous newline.
    /// Returns Some((new_prev_index, cursor_col)) or None if at line 0.
    pub fn join_with_prev(&mut self, line_idx: usize) -> Option<(usize, usize)> {
        if line_idx == 0 || line_idx >= self.line_count() {
            return None;
        }
        let prev_idx = line_idx - 1;
        let prev_len = self.line_content_len_chars(prev_idx);
        let newline_pos = self.rope.line_to_char(line_idx) - 1;
        self.rope.remove(newline_pos..newline_pos + 1);

        self.resize_cache();
        self.invalidate_cache_from(prev_idx);
        Some((prev_idx, prev_len))
    }

    /// Insert an empty line after given index.
    /// Returns the index of the inserted blank line (line_idx + 1).
    pub fn insert_blank_line_after(&mut self, line_idx: usize) -> usize {
        let lines = self.line_count();
        let clamped = line_idx.min(lines.saturating_sub(1));
        let pos = if clamped + 1 < lines {
            self.rope.line_to_char(clamped + 1)
        } else {
            self.rope.len_chars()
        };
        self.rope.insert(pos, "\n");

        self.resize_cache();
        self.invalidate_cache_from(clamped);
        clamped + 1
    }

    /// Insert an empty line before given index.
    /// Returns the index of the inserted blank line (line_idx).
    pub fn insert_blank_line_before(&mut self, line_idx: usize) -> usize {
        let clamped = line_idx.min(self.line_count());
        let pos = if clamped < self.line_count() {
            self.rope.line_to_char(clamped)
        } else {
            self.rope.len_chars()
        };
        self.rope.insert(pos, "\n");

        self.resize_cache();
        self.invalidate_cache_from(clamped);
        clamped
    }
}

impl DataProvider for TextAreaProvider {
    fn field_count(&self) -> usize {
        self.line_count()
    }

    fn field_name(&self, _index: usize) -> &str {
        &self.name
    }

    fn field_value(&self, index: usize) -> &str {
        if index >= self.line_cache.len() {
            return "";
        }
        let cell = &self.line_cache[index];
        let s_ref = cell.get_or_init(|| self.compute_line_string(index));
        s_ref.as_str()
    }

    fn set_field_value(&mut self, index: usize, value: String) {
        if index >= self.line_count() {
            return;
        }
        let clean = value.replace('\n', "");

        let (start, end) = self.line_bounds_chars(index);
        self.rope.remove(start..end);
        self.rope.insert(start, &clean);

        self.resize_cache();
        if index < self.line_cache.len() {
            let _ = self.line_cache[index].take();
            let _ = self.line_cache[index].set(clean);
        }
    }
}

impl TextAreaDataProvider for TextAreaProvider {
    fn from_text(text: String) -> Self
    where
        Self: Sized,
    {
        Self::from_text(text)
    }

    fn to_text(&self) -> String {
        TextAreaProvider::to_text(self)
    }

    fn set_text(&mut self, text: String) {
        TextAreaProvider::set_text(self, text);
    }

    fn line_count(&self) -> usize {
        TextAreaProvider::line_count(self)
    }

    fn split_line_at(&mut self, line_idx: usize, at_char: usize) -> usize {
        TextAreaProvider::split_line_at(self, line_idx, at_char)
    }

    fn join_with_next(&mut self, line_idx: usize) -> Option<usize> {
        TextAreaProvider::join_with_next(self, line_idx)
    }

    fn join_with_prev(&mut self, line_idx: usize) -> Option<(usize, usize)> {
        TextAreaProvider::join_with_prev(self, line_idx)
    }

    fn insert_blank_line_after(&mut self, line_idx: usize) -> usize {
        TextAreaProvider::insert_blank_line_after(self, line_idx)
    }

    fn insert_blank_line_before(&mut self, line_idx: usize) -> usize {
        TextAreaProvider::insert_blank_line_before(self, line_idx)
    }
}
