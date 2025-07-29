// examples/integration_patterns.rs
//! Advanced integration patterns showing how Canvas works with:
//! - State management patterns
//! - Event-driven architectures
//! - Validation systems
//! - Custom rendering
//!
//! Run with: cargo run --example integration_patterns

use canvas::prelude::*;
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    println!("🔧 Canvas Integration Patterns");
    println!("==============================\n");

    // Pattern 1: State machine integration
    state_machine_example().await;

    // Pattern 2: Event-driven architecture
    event_driven_example().await;

    // Pattern 3: Validation pipeline
    validation_pipeline_example().await;

    // Pattern 4: Multi-form orchestration
    multi_form_example().await;
}

// Pattern 1: Canvas with state machine
async fn state_machine_example() {
    println!("🔄 Pattern 1: State Machine Integration");

    #[derive(Debug, Clone, PartialEq)]
    enum FormState {
        Initial,
        Editing,
        Validating,
        Submitting,
        Success,
        Error(String),
    }

    #[derive(Debug)]
    struct StateMachineForm {
        // Canvas state
        current_field: usize,
        cursor_pos: usize,
        username: String,
        password: String,
        has_changes: bool,

        // State machine
        state: FormState,
    }

    impl StateMachineForm {
        fn new() -> Self {
            Self {
                current_field: 0,
                cursor_pos: 0,
                username: String::new(),
                password: String::new(),
                has_changes: false,
                state: FormState::Initial,
            }
        }

        fn transition_to(&mut self, new_state: FormState) -> String {
            let old_state = self.state.clone();
            self.state = new_state;
            format!("State transition: {:?} -> {:?}", old_state, self.state)
        }

        fn can_submit(&self) -> bool {
            matches!(self.state, FormState::Editing) &&
            !self.username.trim().is_empty() &&
            !self.password.trim().is_empty()
        }
    }

    impl CanvasState for StateMachineForm {
        fn current_field(&self) -> usize { self.current_field }
        fn current_cursor_pos(&self) -> usize { self.cursor_pos }
        fn set_current_field(&mut self, index: usize) { self.current_field = index.min(1); }
        fn set_current_cursor_pos(&mut self, pos: usize) { self.cursor_pos = pos; }

        fn get_current_input(&self) -> &str {
            match self.current_field {
                0 => &self.username,
                1 => &self.password,
                _ => "",
            }
        }

        fn get_current_input_mut(&mut self) -> &mut String {
            match self.current_field {
                0 => &mut self.username,
                1 => &mut self.password,
                _ => unreachable!(),
            }
        }

        fn inputs(&self) -> Vec<&String> { vec![&self.username, &self.password] }
        fn fields(&self) -> Vec<&str> { vec!["Username", "Password"] }
        fn has_unsaved_changes(&self) -> bool { self.has_changes }

        fn set_has_unsaved_changes(&mut self, changed: bool) {
            self.has_changes = changed;
            // Transition to editing state when user starts typing
            if changed && self.state == FormState::Initial {
                self.state = FormState::Editing;
            }
        }

        fn handle_feature_action(&mut self, action: &CanvasAction, _context: &ActionContext) -> Option<String> {
            match action {
                CanvasAction::Custom(cmd) => match cmd.as_str() {
                    "submit" => {
                        if self.can_submit() {
                            let msg = self.transition_to(FormState::Submitting);
                            // Simulate submission
                            self.state = FormState::Success;
                            Some(format!("{} -> Form submitted successfully", msg))
                        } else {
                            let msg = self.transition_to(FormState::Error("Invalid form data".to_string()));
                            Some(msg)
                        }
                    }
                    "reset" => {
                        self.username.clear();
                        self.password.clear();
                        self.has_changes = false;
                        Some(self.transition_to(FormState::Initial))
                    }
                    _ => None,
                },
                _ => None,
            }
        }
    }

    let mut form = StateMachineForm::new();
    let mut ideal_cursor = 0;

    println!("  Initial state: {:?}", form.state);

    // Type some text to trigger state change
    let _result = ActionDispatcher::dispatch(
        CanvasAction::InsertChar('u'),
        &mut form,
        &mut ideal_cursor,
    ).await.unwrap();
    println!("  After typing: {:?}", form.state);

    // Try to submit (should fail)
    let result = ActionDispatcher::dispatch(
        CanvasAction::Custom("submit".to_string()),
        &mut form,
        &mut ideal_cursor,
    ).await.unwrap();
    println!("  Submit result: {}", result.message().unwrap_or(""));
    println!("  ✅ State machine integration works!\n");
}

