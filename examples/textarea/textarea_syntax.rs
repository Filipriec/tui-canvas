// examples/textarea_syntax.rs
//! Demonstrates syntax highlighting with the textarea widget
//!
//! This example REQUIRES the `syntect` feature to compile.
//!
//! Run with:
//! cargo run --example textarea_syntax --features "gui,cursor-style,textarea,syntect,textmode-normal"

#[cfg(not(feature = "syntect"))]
compile_error!(
    "This example requires the 'syntect' feature. \
     Run with: cargo run --example textarea_syntax --features \"gui,cursor-style,textarea,syntect,textmode-normal\""
);

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::io;

use canvas::{
    canvas::CursorManager,
    textarea::highlight::{TextAreaSyntax, TextAreaSyntaxState},
};

/// Syntax highlighting TextArea demo
struct SyntaxTextAreaDemo {
    textarea: TextAreaSyntaxState,
    has_unsaved_changes: bool,
    debug_message: String,
    current_language: String,
    current_theme: String,
}

impl SyntaxTextAreaDemo {
    fn new() -> Self {
        let initial_text = r#"// Multi-language Syntax Highlighting Demo
// Rust
fn main() {
    println!("Hello, Rust 🦀");
    let nums = vec![1, 2, 3, 4, 5];
    for n in nums {
        println!("n = {}", n);
    }
}

// Python
# 🐍 Python example
def fib(n):
    if n <= 1:
        return n
    return fib(n-1) + fib(n-2)

print([fib(i) for i in range(6)])

# ==========================
// JavaScript
// JavaScript example
function greet(name) {
    console.log(`Hello, ${name}!`);
}
greet("World");

// Scheme
;; Scheme example
(define (square x) (* x x))
(display (square 5))
(newline)
"#;

        let mut textarea = TextAreaSyntaxState::from_text(initial_text);
        textarea.set_placeholder("Start typing code...");

        // Pick a colorful default theme
        let default_theme = "base16-ocean.dark";
        let _ = textarea.set_syntax_theme(default_theme);
        // Default to Rust syntax
        let _ = textarea.set_syntax_by_extension("rs");

        Self {
            textarea,
            has_unsaved_changes: false,
            debug_message: format!("🎯 Syntax highlighting enabled - Rust ({})", default_theme),
            current_language: "Rust".to_string(),
            current_theme: default_theme.to_string(),
        }
    }

    fn handle_textarea_input(&mut self, key: KeyEvent) {
        self.textarea.input(key);
        self.has_unsaved_changes = true;
    }

    fn switch_to_rust(&mut self) {
        let _ = self.textarea.set_syntax_by_extension("rs");
        self.current_language = "Rust".to_string();
        self.debug_message = format!("🦀 Switched to Rust syntax ({})", self.current_theme);

        let rust_code = r#"// Rust example
fn fibonacci(n: u32) -> u32 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() {
    for i in 0..10 {
        println!("fib({}) = {}", i, fibonacci(i));
    }
}"#;
        self.textarea.set_text(rust_code);
        self.has_unsaved_changes = false;
    }

    fn switch_to_python(&mut self) {
        let _ = self.textarea.set_syntax_by_extension("py");
        self.current_language = "Python".to_string();
        self.debug_message = format!("🐍 Switched to Python syntax ({})", self.current_theme);

        let python_code = r#"# Python example
def fibonacci(n):
    if n <= 1:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)

def main():
    for i in range(10):
        print(f"fib({i}) = {fibonacci(i)}")

if __name__ == "__main__":
    main()"#;
        self.textarea.set_text(python_code);
        self.has_unsaved_changes = false;
    }

    fn switch_to_javascript(&mut self) {
        let _ = self.textarea.set_syntax_by_extension("js");
        self.current_language = "JavaScript".to_string();
        self.debug_message = format!("🟨 Switched to JavaScript syntax ({})", self.current_theme);

        let js_code = r#"// JavaScript example
function fibonacci(n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

function main() {
    for (let i = 0; i < 10; i++) {
        console.log(`fib(${i}) = ${fibonacci(i)}`);
    }
}

main();"#;
        self.textarea.set_text(js_code);
        self.has_unsaved_changes = false;
    }

    fn switch_to_scheme(&mut self) {
        let _ = self.textarea.set_syntax_by_name("Scheme");
        self.current_language = "Scheme".to_string();
        self.debug_message = format!("🎭 Switched to Scheme syntax ({})", self.current_theme);

        let scheme_code = r#";; Scheme example
(define (fibonacci n)
  (cond ((= n 0) 0)
        ((= n 1) 1)
        (else (+ (fibonacci (- n 1))
                 (fibonacci (- n 2))))))

(define (main)
  (do ((i 0 (+ i 1)))
      ((= i 10))
    (display (format "fib(~a) = ~a~n" i (fibonacci i)))))

(main)"#;
        self.textarea.set_text(scheme_code);
        self.has_unsaved_changes = false;
    }

    fn cycle_theme(&mut self) {
        let themes = [
            "InspiredGitHub",
            "base16-ocean.dark",
            "base16-eighties.dark",
            "Solarized (dark)",
            "Monokai Extended",
        ];
        let current_pos = themes.iter().position(|t| *t == self.current_theme);
        let next_pos = match current_pos {
            Some(p) => (p + 1) % themes.len(),
            None => 0,
        };
        let next_theme = themes[next_pos];
        let _ = self.textarea.set_syntax_theme(next_theme);
        self.current_theme = next_theme.to_string();
        self.debug_message = format!("🎨 Theme switched to {}", next_theme);
    }

    fn get_cursor_info(&self) -> String {
        format!(
            "Line {}, Col {} | Lang: {} | Theme: {}",
            self.textarea.current_field() + 1,
            self.textarea.cursor_position() + 1,
            self.current_language,
            self.current_theme
        )
    }

    fn debug_message(&self) -> &str {
        &self.debug_message
    }

    fn set_debug_message(&mut self, msg: String) {
        self.debug_message = msg;
    }

    fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes
    }
}

