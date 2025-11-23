# Add Recursion Depth Limit

## Problem

The interpreter has no maximum recursion depth check, which causes stack overflow crashes instead of controlled errors. Deep recursion in user code can crash the interpreter.

**Location:** `consair-core/src/interpreter.rs:64`

## Impact

- Uncontrolled crashes on deep recursion
- Poor error messages (Rust panic instead of Lisp error)
- No way to catch or handle recursion errors

## Prompt for Implementation

```
Add a recursion depth limit to the interpreter to prevent stack overflow crashes:

1. The eval() function in consair-core/src/interpreter.rs has no recursion depth tracking
2. Deep recursion causes stack overflow crashes instead of controlled errors

Please:
- Add a constant MAX_RECURSION_DEPTH (suggest 1000 as default)
- Modify eval() to track recursion depth:
  * Add a depth parameter (or use thread-local storage)
  * Increment depth on each recursive call
  * Return clear error when limit exceeded: "Maximum recursion depth (1000) exceeded"
- Consider making the limit configurable via environment or function parameter
- Add tests for:
  * Deep recursion that hits the limit
  * Verify error message is clear and helpful
  * Verify normal recursion still works
  * Test mutual recursion (function A calls B calls A)
  * Test the exact boundary (depth = limit - 1, limit, limit + 1)
- Update documentation to mention the recursion limit

Prefer using a depth parameter passed through eval() rather than thread-local storage for better testability.
```

## Success Criteria

- [ ] Recursion depth is tracked
- [ ] Clear error when limit exceeded
- [ ] Normal recursion works (< 1000 depth)
- [ ] Deep recursion fails gracefully
- [ ] Tests cover boundary conditions
- [ ] Documentation updated
