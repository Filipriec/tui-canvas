// src/lib.rs
pub mod canvas;
pub mod editor;
pub mod data_provider;
pub mod integration;

#[cfg(feature = "suggestions")]
pub mod suggestions;

#[cfg(feature = "validation")]
pub mod validation;

#[cfg(feature = "textarea")]
pub mod textarea;

#[cfg(feature = "computed")]
pub mod computed;

#[cfg(feature = "keymap")]
pub mod keymap;

#[cfg(feature = "cursor-style")]
pub use canvas::CursorManager;

pub use editor::FormEditor;
pub use data_provider::DataProvider;
#[cfg(feature = "suggestions")]
pub use data_provider::{SuggestionsProvider, SuggestionItem, SuggestionTrigger};

pub use canvas::state::EditorState;
pub use canvas::modes::AppMode;

pub use canvas::actions::{CanvasAction, ActionResult};

#[cfg(feature = "validation")]
pub use validation::{
    ValidationConfig, ValidationResult, ValidationError,
    CharacterLimits, ValidationConfigBuilder, ValidationState,
    ValidationSummary, PatternFilters, PositionFilter, PositionRange, CharacterFilter, 
    DisplayMask,
    CustomFormatter, FormattingResult, PositionMapper, DefaultPositionMapper,
};

#[cfg(feature = "computed")]
pub use computed::{ComputedProvider, ComputedContext, ComputedState};

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
pub use textarea::{TextArea, TextAreaProvider, TextAreaState, TextAreaEditor};
