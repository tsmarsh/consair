# Add REPL Tab Completion

## Problem

The REPL requires users to type complete function names and symbols, with no autocomplete assistance. This makes interactive development slower and requires memorizing exact function names from the standard library.

**Current state:**
```
consair> (vec<TAB>
[Nothing happens]
consair> (vector-ref...
[Must type complete function name]
```

**Desired state:**
```
consair> (vec<TAB>
vector-length  vector-ref
consair> (vector-<TAB>
consair> (vector-length ...
[Auto-completed!]
```

## Impact

**Medium Priority** - Developer productivity enhancement:

- ✅ **Faster typing** - Less typing, fewer typos
- ✅ **Discoverability** - Users can explore available functions via Tab
- ✅ **Reduced cognitive load** - Don't need to memorize exact names
- ✅ **Better learning** - Beginners can discover stdlib functions
- ✅ **Standard REPL feature** - Expected in modern interpreters
- ⚠️ **Not critical** - Nice-to-have, not essential for functionality

This complements Proposal 10 (REPL improvements) and Proposal 16 (syntax highlighting) to create a fully-featured interactive experience.

## Prompt for Implementation

```
Add tab completion to the Consair REPL using rustyline's Completer trait:

Current state: Proposal 10 completed - rustyline REPL with multi-line support
Goal: Add intelligent tab completion for symbols, keywords, and special commands

Please:

1. **Extend LispHelper** (from Proposal 16 or create if needed) in `cons/src/main.rs`:

   ```rust
   use rustyline::completion::{Completer, Pair};
   use rustyline::Context;
   use rustyline::Result as RlResult;

   #[derive(Clone)]
   struct LispHelper {
       completions: Vec<String>,
   }

   impl LispHelper {
       fn new(env: &Environment) -> Self {
           let mut completions = Vec::new();

           // Add core special forms
           completions.extend(vec![
               "lambda", "label", "cond", "quote", "atom", "eq",
               "car", "cdr", "cons",
           ].into_iter().map(String::from));

           // Add operators
           completions.extend(vec![
               "+", "-", "*", "/", "=", "<", ">", "<=", ">=",
           ].into_iter().map(String::from));

           // Add constants
           completions.extend(vec![
               "t", "nil",
           ].into_iter().map(String::from));

           // Add vector operations
           completions.extend(vec![
               "vector-length", "vector-ref",
           ].into_iter().map(String::from));

           // Add REPL commands
           completions.extend(vec![
               ":help", ":quit", ":env", ":h", ":q",
           ].into_iter().map(String::from));

           // TODO: Add user-defined functions from environment
           // This would require introspection capabilities in Environment

           completions.sort();
           completions.dedup();

           LispHelper { completions }
       }
   }

   impl Completer for LispHelper {
       type Candidate = Pair;

       fn complete(
           &self,
           line: &str,
           pos: usize,
           _ctx: &Context<'_>,
       ) -> RlResult<(usize, Vec<Pair>)> {
           let start = find_completion_start(line, pos);
           let prefix = &line[start..pos];

           if prefix.is_empty() {
               return Ok((start, vec![]));
           }

           let matches: Vec<Pair> = self.completions
               .iter()
               .filter(|c| c.starts_with(prefix))
               .map(|c| Pair {
                   display: c.clone(),
                   replacement: c.clone(),
               })
               .collect();

           Ok((start, matches))
       }
   }

   /// Find where the current token starts for completion
   fn find_completion_start(line: &str, pos: usize) -> usize {
       let mut start = pos;
       let chars: Vec<char> = line.chars().collect();

       while start > 0 {
           let ch = chars[start - 1];
           // Break on delimiters
           if ch.is_whitespace() || ch == '(' || ch == ')' {
               break;
           }
           start -= 1;
       }

       start
   }
   ```

2. **Update Helper trait implementations**:

   ```rust
   impl Hinter for LispHelper {
       type Hint = String;

       fn hint(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Option<String> {
           // Optional: Show hints as you type
           // For now, return None (no inline hints)
           None
       }
   }

   impl Validator for LispHelper {}

   impl Helper for LispHelper {}
   ```

3. **Implement smart completion context**:

   Add context-aware completion that understands when to complete:

   ```rust
   impl LispHelper {
       fn complete_smart(
           &self,
           line: &str,
           pos: usize,
       ) -> RlResult<(usize, Vec<Pair>)> {
           let start = find_completion_start(line, pos);
           let prefix = &line[start..pos];

           if prefix.is_empty() {
               return Ok((start, vec![]));
           }

           // Context analysis
           let context = analyze_context(line, start);

           let candidates = match context {
               CompletionContext::Command => {
                   // After '(' or at start - complete functions
                   self.completions.iter()
                       .filter(|c| !c.starts_with(':'))
                       .filter(|c| c.starts_with(prefix))
                       .cloned()
                       .collect()
               }
               CompletionContext::ReplCommand => {
                   // At line start - complete REPL commands
                   self.completions.iter()
                       .filter(|c| c.starts_with(':'))
                       .filter(|c| c.starts_with(prefix))
                       .cloned()
                       .collect()
               }
               CompletionContext::Symbol => {
                   // In expression - complete any symbol
                   self.completions.iter()
                       .filter(|c| c.starts_with(prefix))
                       .cloned()
                       .collect()
               }
           };

           let pairs = candidates.into_iter()
               .map(|c| Pair {
                   display: c.clone(),
                   replacement: c,
               })
               .collect();

           Ok((start, pairs))
       }
   }

   enum CompletionContext {
       Command,      // After '('
       ReplCommand,  // At line start with ':'
       Symbol,       // General symbol
   }

   fn analyze_context(line: &str, pos: usize) -> CompletionContext {
       let before = &line[..pos];

       if before.trim_start().starts_with(':') {
           return CompletionContext::ReplCommand;
       }

       // Look backwards for the last non-whitespace
       let trimmed = before.trim_end();
       if trimmed.ends_with('(') {
           return CompletionContext::Command;
       }

       CompletionContext::Symbol
   }
   ```

4. **Add dynamic completion from environment**:

   Enable completion of user-defined functions:

   ```rust
   // This requires adding introspection to Environment
   // For now, document this limitation

   impl Environment {
       /// Get all defined symbols in this environment
       /// (This method needs to be added to Environment)
       pub fn list_symbols(&self) -> Vec<String> {
           // TODO: Implement by collecting keys from bindings HashMap
           vec![]
       }
   }

   impl LispHelper {
       fn refresh_completions(&mut self, env: &Environment) {
           // Add user-defined functions
           let user_symbols = env.list_symbols();
           self.completions.extend(user_symbols);
           self.completions.sort();
           self.completions.dedup();
       }
   }
   ```

5. **Update REPL to use completion**:

   ```rust
   fn repl() {
       let mut env = Environment::new();
       register_stdlib(&mut env);

       let config = Config::builder()
           .auto_add_history(true)
           .history_ignore_space(true)
           .completion_type(CompletionType::List)  // Show list on Tab
           .build();

       let helper = LispHelper::new(&env);
       let mut rl = Editor::with_config(config).unwrap();
       rl.set_helper(Some(helper));

       // ... rest of REPL

       // Optional: Refresh completions after each eval
       // (if user defined new functions)
       loop {
           // ... evaluate expression ...

           // Refresh completions with new definitions
           if let Some(helper) = rl.helper_mut() {
               helper.refresh_completions(&env);
           }
       }
   }
   ```

6. **Handle special cases**:

   - **Multi-word completion**: Complete `vector-ref`, `vector-length`
   - **Case sensitivity**: Match case-insensitively if desired
   - **Prefix matching**: Only complete at token boundaries
   - **Don't complete in strings**: Skip completion inside `"..."`
   - **Don't complete in comments**: Skip after `;`

   ```rust
   fn should_complete(line: &str, pos: usize) -> bool {
       let mut in_string = false;
       let mut in_comment = false;

       for (i, ch) in line.chars().enumerate() {
           if i >= pos {
               break;
           }

           match ch {
               '"' => in_string = !in_string,
               ';' => in_comment = true,
               '\n' => in_comment = false,
               _ => {}
           }
       }

       !in_string && !in_comment
   }
   ```

7. **Add completion configuration**:

   ```rust
   use rustyline::config::CompletionType;

   let config = Config::builder()
       .auto_add_history(true)
       .history_ignore_space(true)
       .completion_type(CompletionType::List)     // Show all matches
       // or CompletionType::Circular            // Cycle through matches
       .max_history_size(1000)?
       .build();
   ```

8. **Add tests**:

   ```rust
   #[cfg(test)]
   mod completion_tests {
       use super::*;

       #[test]
       fn test_find_completion_start() {
           assert_eq!(find_completion_start("(lambda", 7), 1);
           assert_eq!(find_completion_start("(+ vec", 6), 3);
           assert_eq!(find_completion_start("  :hel", 6), 2);
       }

       #[test]
       fn test_complete_lambda() {
           let helper = LispHelper::new(&Environment::new());
           let (start, matches) = helper.complete("(lam", 4, &Context::new()).unwrap();

           assert_eq!(start, 1);
           assert!(matches.iter().any(|p| p.replacement == "lambda"));
       }

       #[test]
       fn test_complete_vector_functions() {
           let helper = LispHelper::new(&Environment::new());
           let (_, matches) = helper.complete("(vector-", 8, &Context::new()).unwrap();

           assert!(matches.iter().any(|p| p.replacement == "vector-length"));
           assert!(matches.iter().any(|p| p.replacement == "vector-ref"));
       }

       #[test]
       fn test_complete_repl_commands() {
           let helper = LispHelper::new(&Environment::new());
           let (_, matches) = helper.complete(":h", 2, &Context::new()).unwrap();

           assert!(matches.iter().any(|p| p.replacement == ":help"));
           assert!(matches.iter().any(|p| p.replacement == ":h"));
       }

       #[test]
       fn test_no_complete_in_strings() {
           assert_eq!(should_complete(r#"(quote "lam"#, 12), false);
       }
   }
   ```

9. **Document completion behavior**:

   Update `:help` command to mention tab completion:

   ```rust
   fn print_help() {
       println!("Consair REPL - Interactive Lisp Interpreter");
       println!();
       println!("Special Commands:");
       println!("  :help, :h        Show this help message");
       println!("  :quit, :q        Exit the REPL");
       println!("  :env             Show current environment bindings");
       println!();
       println!("Keyboard Shortcuts:");
       println!("  Tab              Auto-complete symbols and commands");
       println!("  Ctrl-C           Clear current input");
       println!("  Ctrl-D           Exit REPL");
       println!("  Up/Down          Navigate command history");
       println!("  Ctrl-R           Reverse history search");
       println!();
       println!("Tab Completion:");
       println!("  - Complete function names: (vec<TAB> → vector-");
       println!("  - Complete REPL commands: :h<TAB> → :help");
       println!("  - Show all matches: press Tab twice");
       println!();
       // ... rest of help
   }
   ```

10. **Optional: Fuzzy matching**:

    For more advanced completion, add fuzzy matching:

    ```rust
    fn fuzzy_match(pattern: &str, candidate: &str) -> bool {
        let mut pattern_chars = pattern.chars();
        let mut current = pattern_chars.next();

        for ch in candidate.chars() {
            if Some(ch) == current {
                current = pattern_chars.next();
                if current.is_none() {
                    return true;
                }
            }
        }

        current.is_none()
    }

    // Use in completion:
    // .filter(|c| fuzzy_match(prefix, c))
    ```

## Alternative Approaches

### Option 1: Static completion list (Recommended for MVP)
- Pros: Simple, no environment introspection needed
- Cons: Won't complete user-defined functions

### Option 2: Dynamic completion with environment introspection
- Pros: Complete all defined symbols
- Cons: Requires Environment API changes

### Option 3: AI-powered completion
- Pros: Smart, context-aware suggestions
- Cons: Complex, requires ML model, overkill

## Success Criteria

- [x] Tab completion works in interactive REPL
- [x] Completes special forms (lambda, label, cond, etc.)
- [x] Completes operators (+, -, *, /, etc.)
- [x] Completes vector functions (vector-length, vector-ref)
- [x] Completes REPL commands (:help, :quit, :env)
- [x] Shows all matches when multiple completions available
- [x] Doesn't complete inside strings
- [x] Doesn't complete inside comments
- [x] Pressing Tab twice shows list of all matches
- [x] Works with multi-line input from Proposal 10
- [x] Tests for completion logic
- [x] Updated :help documentation

## Optional Success Criteria

- [ ] Completes user-defined functions (requires Environment introspection)
- [ ] Case-insensitive completion
- [ ] Fuzzy matching support
- [ ] Inline hints as you type
- [ ] Context-aware completion (different after '(' vs in middle)

## Testing Plan

### Interactive Testing
```bash
$ ./target/debug/cons
consair> (lam<TAB>
lambda
consair> (lambda (x) (vec<TAB>
vector-length  vector-ref
consair> :h<TAB>
:help  :h
```

### Automated Tests
- Test `find_completion_start()` with various inputs
- Test completion filtering with prefixes
- Test REPL command completion
- Test no completion in strings/comments

### Edge Cases
- Empty input + Tab (should show nothing or all)
- Completion after `(`
- Completion in middle of expression
- Multiple matches handling

## Dependencies

**No new dependencies required** - rustyline already includes completion support.

## Integration with Proposal 16 (Syntax Highlighting)

If Proposal 16 is implemented, the LispHelper will already exist:

```rust
#[derive(Clone)]
struct LispHelper {
    completions: Vec<String>,  // Add this field
}

// Implement both Highlighter (from 16) and Completer (this proposal)
```

## Notes

- Build on Proposal 10 (REPL improvements)
- Works well with Proposal 16 (syntax highlighting)
- Could be enhanced later with environment introspection
- Consider adding `.consairrc` file for custom completions
- May want to add common abbreviations (e.g., `lam` → `lambda`)