// Pattern 2: Event-driven architecture
async fn event_driven_example() {
    println!("📡 Pattern 2: Event-Driven Architecture");

    #[derive(Debug, Clone)]
    enum FormEvent {
        FieldChanged { field: usize, old_value: String, new_value: String },
        ValidationTriggered { field: usize, is_valid: bool },
        ActionExecuted { action: String, success: bool },
    }

    #[derive(Debug)]
    struct EventDrivenForm {
        current_field: usize,
        cursor_pos: usize,
        email: String,
        has_changes: bool,
        events: Vec<FormEvent>,
    }

    impl EventDrivenForm {
        fn new() -> Self {
            Self {
                current_field: 0,
                cursor_pos: 0,
                email: String::new(),
                has_changes: false,
                events: Vec::new(),
            }
        }

        fn emit_event(&mut self, event: FormEvent) {
            println!("  📡 Event: {:?}", event);
            self.events.push(event);
        }

        fn validate_email(&self) -> bool {
            self.email.contains('@') && self.email.contains('.')
        }
    }

    impl CanvasState for EventDrivenForm {
        fn current_field(&self) -> usize { self.current_field }
        fn current_cursor_pos(&self) -> usize { self.cursor_pos }
        fn set_current_field(&mut self, index: usize) { self.current_field = index; }
        fn set_current_cursor_pos(&mut self, pos: usize) { self.cursor_pos = pos; }

        fn get_current_input(&self) -> &str { &self.email }
        fn get_current_input_mut(&mut self) -> &mut String { &mut self.email }
        fn inputs(&self) -> Vec<&String> { vec![&self.email] }
        fn fields(&self) -> Vec<&str> { vec!["Email"] }
        fn has_unsaved_changes(&self) -> bool { self.has_changes }

        fn set_has_unsaved_changes(&mut self, changed: bool) {
            if changed != self.has_changes {
                let old_value = if self.has_changes { "modified" } else { "unmodified" };
                let new_value = if changed { "modified" } else { "unmodified" };

                self.emit_event(FormEvent::FieldChanged {
                    field: self.current_field,
                    old_value: old_value.to_string(),
                    new_value: new_value.to_string(),
                });
            }
            self.has_changes = changed;
        }

        fn handle_feature_action(&mut self, action: &CanvasAction, _context: &ActionContext) -> Option<String> {
            match action {
                CanvasAction::Custom(cmd) => match cmd.as_str() {
                    "validate" => {
                        let is_valid = self.validate_email();
                        self.emit_event(FormEvent::ValidationTriggered {
                            field: self.current_field,
                            is_valid,
                        });

                        self.emit_event(FormEvent::ActionExecuted {
                            action: "validate".to_string(),
                            success: true,
                        });

                        if is_valid {
                            Some("Email is valid!".to_string())
                        } else {
                            Some("Email is invalid".to_string())
                        }
                    }
                    _ => None,
                },
                _ => None,
            }
        }
    }

    let mut form = EventDrivenForm::new();
    let mut ideal_cursor = 0;

    // Type an email address
    let email = "user@example.com";
    for c in email.chars() {
        ActionDispatcher::dispatch(
            CanvasAction::InsertChar(c),
            &mut form,
            &mut ideal_cursor,
        ).await.unwrap();
    }

    // Validate the email
    let result = ActionDispatcher::dispatch(
        CanvasAction::Custom("validate".to_string()),
        &mut form,
        &mut ideal_cursor,
    ).await.unwrap();

    println!("  Final email: {}", form.email);
    println!("  Validation result: {}", result.message().unwrap_or(""));
    println!("  Total events captured: {}", form.events.len());
    println!("  ✅ Event-driven architecture works!\n");
}

