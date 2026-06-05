# Helix Support Gaps

This file tracks the difference between the current textarea Helix mode and
the full Helix editor keymap/behavior.

## Implemented Core

- Modal Helix preset installation with `BuiltinCanvasKeybindingPreset::Helix`.
- Primary selection in normal mode.
- Basic movement: `h`, `j`, `k`, `l`, arrows, `w`, `b`, `e`, `W`, `B`, `E`.
- Goto line motions supported by this widget: `gh`, `gl`, `gg`, `ge`.
- Basic changes: `d`, `Alt-d`, `c`, `Alt-c`, `y`, `p`, `P`, `u`, `U`.
- Basic line selection: `x`, `X`, `;`.
- Insert entry: `i`, `a`, `I`, `A`, `o`, `O`.

## Needs Helix Semantics

- Normal-mode motions should replace the primary selection with the moved-over
  object/range. Word motions (`w`, `b`, `e`, `W`, `B`, `E`) are implemented
  this way; other motions need the same audit.
- Select/extend mode should extend selections consistently for all supported
  motions, including goto motions.
- `x` and `X` should exactly match Helix line-extension behavior for repeated
  use and existing character selections.
- Yank/delete/change/paste need more register-aware behavior to match Helix.

## Missing Supported-Widget Actions

These are reasonable to add to the textarea without needing LSP, pickers, or
multi-buffer editor infrastructure.

- Find motions: `f`, `F`, `t`, `T`, `Alt-.`.
- Page motions: `Ctrl-b`, `Ctrl-f`, `PageUp`, `PageDown`.
- Goto mode: `g|`, `gs`, `gj`, `gk`.
- Search: `/`, `?`, `n`, `N`, `*`.
- Insert mode: `Ctrl-j`, `Ctrl-w`, `Alt-d`, `Ctrl-u`, `Ctrl-r`.
- Replace and case operations: `r`, `R`, `~`, backtick, `Alt-backtick`.
- Indent/unindent where applicable: `>`, `<`.

## Missing Full Helix Features

These require larger data-model or application support beyond a simple textarea.

- Multiple selections.
- Register selection with `"`.
- Selection manipulation: `s`, `S`, `Alt-s`, `&`, `_`, `,`, `Alt-,`,
  `Alt-;`, `Alt-:`, `C`.
- Match mode `m`, text objects, and surround commands.
- View mode `z` / `Z`.
- Window mode `Ctrl-w`.
- Space mode pickers and clipboard/system commands.
- LSP and tree-sitter commands.
- Buffer, file, jumplist, diagnostics, and symbol pickers.
