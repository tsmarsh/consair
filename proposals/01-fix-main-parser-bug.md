# Fix Main.rs Expression Parser Bug

## Problem

The expression parser in `cons/src/main.rs` has a critical bug where the `in_string` variable is declared but never set to `true`. This causes string literals containing parentheses or brackets to break expression boundary detection.

**Location:** `cons/src/main.rs:91`

**Example that fails:**
```lisp
"hello (world)"
```

## Impact

- File execution can fail or produce incorrect results
- Multi-expression files may be parsed incorrectly
- Comments are not handled at all

## Prompt for Implementation

```
Fix the expression parser bug in cons/src/main.rs:

1. The parse_next_expr() function at line 82-156 has a bug where in_string is never set to true
2. String literals containing parentheses/brackets will break expression boundary detection
3. Comments are not handled

Please:
- Fix the string literal tracking (properly set in_string when entering/exiting strings)
- Add support for semicolon comments (ignore ; to end of line)
- Handle raw strings (r"..." and r#"..."#)
- Handle multiline strings properly
- Add comprehensive tests for the file runner including:
  * Multiple expressions in one file
  * String literals with parentheses
  * Comments between expressions
  * Raw strings
  * Edge cases (unclosed strings, nested quotes, etc.)

The function should properly track when we're inside a string literal to avoid counting parentheses/brackets that are part of the string content.
```

## Success Criteria

- [ ] `in_string` tracking works correctly for all string types
- [ ] Comments are properly ignored
- [ ] All existing tests pass
- [ ] New file runner tests added
- [ ] Files with strings containing parens parse correctly