// Pattern 3: Validation pipeline
async fn validation_pipeline_example() {
    println!("✅ Pattern 3: Validation Pipeline");

    type ValidationRule = Box<dyn Fn(&str) -> Result<(), String>>;

    // Custom Debug implementation since function pointers don't implement Debug
    struct ValidatedForm {
        current_field: usize,
        cursor_pos: usize,
        password: String,
        has_changes: bool,
        validators: HashMap<usize, Vec<ValidationRule>>,
    }

    impl std::fmt::Debug for ValidatedForm {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("ValidatedForm")
                .field("current_field", &self.current_field)
                .field("cursor_pos", &self.cursor_pos)
                .field("password", &self.password)
                .field("has_changes", &self.has_changes)
                .field("validators", &format!("HashMap with {} entries", self.validators.len()))
                .finish()
        }
    }

    impl ValidatedForm {
        fn new() -> Self {
            let mut validators: HashMap<usize, Vec<ValidationRule>> = HashMap::new();

            // Password validators
            let mut password_validators: Vec<ValidationRule> = Vec::new();
            password_validators.push(Box::new(|value| {
                if value.len() < 8 {
                    Err("Password must be at least 8 characters".to_string())
                } else {
                    Ok(())
                }
            }));
            password_validators.push(Box::new(|value| {
                if !value.chars().any(|c| c.is_uppercase()) {
                    Err("Password must contain at least one uppercase letter".to_string())
                } else {
                    Ok(())
                }
            }));
            password_validators.push(Box::new(|value| {
                if !value.chars().any(|c| c.is_numeric()) {
                    Err("Password must contain at least one number".to_string())
                } else {
                    Ok(())
                }
            }));

            validators.insert(0, password_validators);

            Self {
                current_field: 0,
                cursor_pos: 0,
                password: String::new(),
                has_changes: false,
                validators,
            }
        }

        fn validate_field(&self, field_index: usize) -> Vec<String> {
            let mut errors = Vec::new();

            if let Some(validators) = self.validators.get(&field_index) {
                let value = match field_index {
                    0 => &self.password,
                    _ => return errors,
                };

                for validator in validators {
                    if let Err(error) = validator(value) {
                        errors.push(error);
                    }
                }
            }

            errors
        }
    }

    impl CanvasState for ValidatedForm {
        fn current_field(&self) -> usize { self.current_field }
        fn current_cursor_pos(&self) -> usize { self.cursor_pos }
        fn set_current_field(&mut self, index: usize) { self.current_field = index; }
        fn set_current_cursor_pos(&mut self, pos: usize) { self.cursor_pos = pos; }

        fn get_current_input(&self) -> &str { &self.password }
        fn get_current_input_mut(&mut self) -> &mut String { &mut self.password }
        fn inputs(&self) -> Vec<&String> { vec![&self.password] }
        fn fields(&self) -> Vec<&str> { vec!["Password"] }
        fn has_unsaved_changes(&self) -> bool { self.has_changes }
        fn set_has_unsaved_changes(&mut self, changed: bool) { self.has_changes = changed; }

        fn handle_feature_action(&mut self, action: &CanvasAction, _context: &ActionContext) -> Option<String> {
            match action {
                CanvasAction::Custom(cmd) => match cmd.as_str() {
                    "validate" => {
                        let errors = self.validate_field(self.current_field);
                        if errors.is_empty() {
                            Some("Password meets all requirements!".to_string())
                        } else {
                            Some(format!("Validation errors: {}", errors.join(", ")))
                        }
                    }
                    _ => None,
                },
                _ => None,
            }
        }
    }

    let mut form = ValidatedForm::new();
    let mut ideal_cursor = 0;

    // Test with weak password
    let weak_password = "abc";
    for c in weak_password.chars() {
        ActionDispatcher::dispatch(
            CanvasAction::InsertChar(c),
            &mut form,
            &mut ideal_cursor,
        ).await.unwrap();
    }

    let result = ActionDispatcher::dispatch(
        CanvasAction::Custom("validate".to_string()),
        &mut form,
        &mut ideal_cursor,
    ).await.unwrap();
    println!("  Weak password '{}': {}", form.password, result.message().unwrap_or(""));

    // Clear and test with strong password
    form.password.clear();
    form.cursor_pos = 0;

    let strong_password = "StrongPass123";
    for c in strong_password.chars() {
        ActionDispatcher::dispatch(
            CanvasAction::InsertChar(c),
            &mut form,
            &mut ideal_cursor,
        ).await.unwrap();
    }

    let result = ActionDispatcher::dispatch(
        CanvasAction::Custom("validate".to_string()),
        &mut form,
        &mut ideal_cursor,
    ).await.unwrap();
    println!("  Strong password '{}': {}", form.password, result.message().unwrap_or(""));
    println!("  ✅ Validation pipeline works!\n");
}

