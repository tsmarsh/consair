# Consair Improvement Proposals

This directory contains detailed implementation prompts for improving the Consair Lisp interpreter.

## Priority Levels

### 🔴 Critical (Do First)
1. ~~[Fix Main.rs Expression Parser Bug](01-fix-main-parser-bug.md)~~ - ✅ **COMPLETED**
2. ~~[Fix Numeric Overflow Bug](02-fix-numeric-overflow-bug.md)~~ - ✅ **COMPLETED**
3. ~~[Add Recursion Depth Limit](03-add-recursion-depth-limit.md)~~ - ✅ **COMPLETED** (Full TCO implemented)

### 🟡 High Priority
4. ~~[Optimize Environment Performance](04-optimize-environment-performance.md)~~ - ✅ **COMPLETED** (Arc::make_mut CoW optimization)
5. ~~[Add String Interning](05-add-string-interning.md)~~ - ✅ **COMPLETED** (Global RwLock-based interner)
6. ~~[Split Parser Module](06-split-parser-module.md)~~ - ✅ **COMPLETED** (Lexer/Parser separated into distinct modules)
7. ~~[Implement Tail Call Optimization](07-implement-tail-call-optimization.md)~~ - ✅ **COMPLETED** (via proposal #3)
8. ~~[Add Property-Based Testing](08-add-property-based-tests.md)~~ - ✅ **COMPLETED** (Property tests for numeric operations with 1000 cases each)
9. ~~[Add Comprehensive Benchmarks](09-add-benchmarks.md)~~ - ✅ **COMPLETED** (38 benchmarks covering parsing, eval, numeric, list, and string operations with HTML reports)
15. [Add Recursive Function Support](15-add-recursive-function-support.md) - Fix `label` to enable self-reference for recursive algorithms

### 🟢 Medium Priority
10. [Improve REPL Experience](10-improve-repl.md) - Modern interactive shell with history
11. [Document Security Implications](11-document-security-implications.md) - Warn about shell injection risks

### 🔵 Low Priority (Future Enhancements)
12. [Add Macro System](12-add-macro-system.md) - defmacro, quasiquote, meta-programming
13. [Add Module System](13-add-module-system.md) - Import/export, namespaces, libraries
14. [Add Debugger Support](14-add-debugger.md) - Stack traces, breakpoints, step-through

## How to Use These Proposals

Each proposal file contains:
- **Problem**: Description of the issue or limitation
- **Impact**: Why this matters
- **Prompt for Implementation**: Detailed instructions you can use with Claude Code or follow manually
- **Success Criteria**: Checklist of what "done" looks like

### Using with Claude Code

Copy the prompt section from any proposal and paste it into Claude Code:

```bash
# Example:
cat proposals/01-fix-main-parser-bug.md
# Copy the "Prompt for Implementation" section
# Paste into Claude Code to implement
```

### Manual Implementation

Use the proposals as detailed specifications:
1. Read the problem description
2. Review the implementation prompt
3. Follow the steps and examples
4. Check success criteria when done

## Estimated Timeline

- **Week 1-2**: Critical fixes (proposals 1-3)
- **Week 3-4**: Performance optimizations (proposals 4-5)
- **Month 2**: Testing and infrastructure (proposals 6-9)
- **Month 3**: Developer experience (proposals 10-11)
- **Future**: Advanced features (proposals 12-14)

## Dependencies Between Proposals

Some proposals should be done in order:

```
01, 02, 03 (critical fixes - can be done in parallel)
    ↓
04, 05 (performance - can be done in parallel)
    ↓
06 (refactoring - makes future work easier)
    ↓
08, 09 (testing infrastructure - can be done in parallel)
    ↓
07, 10 (features - can be done in parallel)
    ↓
11 (documentation)
    ↓
12, 13, 14 (major features - best done sequentially)
```

## Contributing

If you implement any of these proposals:
1. Create a feature branch
2. Follow the success criteria
3. Run all tests: `cargo test`
4. Update documentation
5. Submit a pull request

## Questions or Feedback

- Open an issue on GitHub
- Discuss in project discussions
- Propose new improvements

---

**Note**: These proposals were generated from a comprehensive codebase analysis. They represent concrete, actionable improvements prioritized by impact and feasibility.