fn handle_key_press(key_event: KeyEvent, editor: &mut SyntaxTextAreaDemo) -> anyhow::Result<bool> {
    let KeyEvent {
        code: key,
        modifiers,
        ..
    } = key_event;

    // Quit
    if (key == KeyCode::Char('q') && modifiers.contains(KeyModifiers::CONTROL))
        || (key == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL))
        || key == KeyCode::F(10)
    {
        return Ok(false);
    }

    match (key, modifiers) {
        // Language switching
        (KeyCode::F(5), _) => editor.switch_to_rust(),
        (KeyCode::F(6), _) => editor.switch_to_python(),
        (KeyCode::F(7), _) => editor.switch_to_javascript(),
        (KeyCode::F(8), _) => editor.switch_to_scheme(),

        // Theme cycling
        (KeyCode::F(9), _) => editor.cycle_theme(),

        // Overflow modes
        (KeyCode::F(1), _) => {
            editor.textarea.use_overflow_indicator('$');
            editor.set_debug_message(format!(
                "Overflow: indicator '$' (wrap OFF) | Theme: {}",
                editor.current_theme
            ));
        }
        (KeyCode::F(2), _) => {
            editor.textarea.use_wrap();
            editor.set_debug_message(format!(
                "Overflow: wrap ON | Theme: {}",
                editor.current_theme
            ));
        }

        // Wrap indent
        (KeyCode::F(3), _) => {
            editor.textarea.set_wrap_indent_cols(4);
            editor.set_debug_message(format!(
                "Wrap indent: 4 columns | Theme: {}",
                editor.current_theme
            ));
        }
        (KeyCode::F(4), _) => {
            editor.textarea.set_wrap_indent_cols(0);
            editor.set_debug_message(format!(
                "Wrap indent: 0 columns | Theme: {}",
                editor.current_theme
            ));
        }

        // Info
        (KeyCode::Char('?'), _) => {
            editor.set_debug_message(format!(
                "{} | Syntax highlighting enabled",
                editor.get_cursor_info()
            ));
        }

        // Default: pass to textarea
        _ => editor.handle_textarea_input(key_event),
    }

    Ok(true)
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut editor: SyntaxTextAreaDemo,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut editor))?;

        if let Event::Key(key) = event::read()? {
            match handle_key_press(key, &mut editor) {
                Ok(should_continue) => {
                    if !should_continue {
                        break;
                    }
                }
                Err(e) => {
                    editor.set_debug_message(format!("Error: {e}"));
                }
            }
        }
    }
    Ok(())
}

fn ui(f: &mut Frame, editor: &mut SyntaxTextAreaDemo) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(8)])
        .split(f.area());

    render_textarea(f, chunks[0], editor);
    render_status_and_help(f, chunks[1], editor);
}

fn render_textarea(f: &mut Frame, area: ratatui::layout::Rect, editor: &mut SyntaxTextAreaDemo) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("🎨 Syntax Highlighted Code Editor");

    let textarea_widget = TextAreaSyntax::default().block(block.clone());
    f.render_stateful_widget(textarea_widget, area, &mut editor.textarea);

    // Reuse cursor calculation from the wrapped textarea
    let (cx, cy) = editor.textarea.cursor(area, Some(&block));
    f.set_cursor_position((cx, cy));
}

fn render_status_and_help(f: &mut Frame, area: ratatui::layout::Rect, editor: &SyntaxTextAreaDemo) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(5)])
        .split(area);

    let status_text = if editor.has_unsaved_changes() {
        format!(
            "-- SYNTAX MODE (highlighting enabled) -- [Modified] {} | {}",
            editor.debug_message(),
            editor.get_cursor_info()
        )
    } else {
        format!(
            "-- SYNTAX MODE (highlighting enabled) -- {} | {}",
            editor.debug_message(),
            editor.get_cursor_info()
        )
    };

    let status = Paragraph::new(Line::from(Span::raw(status_text))).block(
        Block::default()
            .borders(Borders::ALL)
            .title("🎨 Syntax Status"),
    );

    f.render_widget(status, chunks[0]);

    let help_text = "🎨 SYNTAX HIGHLIGHTING DEMO\n\
F5=Rust, F6=Python, F7=JavaScript, F8=Scheme\n\
F1/F2=overflow modes, F3/F4=wrap indent\n\
F9=cycle themes, ?=info, Ctrl+Q=quit";

    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("🚀 Help"))
        .style(Style::default().fg(Color::Cyan));

    f.render_widget(help, chunks[1]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎨 Canvas Textarea Syntax Highlighting Demo");
    println!("✅ cursor-style feature: ENABLED");
    println!("✅ textarea feature: ENABLED");
    println!("✅ syntect feature: ENABLED");
    println!("🎨 Syntax highlighting active");
    println!();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let editor = SyntaxTextAreaDemo::new();

    let res = run_app(&mut terminal, editor);

    CursorManager::reset()?;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    println!("🎨 Syntax highlighting demo complete!");
    Ok(())
}
