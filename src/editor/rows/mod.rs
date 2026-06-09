// src/editor/rows/mod.rs
//! Row-structure operations that genuinely differ between the two products.
//!
//! Everything that is identical regardless of row model lives as plain
//! primitives on [`crate::editor::EditorCore`] (see `editor/selection.rs`). The
//! operations here are the *only* ones that behave differently depending on
//! whether rows are a fixed set of slots or a growable buffer, so they are split
//! into two implementations instead of branching inside one function:
//!
//! - [`fixed`] — used by text **forms**: the row count never changes; edits
//!   clear/overwrite slots in place.
//! - [`dynamic`] — used by text **areas**: edits add, remove and merge rows.
//!
//! Each product calls its own variant directly; there is no policy branch.

#![cfg(feature = "keybindings")]

pub(crate) mod dynamic;
pub(crate) mod fixed;

/// Join register lines with `\n` and repeat the result `count` times. Shared by
/// both row models when turning a text register into pasteable text.
pub(crate) fn repeated_text(lines: &[String], count: usize) -> String {
    let repeat = count.max(1);
    let text = lines.join("\n");
    let mut pasted = String::new();
    for i in 0..repeat {
        if i > 0 {
            pasted.push('\n');
        }
        pasted.push_str(&text);
    }
    pasted
}
