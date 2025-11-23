# Document Security Implications

## Problem

The `shell()` function in stdlib has no input sanitization, allowing command injection. While this may be acceptable for an educational Lisp, it should be clearly documented with warnings.

**Location:** `consair-core/src/stdlib.rs`

## Impact

- Command injection vulnerability
- File system access without restrictions
- Users may not be aware of security implications

## Prompt for Implementation

```
Document security implications and add appropriate warnings for shell() and file I/O functions:

1. shell() function allows arbitrary command execution with no sanitization
2. File I/O functions (slurp, spit) have unrestricted file system access
3. Users should be aware of security implications

Please:
- Create docs/SECURITY.md documenting:

  **Security Considerations:**
  ```markdown
  # Security Considerations

  ## Overview

  Consair is an educational Lisp interpreter. It is NOT designed for running
  untrusted code. Several features have security implications:

  ## Command Execution

  The `shell` function executes arbitrary shell commands:

  ```lisp
  (shell "rm -rf /")  ; DANGEROUS!
  ```

  **Risks:**
  - Command injection if input comes from untrusted sources
  - No sandboxing or restrictions
  - Full access to system shell

  **Recommendations:**
  - Never pass untrusted input to shell()
  - Consider disabling shell() in production
  - Use explicit command builders instead of shell strings

  ## File System Access

  The `slurp` and `spit` functions provide unrestricted file access:

  ```lisp
  (slurp "/etc/passwd")  ; Can read any file
  (spit "/tmp/evil" "...")  ; Can write anywhere
  ```

  **Risks:**
  - Read sensitive files
  - Write to arbitrary locations
  - No permission checks beyond OS level

  **Recommendations:**
  - Validate file paths before use
  - Use absolute paths to prevent traversal
  - Consider restricting to specific directories

  ## Memory Safety

  Consair uses safe Rust with minimal unsafe code. However:

  **Circular References:**
  - Can create memory leaks (won't crash, but won't free)
  - Arc-based reference counting doesn't handle cycles

  **Stack Overflow:**
  - Deep recursion can crash the interpreter
  - [Will be fixed with recursion limit]

  ## Recommendations for Production Use

  If you want to use Consair to run untrusted code:

  1. **Disable dangerous functions:**
     - Remove shell(), slurp(), spit() from stdlib
     - Or replace with sandboxed versions

  2. **Add resource limits:**
     - Memory limits (track allocations)
     - CPU time limits (timeout evaluation)
     - Recursion depth limits [TODO]

  3. **Sandbox the process:**
     - Run in container
     - Use seccomp/AppArmor/SELinux
     - Drop privileges

  4. **Input validation:**
     - Limit program size
     - Timeout parsing
     - Reject deeply nested expressions
  ```

- Update README.md to reference SECURITY.md:
  ```markdown
  ## Security

  ⚠️ **Warning:** Consair is an educational interpreter and is NOT designed
  for running untrusted code. See [SECURITY.md](docs/SECURITY.md) for details.
  ```

- Add doc comments to dangerous functions:
  ```rust
  /// Execute a shell command
  ///
  /// # Security Warning
  ///
  /// This function executes arbitrary shell commands with no sanitization.
  /// Never pass untrusted input to this function as it enables command injection.
  ///
  /// # Example
  ///
  /// ```lisp
  /// (shell "echo hello")  ; Safe
  /// (shell user-input)    ; DANGEROUS - command injection!
  /// ```
  pub fn shell(args: &[Value], _env: &mut Environment) -> Result<Value, String>
  ```

- Add runtime warnings (optional):
  ```rust
  // In stdlib.rs registration
  if cfg!(debug_assertions) {
      eprintln!("Warning: shell() function is enabled. Do not run untrusted code.");
  }
  ```

- Consider adding a safe mode:
  ```rust
  pub struct StdlibConfig {
      pub enable_shell: bool,
      pub enable_file_io: bool,
      pub allowed_directories: Vec<PathBuf>,
  }

  pub fn register_stdlib_with_config(env: &mut Environment, config: StdlibConfig)
  ```

- Add to documentation:
  * Threat model
  * Trust boundaries
  * Safe usage patterns
  * Unsafe usage patterns to avoid

## Success Criteria

- [ ] SECURITY.md created and comprehensive
- [ ] README references security documentation
- [ ] Dangerous functions have doc comment warnings
- [ ] Clear recommendations for safe usage
- [ ] Examples of both safe and unsafe patterns
- [ ] (Optional) Safe mode configuration added
