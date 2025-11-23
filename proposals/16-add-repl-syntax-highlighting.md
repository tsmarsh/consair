# Add REPL Syntax Highlighting

## Problem

The REPL displays all input in plain white text, making it difficult to visually distinguish between different code elements like keywords, strings, numbers, and symbols. This reduces code readability and makes it harder to spot syntax errors while typing.

**Current state:**
```
consair> (label factorial (lambda (n) (cond ((= n 0) 1) (t (* n (factorial (- n 1)))))))
```
All text appears in the same color with no visual distinction.

**Desired state:**
```
consair> (label factorial (lambda (n) (cond ((= n 0) 1) (t (* n (factorial (- n 1)))))))
         ^       ^         ^            ^
      keyword  symbol    keyword      keyword
```
With colors: keywords in blue, strings in green, numbers in yellow, etc.

## Impact

**Medium Priority** - Quality of life improvement that enhances developer experience:

- ✅ **Improved readability** - Easier to parse complex expressions visually
- ✅ **Faster error detection** - Spot mismatched strings, invalid syntax at a glance
- ✅ **Better learning experience** - New users can distinguish language constructs
- ✅ **Professional appearance** - Modern REPL experience like Python, Node.js
- ⚠️ **Not critical** - Doesn't affect functionality, purely cosmetic

This is a natural follow-up to Proposal 10 (REPL improvements) that makes the interactive experience more polished.

## Prompt for Implementation

```
Add syntax highlighting to the Consair REPL using rustyline's Highlighter trait:

Current state: Proposal 10 completed - rustyline REPL with history and multi-line support
Goal: Add color-coded syntax highlighting for better readability

Please:

1. **Add ANSI color support dependencies** to `cons/Cargo.toml`:
   ```toml
   [dependencies]
   colored = "2.1"  # For ANSI color codes
   # or use rustyline's built-in coloring
   ```

2. **Implement LispHighlighter** in `cons/src/main.rs`:

   ```rust
   use rustyline::highlight::Highlighter;
   use rustyline::hint::Hinter;
   use rustyline::validate::Validator;
   use rustyline::completion::Completer;
   use rustyline::Helper;
   use std::borrow::Cow;
   use colored::*;

   #[derive(Clone)]
   struct LispHelper;

   impl Highlighter for LispHelper {
       fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
           highlight_lisp(line)
       }

       fn highlight_char(&self, _line: &str, _pos: usize, _forced: bool) -> bool {
           true
       }
   }

   impl Hinter for LispHelper {
       type Hint = String;
   }

   impl Completer for LispHelper {
       type Candidate = String;
   }

   impl Validator for LispHelper {}

   impl Helper for LispHelper {}
   ```

3. **Implement the highlighting function**:

   ```rust
   fn highlight_lisp(line: &str) -> Cow<'static, str> {
       let mut result = String::new();
       let mut chars = line.chars().peekable();
       let mut in_string = false;
       let mut escape_next = false;
       let mut current_token = String::new();

       while let Some(ch) = chars.next() {
           // Handle strings
           if in_string {
               current_token.push(ch);
               if escape_next {
                   escape_next = false;
                   continue;
               }
               if ch == '\\' {
                   escape_next = true;
                   continue;
               }
               if ch == '"' {
                   result.push_str(&current_token.green().to_string());
                   current_token.clear();
                   in_string = false;
               }
               continue;
           }

           match ch {
               '"' => {
                   flush_token(&mut result, &mut current_token);
                   in_string = true;
                   current_token.push(ch);
               }
               '(' | ')' => {
                   flush_token(&mut result, &mut current_token);
                   result.push_str(&ch.to_string().bright_black().to_string());
               }
               ' ' | '\t' | '\n' => {
                   flush_token(&mut result, &mut current_token);
                   result.push(ch);
               }
               _ => {
                   current_token.push(ch);
               }
           }
       }

       // Flush remaining string if unclosed
       if in_string {
           result.push_str(&current_token.green().to_string());
       } else {
           flush_token(&mut result, &mut current_token);
       }

       Cow::Owned(result)
   }

   fn flush_token(result: &mut String, token: &mut String) {
       if token.is_empty() {
           return;
       }

       let colored = colorize_token(token);
       result.push_str(&colored);
       token.clear();
   }

   fn colorize_token(token: &str) -> String {
       // Special forms and keywords
       match token {
           // Core special forms
           "lambda" | "label" | "cond" | "quote" | "atom" | "eq" |
           "car" | "cdr" | "cons" => token.blue().bold().to_string(),

           // Operators
           "+" | "-" | "*" | "/" | "=" | "<" | ">" | "<=" | ">=" =>
               token.yellow().to_string(),

           // Constants
           "t" | "nil" => token.magenta().bold().to_string(),

           // Numbers (simple check)
           _ if token.parse::<i64>().is_ok() ||
                token.parse::<f64>().is_ok() =>
               token.cyan().to_string(),

           // Keywords (start with :)
           _ if token.starts_with(':') => token.magenta().to_string(),

           // Default: symbols
           _ => token.normal().to_string(),
       }
   }
   ```

4. **Update Editor initialization** in `repl()`:

   ```rust
   fn repl() {
       let mut env = Environment::new();
       register_stdlib(&mut env);

       let config = Config::builder()
           .auto_add_history(true)
           .history_ignore_space(true)
           .build();

       // Use the helper with highlighting
       let helper = LispHelper;
       let mut rl = Editor::with_config(config).unwrap();
       rl.set_helper(Some(helper));

       // ... rest of REPL code
   }
   ```

5. **Color scheme design**:

   Define a consistent, readable color scheme:

   | Element | Color | Reason |
   |---------|-------|--------|
   | Special forms (lambda, label, cond) | **Blue bold** | Keywords stand out |
   | Operators (+, -, *, /) | **Yellow** | Math operators visible |
   | Numbers | **Cyan** | Distinct from symbols |
   | Strings | **Green** | Standard convention |
   | Booleans (t, nil) | **Magenta bold** | Important constants |
   | Parentheses | **Bright black** | Subtle, don't distract |
   | Symbols | **White/Default** | Normal text |
   | REPL commands (:help, :quit) | **Magenta** | Special REPL syntax |
   | Errors | **Red** | Already using ⚠ symbol |

6. **Handle edge cases**:

   - Don't highlight inside strings (preserve literal content)
   - Handle escaped quotes properly
   - Work with multi-line input (highlight each line)
   - Don't break on incomplete expressions
   - Handle comments (if they start with `;`)

7. **Add tests** for the highlighter:

   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_highlight_keywords() {
           let input = "(lambda (x) x)";
           let output = highlight_lisp(input);
           assert!(output.contains("lambda"));
       }

       #[test]
       fn test_highlight_strings() {
           let input = r#"(quote "hello world")"#;
           let output = highlight_lisp(input);
           // Should highlight string in green
           assert!(output.contains("hello world"));
       }

       #[test]
       fn test_highlight_numbers() {
           let input = "(+ 123 456)";
           let output = highlight_lisp(input);
           // Numbers should be highlighted
           assert!(output.contains("123"));
       }

       #[test]
       fn test_no_highlight_in_strings() {
           let input = r#""(lambda (x) x)""#;
           let output = highlight_lisp(input);
           // Should NOT highlight keywords inside strings
           // Entire content should be green
       }
   }
   ```

