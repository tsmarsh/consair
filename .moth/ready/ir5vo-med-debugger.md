# Add Debugger Support

## Problem

The interpreter has no debugging capabilities. When code fails or behaves unexpectedly, users have limited tools to understand what's happening.

## Impact

- Difficult to debug complex programs
- No visibility into execution
- No way to inspect state
- Poor error diagnostics

## Prompt for Implementation

```
Add debugging capabilities including stack traces, breakpoints, and step-through execution:

1. Currently no debugging support
2. Need tools to understand program execution

Please implement:

**Core Debugging Features:**

1. **Stack Traces:**
   ```rust
   struct StackFrame {
       function_name: Option<String>,
       expr: Value,
       location: SourceLocation,
   }

   struct CallStack {
       frames: Vec<StackFrame>,
   }

   impl Environment {
       fn push_frame(&mut self, frame: StackFrame) {
           self.call_stack.push(frame);
       }

       fn pop_frame(&mut self) {
           self.call_stack.pop();
       }

       fn get_stack_trace(&self) -> String {
           self.call_stack.frames.iter()
               .enumerate()
               .map(|(i, frame)| format!("  #{}: {}", i, frame.display()))
               .collect::<Vec<_>>()
               .join("\n")
       }
   }
   ```

2. **Source Location Tracking:**
   ```rust
   #[derive(Debug, Clone)]
   struct SourceLocation {
       file: Option<String>,
       line: usize,
       column: usize,
   }

   // Modify Value to track source location
   enum Value {
       // ... existing variants with added location field
   }
   ```

3. **Breakpoints:**
   ```rust
   struct Debugger {
       breakpoints: HashSet<SourceLocation>,
       step_mode: StepMode,
       watches: Vec<String>,  // Variable names to watch
   }

   enum StepMode {
       Continue,
       StepOver,
       StepInto,
       StepOut,
   }
   ```

4. **Interactive Debugging REPL:**
   ```lisp
   ; Set breakpoint
   (break! function-name)
   (break! file.lisp:42)

   ; When breakpoint hit:
   consair> (factorial 5)
   Breakpoint at factorial (factorial.lisp:3)
   debug> p n           ; Print variable
   5
   debug> s             ; Step
   debug> c             ; Continue
   debug> bt            ; Backtrace
   debug> q             ; Quit debug mode
   ```

**Implementation Details:**

1. **Modify eval() to support debugging:**
   ```rust
   fn eval_debug(
       expr: Value,
       env: &mut Environment,
       debugger: &mut Debugger
   ) -> Result<Value, String> {
       // Push stack frame
       env.push_frame(StackFrame::from_expr(&expr));

       // Check breakpoint
       if debugger.should_break(&expr) {
           debug_repl(env, debugger)?;
       }

       // Check step mode
       match debugger.step_mode {
           StepMode::StepInto => {
               debug_repl(env, debugger)?;
           }
           _ => {}
       }

       // Evaluate
       let result = eval_impl(expr, env)?;

       // Pop stack frame
       env.pop_frame();

       Ok(result)
   }
   ```

2. **Error reporting with stack traces:**
   ```rust
   fn format_error(error: &str, env: &Environment) -> String {
       format!(
           "Error: {}\n\nStack trace:\n{}",
           error,
           env.get_stack_trace()
       )
   }
   ```

3. **Debug REPL commands:**
   ```
   Commands:
   - p <expr>      Print expression
   - bt            Backtrace
   - s             Step into
   - n             Step over (next)
   - c             Continue
   - break <loc>   Set breakpoint
   - clear <loc>   Clear breakpoint
   - watch <var>   Watch variable
   - locals        Show local variables
   - up            Move up stack frame
   - down          Move down stack frame
   - q             Quit debugger
   ```

4. **Watch expressions:**
   ```rust
   struct WatchPoint {
       expr: String,
       old_value: Option<Value>,
   }

   fn check_watches(&mut self, env: &Environment) -> Vec<String> {
       let mut changes = Vec::new();
       for watch in &mut self.watches {
           let new_value = eval_str(&watch.expr, env);
           if new_value != watch.old_value {
               changes.push(format!(
                   "{}: {} -> {}",
                   watch.expr,
                   watch.old_value,
                   new_value
               ));
               watch.old_value = new_value;
           }
       }
       changes
   }
   ```

**REPL Integration:**

```lisp
; Enable debug mode
(debug-mode #t)

; Run with debugging
(debug (factorial 5))

; Step through evaluation
(step '(+ 1 2 3))
Step: (+ 1 2 3)
  Step into +? [y/n/c]
```

**Trace Mode:**

```lisp
(trace factorial)
; Now prints entry/exit:
=> factorial(5)
  => factorial(4)
    => factorial(3)
      => factorial(2)
        => factorial(1)
        <= 1
      <= 2
    <= 6
  <= 24
<= 120
```

**Testing:**

- Test stack trace accuracy
- Test breakpoint hit detection
- Test step modes (into, over, out)
- Test variable inspection
- Test watch expressions
- Test trace mode

**Documentation:**

- Debugging guide
- Breakpoint syntax
- Debug commands reference
- Common debugging workflows
- Examples of debugging sessions

**Example Session:**

```lisp
consair> (break! factorial)
Breakpoint set at factorial

consair> (factorial 3)
Breakpoint hit: factorial
  at factorial.lisp:2
  in (factorial 3)

debug> bt
  #0: factorial(3)
  #1: <repl>

debug> p n
3

debug> s
Step: (cond ((eq n 0) 1) ...)

debug> locals
n = 3

debug> c
Continuing...
6

consair>
```

**Advanced Features (Future):**

- Time-travel debugging (record/replay)
- Conditional breakpoints
- Performance profiling integration
- Memory inspection
- Visual debugger UI

## Success Criteria

- [ ] Stack traces on errors
- [ ] Source location tracking
- [ ] Breakpoints work
- [ ] Step-through debugging (into/over/out)
- [ ] Debug REPL with commands
- [ ] Variable inspection
- [ ] Watch expressions
- [ ] Trace mode
- [ ] Tests for debugger features
- [ ] Debugging guide documentation