// Pattern 4: Multi-form orchestration
async fn multi_form_example() {
    println!("🎭 Pattern 4: Multi-Form Orchestration");

    #[derive(Debug)]
    struct PersonalInfoForm {
        current_field: usize,
        cursor_pos: usize,
        name: String,
        age: String,
        has_changes: bool,
    }

    #[derive(Debug)]
    struct ContactInfoForm {
        current_field: usize,
        cursor_pos: usize,
        email: String,
        phone: String,
        has_changes: bool,
    }

    // Implement CanvasState for both forms
    impl CanvasState for PersonalInfoForm {
        fn current_field(&self) -> usize { self.current_field }
        fn current_cursor_pos(&self) -> usize { self.cursor_pos }
        fn set_current_field(&mut self, index: usize) { self.current_field = index.min(1); }
        fn set_current_cursor_pos(&mut self, pos: usize) { self.cursor_pos = pos; }

        fn get_current_input(&self) -> &str {
            match self.current_field {
                0 => &self.name,
                1 => &self.age,
                _ => "",
            }
        }

        fn get_current_input_mut(&mut self) -> &mut String {
            match self.current_field {
                0 => &mut self.name,
                1 => &mut self.age,
                _ => unreachable!(),
            }
        }

        fn inputs(&self) -> Vec<&String> { vec![&self.name, &self.age] }
        fn fields(&self) -> Vec<&str> { vec!["Name", "Age"] }
        fn has_unsaved_changes(&self) -> bool { self.has_changes }
        fn set_has_unsaved_changes(&mut self, changed: bool) { self.has_changes = changed; }
    }

    impl CanvasState for ContactInfoForm {
        fn current_field(&self) -> usize { self.current_field }
        fn current_cursor_pos(&self) -> usize { self.cursor_pos }
        fn set_current_field(&mut self, index: usize) { self.current_field = index.min(1); }
        fn set_current_cursor_pos(&mut self, pos: usize) { self.cursor_pos = pos; }

        fn get_current_input(&self) -> &str {
            match self.current_field {
                0 => &self.email,
                1 => &self.phone,
                _ => "",
            }
        }

        fn get_current_input_mut(&mut self) -> &mut String {
            match self.current_field {
                0 => &mut self.email,
                1 => &mut self.phone,
                _ => unreachable!(),
            }
        }

        fn inputs(&self) -> Vec<&String> { vec![&self.email, &self.phone] }
        fn fields(&self) -> Vec<&str> { vec!["Email", "Phone"] }
        fn has_unsaved_changes(&self) -> bool { self.has_changes }
        fn set_has_unsaved_changes(&mut self, changed: bool) { self.has_changes = changed; }
    }

    // Form orchestrator
    #[derive(Debug)]
    struct FormOrchestrator {
        personal_form: PersonalInfoForm,
        contact_form: ContactInfoForm,
        current_form: usize, // 0 = personal, 1 = contact
    }

    impl FormOrchestrator {
        fn new() -> Self {
            Self {
                personal_form: PersonalInfoForm {
                    current_field: 0,
                    cursor_pos: 0,
                    name: String::new(),
                    age: String::new(),
                    has_changes: false,
                },
                contact_form: ContactInfoForm {
                    current_field: 0,
                    cursor_pos: 0,
                    email: String::new(),
                    phone: String::new(),
                    has_changes: false,
                },
                current_form: 0,
            }
        }

        async fn execute_action(&mut self, action: CanvasAction) -> ActionResult {
            let mut ideal_cursor = 0;

            match self.current_form {
                0 => ActionDispatcher::dispatch(action, &mut self.personal_form, &mut ideal_cursor).await.unwrap(),
                1 => ActionDispatcher::dispatch(action, &mut self.contact_form, &mut ideal_cursor).await.unwrap(),
                _ => ActionResult::error("Invalid form index"),
            }
        }

        fn switch_form(&mut self) -> String {
            self.current_form = (self.current_form + 1) % 2;
            match self.current_form {
                0 => "Switched to Personal Info form".to_string(),
                1 => "Switched to Contact Info form".to_string(),
                _ => "Unknown form".to_string(),
            }
        }

        fn current_form_name(&self) -> &str {
            match self.current_form {
                0 => "Personal Info",
                1 => "Contact Info",
                _ => "Unknown",
            }
        }
    }

    let mut orchestrator = FormOrchestrator::new();

    println!("  Current form: {}", orchestrator.current_form_name());

    // Fill personal info
    for &c in &['J', 'o', 'h', 'n'] {
        orchestrator.execute_action(CanvasAction::InsertChar(c)).await;
    }

    orchestrator.execute_action(CanvasAction::NextField).await;

    for &c in &['2', '5'] {
        orchestrator.execute_action(CanvasAction::InsertChar(c)).await;
    }

    println!("  Personal form - Name: '{}', Age: '{}'",
             orchestrator.personal_form.name,
             orchestrator.personal_form.age);

    // Switch to contact form
    let switch_msg = orchestrator.switch_form();
    println!("  {}", switch_msg);

    // Fill contact info
    for &c in &['j', 'o', 'h', 'n', '@', 'e', 'x', 'a', 'm', 'p', 'l', 'e', '.', 'c', 'o', 'm'] {
        orchestrator.execute_action(CanvasAction::InsertChar(c)).await;
    }

    orchestrator.execute_action(CanvasAction::NextField).await;

    for &c in &['5', '5', '5', '-', '1', '2', '3', '4'] {
        orchestrator.execute_action(CanvasAction::InsertChar(c)).await;
    }

    println!("  Contact form - Email: '{}', Phone: '{}'",
             orchestrator.contact_form.email,
             orchestrator.contact_form.phone);
}
