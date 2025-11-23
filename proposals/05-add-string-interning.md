# Add String Interning for Symbols

## Problem

Symbols are currently stored as heap-allocated Strings, with the same symbol (e.g., "foo") allocated multiple times throughout the program. This wastes memory and makes symbol comparison slower than necessary.

## Impact

- 10-20% memory overhead for typical programs
- Symbol comparison requires string comparison instead of pointer equality
- Unnecessary allocations

## Prompt for Implementation

```
Add string interning for symbols to reduce memory usage and improve comparison performance:

1. Symbols are currently heap-allocated Strings that are duplicated
2. Same symbol "foo" may be allocated hundreds of times

Please:
- Add the string-interner crate dependency: string-interner = "0.17"
- Create an Interner struct that wraps StringInterner
- Modify SymbolType to store interned symbols (Sym instead of String):
  ```rust
  use string_interner::{StringInterner, Symbol};

  pub struct Interner {
      strings: StringInterner,
  }

  #[derive(Debug, Clone, Copy)]
  pub struct InternedSymbol(Symbol);
  ```
- Update parser to intern symbols during parsing
- Update interpreter to use interned symbols
- Consider thread-safety: might need Arc<Mutex<StringInterner>> or thread-local
- Add benchmarks to measure:
  * Memory usage improvement (before/after)
  * Symbol comparison speed (string vs pointer equality)
  * Parsing performance with many repeated symbols
- Ensure all tests pass
- Update Display implementation to resolve symbols when printing

Design considerations:
- Where to store the global interner? (Environment, separate global, or threaded?)
- How to handle serialization/deserialization if needed?
- Should keywords also be interned?

Recommend using a global thread-local interner for simplicity.
```

## Success Criteria

- [ ] Symbols are interned, not duplicated
- [ ] Memory usage reduced (measured via benchmark)
- [ ] Symbol comparison is faster
- [ ] All existing tests pass
- [ ] Printing symbols works correctly
