# Split Parser Module into Lexer and Parser

## Problem

The parser module is quite large (898 lines) and combines both tokenization (lexing) and parsing into a single module. This makes the code harder to understand, test, and maintain.

**Location:** `consair-core/src/parser.rs`

## Impact

- Large module is harder to navigate
- Mixing concerns (tokenization vs parsing)
- Testing is less granular

## Prompt for Implementation

```
Split the parser.rs module into separate lexer.rs and parser.rs modules for better separation of concerns:

1. Current parser.rs is 898 lines combining tokenization and parsing
2. Should separate into:
   - lexer.rs: Token types, tokenize() function, character-level processing
   - parser.rs: AST construction from tokens, parse() function

Please:
- Create consair-core/src/lexer.rs with:
  * Token enum (move from parser.rs)
  * Tokenizer struct (if applicable, or keep as function)
  * tokenize() function
  * All string/sigil reading functions
  * Character-level utilities
  * Tests for tokenization only

- Update consair-core/src/parser.rs to:
  * Import Token from lexer module
  * Focus only on token → AST transformation
  * Parser struct and parse() function
  * Tests for parsing (assumes valid tokens)

- Update consair-core/src/lib.rs to export both modules appropriately

- Ensure clean interface between modules:
  * lexer.rs exports: Token, tokenize()
  * parser.rs imports Token from lexer
  * Public API remains unchanged (lib.rs still exports parse_str())

- Split tests appropriately:
  * Lexer tests: "5" → Token::Number("5")
  * Parser tests: [Token::Number("5")] → Value::Atom(Number(5))
  * Integration tests: "5" → Value::Atom(Number(5))

- Update documentation in each module

This refactoring should be purely organizational - no behavior changes.
```

## Success Criteria

- [ ] lexer.rs contains all tokenization logic
- [ ] parser.rs contains only AST construction
- [ ] All existing tests pass
- [ ] Tests are split appropriately
- [ ] Public API unchanged
- [ ] Module documentation is clear