8. **Optional: Add theme support**:

   Allow users to configure colors:

   ```rust
   pub struct ColorTheme {
       pub keyword: Color,
       pub operator: Color,
       pub number: Color,
       pub string: Color,
       pub constant: Color,
       pub paren: Color,
   }

   impl ColorTheme {
       pub fn default() -> Self { /* ... */ }
       pub fn dark() -> Self { /* ... */ }
       pub fn light() -> Self { /* ... */ }
   }
   ```

9. **Optional: Disable for non-TTY**:

   Detect if output is a terminal:

   ```rust
   use std::io::IsTerminal;

   let use_colors = std::io::stdout().is_terminal();
   ```

## Alternative Approaches

### Option 1: Use rustyline's built-in coloring (Recommended)
- Pros: No extra dependencies, well-integrated
- Cons: Limited to rustyline's API

### Option 2: Use external crate like `syntect`
- Pros: More powerful, theme support
- Cons: Heavy dependency, overkill for Lisp

### Option 3: Use ANSI codes directly
- Pros: No dependencies
- Cons: Manual escape code management, cross-platform issues

## Success Criteria

- [x] Syntax highlighting works in interactive REPL
- [x] Keywords (lambda, label, cond, etc.) highlighted in blue
- [x] Strings highlighted in green
- [x] Numbers highlighted in cyan
- [x] Operators highlighted in yellow
- [x] Parentheses subtly highlighted
- [x] Multi-line input preserves highlighting
- [x] Strings are NOT highlighted internally (literal content preserved)
- [x] Works with existing multi-line support from Proposal 10
- [x] No highlighting in non-TTY mode (piped output)
- [x] Tests for highlighter function
- [x] No performance degradation

## Testing Plan

### Visual Tests
Run the REPL and verify colors appear:
```lisp
consair> (lambda (x) (+ x 1))
consair> (label factorial (lambda (n) (cond ((= n 0) 1) (t (* n (factorial (- n 1)))))))
consair> "This is a string with (parens) inside"
consair> (+ 123 456)
consair> :help
```

### Automated Tests
- Test colorize_token() with various inputs
- Test highlight_lisp() preserves string content
- Test edge cases (unclosed strings, multi-line)

### Regression Tests
- Ensure existing REPL features still work
- Verify history, multi-line, special commands unaffected
- Check piped input doesn't show ANSI codes

## Notes

- This builds directly on Proposal 10 (REPL improvements)
- Should work seamlessly with existing multi-line support
- Colors should be subtle enough not to distract
- Must handle incomplete expressions gracefully
- Consider accessibility - some users may be colorblind
- Could add `NO_COLOR` environment variable support
