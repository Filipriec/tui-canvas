// canvas/src/dispatcher.rs

use crate::canvas::state::CanvasState;
use crate::canvas::actions::{CanvasAction, ActionResult, execute_canvas_action};
use crate::config::CanvasConfig;

/// High-level action dispatcher that coordinates between different action types
pub struct ActionDispatcher;

impl ActionDispatcher {
    /// Dispatch any action to the appropriate handler
    pub async fn dispatch<S: CanvasState>(
        action: CanvasAction,
        state: &mut S,
        ideal_cursor_column: &mut usize,
    ) -> anyhow::Result<ActionResult> {

        // Load config once here instead of threading it everywhere
        execute_canvas_action(action, state, ideal_cursor_column, Some(&CanvasConfig::load())).await
    }

    /// Quick action dispatch from KeyCode
    pub async fn dispatch_key<S: CanvasState>(
        key: crossterm::event::KeyCode,
        state: &mut S,
        ideal_cursor_column: &mut usize,
    ) -> anyhow::Result<Option<ActionResult>> {
        if let Some(action) = CanvasAction::from_key(key) {
            let result = Self::dispatch(action, state, ideal_cursor_column).await?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    /// Batch dispatch multiple actions
    pub async fn dispatch_batch<S: CanvasState>(
        actions: Vec<CanvasAction>,
        state: &mut S,
        ideal_cursor_column: &mut usize,
    ) -> anyhow::Result<Vec<ActionResult>> {
        let mut results = Vec::new();
        for action in actions {
            let result = Self::dispatch(action, state, ideal_cursor_column).await?;
            let is_success = result.is_success(); // Check success before moving
            results.push(result);

            // Stop on first error
            if !is_success {
                break;
            }
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::CanvasAction;

    // Simple test implementation
    struct TestFormState {
        current_field: usize,
        cursor_pos: usize,
        inputs: Vec<String>,
        field_names: Vec<String>,
        has_changes: bool,
    }

    impl TestFormState {
        fn new() -> Self {
            Self {
                current_field: 0,
                cursor_pos: 0,
                inputs: vec!["".to_string(), "".to_string()],
                field_names: vec!["username".to_string(), "password".to_string()],
                has_changes: false,
            }
        }
    }

    impl CanvasState for TestFormState {
        fn current_field(&self) -> usize { self.current_field }
        fn current_cursor_pos(&self) -> usize { self.cursor_pos }
        fn set_current_field(&mut self, index: usize) { self.current_field = index; }
        fn set_current_cursor_pos(&mut self, pos: usize) { self.cursor_pos = pos; }

        fn get_current_input(&self) -> &str { &self.inputs[self.current_field] }
        fn get_current_input_mut(&mut self) -> &mut String { &mut self.inputs[self.current_field] }
        fn inputs(&self) -> Vec<&String> { self.inputs.iter().collect() }
        fn fields(&self) -> Vec<&str> { self.field_names.iter().map(|s| s.as_str()).collect() }

        fn has_unsaved_changes(&self) -> bool { self.has_changes }
        fn set_has_unsaved_changes(&mut self, changed: bool) { self.has_changes = changed; }

        // Custom action handling for testing
        fn handle_feature_action(&mut self, action: &CanvasAction, _context: &crate::state::ActionContext) -> Option<String> {
            match action {
                CanvasAction::Custom(s) if s == "test_custom" => {
                    Some("Custom action handled".to_string())
                }
                _ => None,
            }
        }
    }

    #[tokio::test]
    async fn test_typed_action_dispatch() {
        let mut state = TestFormState::new();
        let mut ideal_cursor = 0;

        // Test character insertion
        let result = ActionDispatcher::dispatch(
            CanvasAction::InsertChar('a'),
            &mut state,
            &mut ideal_cursor,
        ).await.unwrap();

        assert!(result.is_success());
        assert_eq!(state.get_current_input(), "a");
        assert_eq!(state.cursor_pos, 1);
        assert!(state.has_changes);
    }

    #[tokio::test]
    async fn test_key_dispatch() {
        let mut state = TestFormState::new();
        let mut ideal_cursor = 0;

        let result = ActionDispatcher::dispatch_key(
            crossterm::event::KeyCode::Char('b'),
            &mut state,
            &mut ideal_cursor,
        ).await.unwrap();

        assert!(result.is_some());
        assert!(result.unwrap().is_success());
        assert_eq!(state.get_current_input(), "b");
    }

    #[tokio::test]
    async fn test_custom_action() {
        let mut state = TestFormState::new();
        let mut ideal_cursor = 0;

        let result = ActionDispatcher::dispatch(
            CanvasAction::Custom("test_custom".to_string()),
            &mut state,
            &mut ideal_cursor,
        ).await.unwrap();

        match result {
            ActionResult::HandledByFeature(msg) => {
                assert_eq!(msg, "Custom action handled");
            }
            _ => panic!("Expected HandledByFeature result"),
        }
    }

    #[tokio::test]
    async fn test_batch_dispatch() {
        let mut state = TestFormState::new();
        let mut ideal_cursor = 0;

        let actions = vec![
            CanvasAction::InsertChar('h'),
            CanvasAction::InsertChar('i'),
            CanvasAction::MoveLeft,
            CanvasAction::InsertChar('e'),
        ];

        let results = ActionDispatcher::dispatch_batch(
            actions,
            &mut state,
            &mut ideal_cursor,
        ).await.unwrap();

        assert_eq!(results.len(), 4);
        assert!(results.iter().all(|r| r.is_success()));
        assert_eq!(state.get_current_input(), "hei");
    }
}
