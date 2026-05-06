// src/lib.rs
pub mod canvas;
pub mod data_provider;
pub mod editor;
pub mod integration;

#[cfg(feature = "gui")]
mod gui_utils;

#[cfg(feature = "suggestions")]
pub mod suggestions;

#[cfg(feature = "validation")]
pub mod validation;

#[cfg(feature = "textarea")]
pub mod textarea;

#[cfg(feature = "textinput")]
pub mod textinput;

#[cfg(feature = "computed")]
pub mod computed;

#[cfg(feature = "keymap")]
pub mod keymap;

#[cfg(feature = "cursor-style")]
pub use canvas::CursorManager;

pub use data_provider::DataProvider;
#[cfg(feature = "suggestions")]
pub use data_provider::{SuggestionItem, SuggestionQuery, SuggestionTrigger};
#[cfg(feature = "crossterm")]
pub use editor::event_input::FormInputEventOutcome;
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
pub use canvas::gui::{render_canvas, render_canvas_default};

#[cfg(feature = "gui")]
pub use canvas::gui::render_canvas_with_options;

#[cfg(feature = "gui")]
pub use canvas::gui::{CanvasDisplayOptions, OverflowMode};

#[cfg(all(feature = "gui", feature = "suggestions"))]
pub use suggestions::gui::render_suggestions_dropdown;

#[cfg(feature = "keymap")]
pub use keymap::{CanvasKeyMap, KeyEventOutcome};

#[cfg(feature = "textarea")]
pub use textarea::{
    TextArea, TextAreaDataProvider, TextAreaEditor, TextAreaProvider, TextAreaState,
};

#[cfg(feature = "textinput")]
pub use textinput::{
    TextInput, TextInputDataProvider, TextInputEditor, TextInputEventOutcome, TextInputProvider,
    TextInputState,
};
