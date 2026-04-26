#[cfg(feature = "gui")]
use ratatui::style::Style;

#[cfg(feature = "gui")]
use ratatui::text::{Line, Span};

#[cfg(feature = "gui")]
use unicode_width::UnicodeWidthChar;

#[cfg(feature = "gui")]
pub(crate) const RIGHT_PAD: u16 = 3;

#[cfg(feature = "gui")]
pub(crate) fn display_width(s: &str) -> u16 {
    s.chars()
        .map(|c| UnicodeWidthChar::width(c).unwrap_or(0) as u16)
        .sum()
}

#[cfg(feature = "gui")]
pub(crate) fn display_cols_up_to(s: &str, char_count: usize) -> u16 {
    let mut cols: u16 = 0;
    for (i, ch) in s.chars().enumerate() {
        if i >= char_count {
            break;
        }
        cols = cols.saturating_add(UnicodeWidthChar::width(ch).unwrap_or(0) as u16);
    }
    cols
}

#[cfg(feature = "gui")]
pub(crate) fn slice_by_display_cols(
    s: &str,
    start_cols: u16,
    max_cols: u16,
) -> String {
    if max_cols == 0 {
        return String::new();
    }

    let mut cols: u16 = 0;
    let mut out = String::new();
    let mut taken: u16 = 0;
    let mut started = false;

    for ch in s.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
        let next = cols.saturating_add(w);

        if !started {
            if next <= start_cols {
                cols = next;
                continue;
            }
            started = true;
        }

        if taken.saturating_add(w) > max_cols {
            break;
        }

        out.push(ch);
        taken = taken.saturating_add(w);
        cols = next;
    }

    out
}

#[cfg(feature = "gui")]
pub(crate) fn compute_h_scroll_with_padding(
    cursor_cols: u16,
    width: u16,
) -> (u16, u16) {
    let mut h = 0u16;
    for _ in 0..2 {
        let left_cols = if h > 0 { 1 } else { 0 };
        let max_x_visible = width.saturating_sub(1 + RIGHT_PAD + left_cols);
        let needed = cursor_cols.saturating_sub(max_x_visible);
        if needed <= h {
            return (h, left_cols);
        }
        h = needed;
    }
    let left_cols = if h > 0 { 1 } else { 0 };
    (h, left_cols)
}

#[cfg(feature = "gui")]
pub(crate) fn clip_window_with_indicator_padded(
    text: &str,
    view_width: u16,
    indicator: char,
    start_cols: u16,
) -> Line<'static> {
    if view_width == 0 {
        return Line::from("");
    }

    let total = display_width(text);
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

    let visible = slice_by_display_cols(text, start_cols, max_visible);

    let mut spans: Vec<Span> = Vec::new();
    if show_left {
        spans.push(Span::raw(indicator.to_string()));
    }

    spans.push(Span::raw(visible.clone()));

    if show_right {
        let used_cols = left_cols + display_width(&visible);
        let right_pos = view_width.saturating_sub(1);
        let filler = right_pos.saturating_sub(used_cols);
        if filler > 0 {
            spans.push(Span::raw(" ".repeat(filler as usize)));
        }
        spans.push(Span::raw(indicator.to_string()));
    }

    Line::from(spans)
}

#[cfg(feature = "gui")]
pub(crate) fn clip_inline_completion_with_indicator_padded(
    typed_text: &str,
    completion: Option<&str>,
    view_width: u16,
    indicator: char,
    start_cols: u16,
    typed_style: Style,
    completion_style: Style,
) -> Line<'static> {
    if view_width == 0 {
        return Line::from("");
    }

    let total = display_width(typed_text);
    let show_left = start_cols > 0;
    let left_cols: u16 = if show_left { 1 } else { 0 };
    let cap_with_right = view_width.saturating_sub(left_cols + 1);
    let remaining = total.saturating_sub(start_cols);
    let show_right = remaining > cap_with_right;
    let right_cols: u16 = if show_right { 1 } else { 0 };
    let visible_cols = view_width.saturating_sub(left_cols + right_cols);

    let visible_typed = slice_by_display_cols(typed_text, start_cols, visible_cols);
    let used_typed_cols = display_width(&visible_typed);
    let remaining_cols = visible_cols.saturating_sub(used_typed_cols);

    let mut visible_completion = String::new();
    if let Some(comp) = completion {
        if !comp.is_empty() && remaining_cols > 0 {
            visible_completion = slice_by_display_cols(comp, 0, remaining_cols);
        }
    }

    let mut spans: Vec<Span> = Vec::with_capacity(4);
    if show_left {
        spans.push(Span::raw(indicator.to_string()));
    }

    if !visible_typed.is_empty() {
        spans.push(Span::styled(visible_typed, typed_style));
    }

    if !visible_completion.is_empty() {
        spans.push(Span::styled(visible_completion, completion_style));
    }

    if show_right {
        spans.push(Span::raw(indicator.to_string()));
    }

    Line::from(spans)
}
