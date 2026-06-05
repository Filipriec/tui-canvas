// src/textarea/highlight/chunks.rs
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthChar;

use crate::textarea::state::{continuation_prefix, continuation_prefix_width};

#[derive(Debug, Clone)]
pub struct StyledChunk {
    pub text: String,
    pub style: Style,
}

pub fn display_width_chunks(chunks: &[StyledChunk]) -> u16 {
    chunks
        .iter()
        .map(|c| {
            c.text
                .chars()
                .map(|ch| UnicodeWidthChar::width(ch).unwrap_or(0) as u16)
                .sum::<u16>()
        })
        .sum()
}

pub fn slice_chunks_by_display_cols(
    chunks: &[StyledChunk],
    start_cols: u16,
    max_cols: u16,
) -> Vec<StyledChunk> {
    if max_cols == 0 {
        return Vec::new();
    }

    let mut skipped: u16 = 0;
    let mut taken: u16 = 0;
    let mut out: Vec<StyledChunk> = Vec::new();

    for ch in chunks {
        if taken >= max_cols {
            break;
        }

        let mut acc = String::new();

        for c in ch.text.chars() {
            let w = UnicodeWidthChar::width(c).unwrap_or(0) as u16;
            if skipped + w <= start_cols {
                skipped += w;
                continue;
            }
            if taken + w > max_cols {
                break;
            }
            acc.push(c);
            taken = taken.saturating_add(w);
            if taken >= max_cols {
                break;
            }
        }

        if !acc.is_empty() {
            out.push(StyledChunk {
                text: acc,
                style: ch.style,
            });
        }
    }

    out
}

pub fn clip_chunks_window_with_indicator_padded(
    chunks: &[StyledChunk],
    view_width: u16,
    indicator: char,
    start_cols: u16,
) -> Line<'static> {
    if view_width == 0 {
        return Line::from("");
    }

    let total = display_width_chunks(chunks);
    let show_left = start_cols > 0;
    let left_cols: u16 = if show_left { 1 } else { 0 };

    let cap_with_right = view_width.saturating_sub(left_cols + 1);
    let remaining = total.saturating_sub(start_cols);
    let show_right = remaining > cap_with_right;

    let max_visible = if show_right {
        cap_with_right
    } else {
        view_width.saturating_sub(left_cols)
    };

    let visible = slice_chunks_by_display_cols(chunks, start_cols, max_visible);
    let used_cols = left_cols + display_width_chunks(&visible);

    let mut spans: Vec<Span> = Vec::new();
    if show_left {
        spans.push(Span::raw(indicator.to_string()));
    }
    for v in visible {
        spans.push(Span::styled(v.text, v.style));
    }
    if show_right {
        let right_pos = view_width.saturating_sub(1);
        let filler = right_pos.saturating_sub(used_cols);
        if filler > 0 {
            spans.push(Span::raw(" ".repeat(filler as usize)));
        }
        spans.push(Span::raw(indicator.to_string()));
    }

    Line::from(spans)
}

pub fn wrap_chunks_indented(chunks: &[StyledChunk], width: u16, indent: u16) -> Vec<Line<'static>> {
    if width == 0 {
        return vec![Line::from("")];
    }
    let indent = indent.min(width.saturating_sub(1));
    let cont_prefix = continuation_prefix(width, indent);
    let cont_prefix_width = continuation_prefix_width(width, indent);
    let cont_cap = width.saturating_sub(cont_prefix_width);

    let mut lines: Vec<Line> = Vec::new();
    let mut current_spans: Vec<Span> = Vec::new();
    let mut used: u16 = 0;
    let mut first_line = true;

    for chunk in chunks {
        let mut buf = String::new();
        let mut buf_style = chunk.style;

        for ch in chunk.text.chars() {
            let w = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
            let cap = if first_line { width } else { cont_cap };

            if used > 0 && used.saturating_add(w) >= cap {
                if !buf.is_empty() {
                    current_spans.push(Span::styled(buf.clone(), buf_style));
                    buf.clear();
                }
                lines.push(Line::from(current_spans));
                current_spans = Vec::new();
                first_line = false;
                used = cont_prefix_width;

                current_spans.push(Span::raw(cont_prefix.clone()));
            }

            if !buf.is_empty() && buf_style != chunk.style {
                current_spans.push(Span::styled(buf.clone(), buf_style));
                buf.clear();
            }
            buf_style = chunk.style;

            if used == 0 && !first_line {
                current_spans.push(Span::raw(cont_prefix.clone()));
                used = cont_prefix_width;
            }

            buf.push(ch);
            used = used.saturating_add(w);
        }

        if !buf.is_empty() {
            current_spans.push(Span::styled(buf, buf_style));
        }
    }

    lines.push(Line::from(current_spans));
    lines
}
