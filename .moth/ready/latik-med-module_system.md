# Add Module System

## Problem

The language has no module system - all code shares a single global namespace. This makes it difficult to organize large programs or create reusable libraries.

## Impact

- Name collisions in large programs
- Cannot hide implementation details
- No dependency management
- Hard to organize code into libraries

## Prompt for Implementation

```
Add a module system with import/export, namespaces, and library support:

1. Currently all code in single global namespace
2. Need module system for larger programs

Please implement:

**Basic Module Features:**

1. **Module definition:**
   ```lisp
   ; In file math.lisp
   (module math
     (export add multiply square)

     (define (add x y) (+ x y))
     (define (multiply x y) (* x y))
     (define (square x) (multiply x x))

     ; Private helper
     (define (helper x) ...)
   )
   ```

2. **Module import:**
   ```lisp
   ; In file main.lisp
   (import math)
   (math:square 5)  ; => 25

   ; Or selective import
   (import math (add multiply))
   (add 1 2)  ; => 3

   ; Or qualified import
   (import math :as m)
   (m:square 5)  ; => 25
   ```

3. **Module path resolution:**
   ```
   search paths:
   1. Current directory
   2. ./lib/
   3. ~/.consair/lib/
   4. $CONSAIR_PATH
   ```

**Implementation:**

1. **Add module metadata:**
   ```rust
   struct Module {
       name: String,
       exports: HashSet<String>,
       bindings: Environment,
       path: PathBuf,
   }

   struct ModuleRegistry {
       modules: HashMap<String, Module>,
       search_paths: Vec<PathBuf>,
   }
   ```

2. **Module evaluation:**
   ```rust
   fn eval_module(name: &str, registry: &mut ModuleRegistry) -> Result<Module, String> {
       // Find module file
       let path = find_module(name, &registry.search_paths)?;

       // Parse and evaluate in isolated environment
       let code = std::fs::read_to_string(path)?;
       let mut mod_env = Environment::new();
       eval_str(&code, &mut mod_env)?;

       // Extract exports
       let exports = mod_env.get_exports()?;

       Ok(Module { name, exports, bindings: mod_env, path })
   }
   ```

3. **Import implementation:**
   ```rust
   fn eval_import(module_name: &str, imports: Option<Vec<String>>, env: &mut Environment) {
       let module = registry.get_or_load(module_name)?;

       match imports {
           None => {
               // Import all exports with qualification
               for name in &module.exports {
                   let qual_name = format!("{}:{}", module_name, name);
                   env.define(qual_name, module.bindings.get(name)?);
               }
           }
           Some(names) => {
               // Import specific names
               for name in names {
                   if !module.exports.contains(&name) {
                       return Err(format!("{} not exported from {}", name, module_name));
                   }
                   env.define(name, module.bindings.get(&name)?);
               }
           }
       }
   }
   ```

4. **Syntax for modules:**
   ```lisp
   ; Module definition
   (module <name>
     (export <symbol>...)
     <body>...)

   ; Import variations
   (import <module>)                    ; Qualified access
   (import <module> (<sym>...))         ; Selective import
   (import <module> :as <alias>)        ; Aliased import
   (import <module> :all)               ; Import all (dangerous)
   ```

**Standard Library Organization:**

```
lib/
├── prelude.lisp       (auto-imported basics)
├── list.lisp          (list operations)
├── math.lisp          (numeric functions)
├── string.lisp        (string operations)
├── io.lisp            (I/O functions)
└── test.lisp          (testing framework)
```

**Advanced Features:**

1. **Circular dependency detection:**
   ```rust
   fn load_module_with_stack(name: &str, stack: &mut Vec<String>) -> Result<Module> {
       if stack.contains(name) {
           return Err(format!("Circular dependency: {:?} -> {}", stack, name));
       }
       stack.push(name.to_string());
       // ... load ...
       stack.pop();
   }
   ```

2. **Module reloading (REPL):**
   ```lisp
   (reload math)  ; Force reload of math module
   ```

3. **Private definitions:**
   ```lisp
   (module foo
     (export public-fn)

     (define public-fn ...)
     (define -private-fn ...)  ; Leading dash = private convention
   )
   ```

4. **Package metadata:**
   ```lisp
   ; package.lisp
   (package my-app
     (version "1.0.0")
     (author "...")
     (dependencies
       (math ">=1.0")
       (io "2.3")))
   ```

**Testing:**

- Test module loading from file
- Test export/import mechanics
- Test qualified vs unqualified access
- Test selective imports
- Test circular dependency detection
- Test module not found errors
- Test private vs exported bindings

**Documentation:**

- Module system guide
- How to organize code
- Best practices
- Standard library modules
- Creating packages

**Example Usage:**

```lisp
; lib/geometry.lisp
(module geometry
  (export circle-area rectangle-area)

  (define pi 3.14159)

  (define (circle-area radius)
    (* pi (* radius radius)))

  (define (rectangle-area width height)
    (* width height)))

; main.lisp
(import geometry)

(println (geometry:circle-area 5))
(println (geometry:rectangle-area 10 20))
```

## Success Criteria

- [ ] Modules can be defined and exported
- [ ] Import works (qualified, selective, aliased)
- [ ] Module search paths work
- [ ] Circular dependencies detected
- [ ] Private vs public bindings enforced
- [ ] Standard library reorganized into modules
- [ ] Tests for all module features
- [ ] Documentation complete
- [ ] Example modules provided

