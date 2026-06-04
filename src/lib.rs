// src/lib.rs

// Text modes are mutually exclusive: enable exactly one of `textmode-vim`
// (the default) or `textmode-normal`. The editing logic branches on
// `textmode-normal`, so enabling both would silently behave as normal mode and
// enabling neither would silently behave as vim mode — both contradict the
// documented contract, so reject them at compile time.
#[cfg(all(feature = "textmode-vim", feature = "textmode-normal"))]
compile_error!(
    "features `textmode-vim` and `textmode-normal` are mutually exclusive; \
     enable exactly one"
);
#[cfg(not(any(feature = "textmode-vim", feature = "textmode-normal")))]
compile_error!(
    "exactly one text mode must be enabled: `textmode-vim` (default) or \
     `textmode-normal`"
);

pub mod canvas;
pub mod data_provider;
pub mod editor;
pub mod integration;

#[cfg(feature = "gui")]
pub mod form;

#[cfg(feature = "textarea")]
pub mod textarea;

#[cfg(feature = "textinput")]
pub mod textinput;

#[cfg(feature = "cursor-style")]
pub mod cursor;

#[cfg(feature = "gui")]
mod gui_utils;

#[cfg(feature = "suggestions")]
pub mod suggestions;

#[cfg(feature = "validation")]
pub mod validation;

#[cfg(feature = "computed")]
pub mod computed;

#[cfg(feature = "keybindings")]
pub mod keybindings;

#[cfg(feature = "cursor-style")]
pub use cursor::CursorManager;

pub use data_provider::DataProvider;
#[cfg(feature = "suggestions")]
pub use data_provider::{SuggestionItem, SuggestionQuery, SuggestionTrigger};
#[cfg(feature = "crossterm")]
pub use editor::input::normal::FormInputEventOutcome;
pub use editor::FormEditor;

pub use canvas::modes::AppMode;
pub use canvas::state::EditorState;

pub use canvas::actions::{ActionResult, CanvasAction};

#[cfg(feature = "validation")]
pub use validation::{
    AppliedValidation, CharacterFilter, CharacterLimits, CustomFormatter, DefaultPositionMapper,
    DisplayMask, FormattingResult, PatternFilters, PositionFilter, PositionMapper, PositionRange,
    ValidationConfig, ValidationConfigBuilder, ValidationError, ValidationResult, ValidationRule,
    ValidationSet, ValidationSettings, ValidationState, ValidationSummary,
};

#[cfg(feature = "computed")]
pub use computed::{ComputedContext, ComputedProvider, ComputedState};

#[cfg(feature = "gui")]
pub use canvas::theme::{CanvasTheme, DefaultCanvasTheme};

#[cfg(feature = "gui")]
pub use form::{render_canvas, render_canvas_default};

#[cfg(feature = "gui")]
pub use form::render_canvas_with_options;

#[cfg(feature = "gui")]
pub use form::{CanvasDisplayOptions, OverflowMode};

#[cfg(all(feature = "gui", feature = "suggestions"))]
pub use suggestions::render::render_suggestions_dropdown;

#[cfg(feature = "keybindings")]
pub use keybindings::{
    default_builtin_action_bindings, default_emacs_action_bindings, default_helix_action_bindings,
    default_vim_action_bindings, preset, BuiltinCanvasKeybindingPreset, CanvasActionBinding,
    CanvasActionKeyBinding, CanvasKeyBindings, KeyEventOutcome,
};

#[cfg(feature = "textarea")]
pub use textarea::{
    TextArea, TextAreaDataProvider, TextAreaEditor, TextAreaLineNumberMode, TextAreaProvider,
    TextAreaState,
};

#[cfg(feature = "textinput")]
pub use textinput::{
    TextInput, TextInputDataProvider, TextInputEditor, TextInputEventOutcome, TextInputProvider,
    TextInputState,
};
