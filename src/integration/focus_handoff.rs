// src/integration/focus_handoff.rs
//! Host-facing adapters for canvas boundary handoff.
//!
//! This module makes it explicit how to connect canvas movement boundaries
//! (`top` / `bottom`) to a parent application's focus manager.
//!
//! Use this module when you want a clear outside-world contract instead of
//! matching on low-level internals in your app code.

use crate::canvas::actions::{ActionResult, CanvasAction};
use crate::canvas::modes::AppMode;
use crate::editor::TextForm;
use crate::DataProvider;

/// Boundary reached while moving inside the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryExit {
    Top,
    Bottom,
}

/// Result returned when a host dispatches a typed canvas action.
#[derive(Debug, Clone)]
pub enum HostActionOutcome {
    Applied(ActionResult),
    ExitCanvas(BoundaryExit),
}

/// Execute a typed canvas action with explicit boundary handoff semantics.
///
/// Boundary exits are emitted only when:
/// - the action is `MoveUp`/`PrevField` at top or `MoveDown`/`NextField` at bottom
/// - editor is in `Nor` mode
pub fn execute_action_for_host<D: DataProvider>(
    editor: &mut TextForm<D>,
    action: CanvasAction,
) -> HostActionOutcome {
    execute_action_for_host_with_options(editor, action, true)
}

/// Same as [`execute_action_for_host`], but allows disabling boundary exits.
pub fn execute_action_for_host_with_options<D: DataProvider>(
    editor: &mut TextForm<D>,
    action: CanvasAction,
    allow_exit_in_read_only: bool,
) -> HostActionOutcome {
    let allow_exit = allow_exit_in_read_only && editor.mode() == AppMode::Nor;

    match action {
        CanvasAction::MoveDown | CanvasAction::NextField => {
            let last = editor.data_provider().field_count().saturating_sub(1);
            let at_bottom = editor.current_field() >= last;
            if allow_exit && at_bottom {
                HostActionOutcome::ExitCanvas(BoundaryExit::Bottom)
            } else {
                let _ = editor.move_down();
                HostActionOutcome::Applied(ActionResult::Success)
            }
        }
        CanvasAction::MoveUp | CanvasAction::PrevField => {
            let at_top = editor.current_field() == 0;
            if allow_exit && at_top {
                HostActionOutcome::ExitCanvas(BoundaryExit::Top)
            } else {
                let _ = editor.move_up();
                HostActionOutcome::Applied(ActionResult::Success)
            }
        }
        other => HostActionOutcome::Applied(editor.execute(other)),
    }
}

#[cfg(feature = "keybindings")]
use crate::keybindings::KeyEventOutcome;
#[cfg(feature = "keybindings")]
use crossterm::event::KeyEvent;

/// Host-friendly key processing outcome.
#[cfg(feature = "keybindings")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostKeyEventOutcome {
    Consumed(Option<String>),
    PendingSequence,
    NotHandled,
    ExitCanvas(BoundaryExit),
}

/// Convert a canvas `KeyEventOutcome` into host-friendly focus handoff output.
#[cfg(feature = "keybindings")]
pub fn map_key_event_outcome_for_host(outcome: KeyEventOutcome) -> HostKeyEventOutcome {
    match outcome {
        KeyEventOutcome::Consumed(msg) => HostKeyEventOutcome::Consumed(msg),
        KeyEventOutcome::Pending => HostKeyEventOutcome::PendingSequence,
        KeyEventOutcome::NotMatched => HostKeyEventOutcome::NotHandled,
        KeyEventOutcome::ExitTop => HostKeyEventOutcome::ExitCanvas(BoundaryExit::Top),
        KeyEventOutcome::ExitBottom => HostKeyEventOutcome::ExitCanvas(BoundaryExit::Bottom),
    }
}

/// Handle a key event and directly emit host-friendly handoff outcomes.
#[cfg(feature = "keybindings")]
pub fn handle_key_event_for_host<D: DataProvider>(
    editor: &mut TextForm<D>,
    evt: KeyEvent,
) -> HostKeyEventOutcome {
    let outcome = map_key_event_outcome_for_host(editor.handle_key_event(evt));

    // Preserve common host expectation: boundary exits are only meaningful for
    // outer focus handoff when the editor is in normal navigation mode.
    if editor.mode() != AppMode::Nor && matches!(outcome, HostKeyEventOutcome::ExitCanvas(_)) {
        HostKeyEventOutcome::NotHandled
    } else {
        outcome
    }
}

/// Extract only boundary exit information from a key outcome.
#[cfg(feature = "keybindings")]
pub fn boundary_from_key_outcome(outcome: &KeyEventOutcome) -> Option<BoundaryExit> {
    match outcome {
        KeyEventOutcome::ExitTop => Some(BoundaryExit::Top),
        KeyEventOutcome::ExitBottom => Some(BoundaryExit::Bottom),
        _ => None,
    }
}

/// Canonical mapping for vertical field navigation outcomes.
///
/// This is the single place in the crate that decides when a failed vertical
/// movement should become `ExitTop` / `ExitBottom`.
#[cfg(feature = "keybindings")]
pub fn key_outcome_for_vertical_navigation(moved: bool, boundary: BoundaryExit) -> KeyEventOutcome {
    if moved {
        KeyEventOutcome::Consumed(None)
    } else {
        match boundary {
            BoundaryExit::Top => KeyEventOutcome::ExitTop,
            BoundaryExit::Bottom => KeyEventOutcome::ExitBottom,
        }
    }
}
