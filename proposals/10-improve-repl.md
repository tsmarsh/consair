# Improve REPL Experience

## Problem

Current REPL is basic with no line editing, history, or multi-line input support. This makes interactive development frustrating.

## Impact

- Poor developer experience
- No arrow key navigation
- No command history
- Can't edit multi-line expressions
- No syntax highlighting

## Prompt for Implementation

```
Improve the REPL experience using rustyline for a modern interactive shell:

1. Current REPL in cons/src/main.rs is basic (no readline, history, multi-line)
2. Need modern CLI experience

Please:
- Add rustyline dependency: rustyline = "14.0"
- Replace current input handling with rustyline Editor
- Implement features:

  **Basic Editing:**
  ```rust
  use rustyline::error::ReadlineError;
  use rustyline::Editor;
  use rustyline::Config;

  let config = Config::builder()
      .auto_add_history(true)
      .build();
  let mut rl = Editor::with_config(config)?;

  loop {
      match rl.readline("consair> ") {
          Ok(line) => { /* eval */ },
          Err(ReadlineError::Interrupted) => break,
          Err(ReadlineError::Eof) => break,
      }
  }
  ```

  **History Persistence:**
  ```rust
  let history_file = dirs::home_dir()
      .map(|h| h.join(".consair_history"))
      .unwrap_or_else(|| PathBuf::from(".consair_history"));

  if rl.load_history(&history_file).is_err() {
      println!("No previous history.");
  }

  // On exit:
  rl.save_history(&history_file)?;
  ```

  **Multi-line Input:**
  - Detect incomplete expressions (unclosed parens/brackets)
  - Continue prompt on next line: "......> "
  - Track paren/bracket depth
  - Handle strings properly (don't count parens in strings)

  **Syntax Highlighting (optional):**
  ```rust
  use rustyline::highlight::Highlighter;

  struct LispHighlighter;

  impl Highlighter for LispHighlighter {
      fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
          // Color keywords, strings, numbers, etc.
      }
  }
  ```

  **Tab Completion (optional):**
  ```rust
  use rustyline::completion::Completer;

  struct LispCompleter {
      env: Environment,
  }

  impl Completer for LispCompleter {
      fn complete(&self, line: &str, pos: usize, ...) {
          // Complete symbol names from environment
      }
  }
  ```

  **Keybindings:**
  - Ctrl-C: Interrupt current input
  - Ctrl-D: Exit REPL
  - Up/Down: History navigation
  - Ctrl-R: Reverse history search
  - Home/End: Line navigation

  **Help Command:**
  - Add special commands: `:help`, `:quit`, `:reset`, `:env`
  - `:help` shows available commands
  - `:env` shows current bindings

- Add configuration file support (optional):
  ```rust
  // ~/.consairrc
  (define pi 3.14159)
  (define e 2.71828)
  // Auto-loaded on REPL start
  ```

- Improve error display:
  - Show where in input the error occurred
  - Syntax highlight error messages
  - Suggest corrections for common mistakes

- Add REPL-specific features:
  - `:doc symbol` to show documentation
  - `:type expr` to show type without evaluating
  - Previous result available as `*1`, `*2`, `*3`

## Success Criteria

- [ ] Line editing works (arrows, home/end)
- [ ] History navigation (up/down)
- [ ] History persists between sessions
- [ ] Multi-line input for incomplete expressions
- [ ] Ctrl-C/Ctrl-D work correctly
- [ ] Help command available
- [ ] Improved error messages
- [ ] (Optional) Syntax highlighting
- [ ] (Optional) Tab completion
