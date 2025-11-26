Here you go — a copy-pasteable prompt you can feed into another instance (or model) to drive an implementation pass.

⸻


You are an expert in:

- Rust, ownership, and `Arc`-based memory models
- Lisp interpreters / VMs
- Clojure’s runtime abstractions (ISeq, ILookup, etc.)

You are working on a small Lisp implemented in Rust named **consair**.

## Context

- consair is a McCarthy-style Lisp implemented in Rust.
- Core `Value` enum includes (at least): `Nil`, `Atom`, `Cons`, `Lambda`, `Macro`, and probably `Vector`, `Map`, etc.
- Memory is managed via `Arc` (no GC); values are immutable and shared structurally.
- There is an interpreter and an optional LLVM JIT.
- The goal is to keep the *core* small and foundational, and layer higher-level dialects (e.g. a Clojure-flavoured stdlib and a GOOL-like DSL) on top via macros and libraries.

We now want to add **engine-level runtime abstractions inspired by Clojure** — the “polymorphic behaviors” that collections and values implement — while *not* hard-coding one particular dialect (like Clojure) into the language.

## Goal

Implement the following **engine-level abstractions** in Rust, in a way that:

- Fits naturally into consair’s existing `Value` implementation
- Respects the ARC / immutable data model
- Supports a future Clojure-like stdlib
- Keeps the core generic and not domain-specific

The target abstractions are:

1. **Seq abstraction** (like Clojure’s `ISeq` + `Seqable`)
   - Uniform iteration over values
   - Engine-level polymorphism

2. **Lookup abstraction** (like `ILookup`)
   - Uniform keyed/indexed retrieval (`get` semantics)

3. **Associative abstraction**
   - Uniform keyed updates (`assoc` semantics)

4. **Counted abstraction** (like `Counted`)
   - Fast, O(1) size queries when possible

5. **Indexed abstraction**
   - Fast random access (`nth` semantics) for indexable types

6. **Conj abstraction** (like `IPersistentCollection`’s `conj`)
   - Polymorphic insertion, behavior depending on collection type

7. **Reduced abstraction**
   - A “reduced” wrapper signaling early termination in folds/reductions

8. **Callable abstraction** (like `IFn`)
   - Making some values invocable as functions (`Value` as function)

(Metadata / IMeta/IObj can be treated as optional for now; if you have time, outline how they *would* be added.)

## Requirements

### 1. Design / API

Design **Rust traits or internal dispatch mechanisms** to represent these abstractions.

You should:

- Provide a small set of traits or methods that can be implemented by the relevant `Value` variants.
- Keep the public surface small and composable.
- Prefer simple trait-based designs over excessive indirection, but don’t be afraid to centralize dispatch on `Value` if it’s cleaner.

Examples (illustrative, not prescriptive):

```rust
pub trait Seqable {
    fn seq(&self) -> Option<Seq>;
}

pub trait ISeq {
    fn first(&self) -> Value;
    fn next(&self) -> Option<Seq>;
}

pub trait Lookup {
    fn val_at(&self, key: &Value, default: Option<&Value>) -> Value;
}

pub trait Associative {
    fn assoc(&self, key: Value, val: Value) -> Value;
}

pub trait Counted {
    fn count(&self) -> usize;
}

pub trait Indexed {
    fn nth(&self, index: usize, default: Option<&Value>) -> Value;
}

pub trait Conj {
    fn conj(&self, item: Value) -> Value;
}

These are just sketches. Feel free to refine/merge/simplify as long as the abstractions are coherent and composable.

2. Value variants

Assume the Value enum either currently includes or will include:
	•	Value::Nil
	•	Value::Cons(Arc<ConsCell>) – lists
	•	Value::Vector(Arc<VectorCell>)
	•	Value::Map(Arc<MapCell>)
	•	Value::Set(Arc<SetCell>) (if not present, sketch it)
	•	Value::String(Arc<String>) (or similar string representation)
	•	Value::Lambda(Arc<LambdaCell>)
	•	Value::Macro(Arc<MacroCell>)
	•	Other scalar atom types (numbers, keywords, symbols, booleans, etc.)

Implement the relevant abstractions for each applicable variant:
	•	Seq / Seqable:
	•	Lists → natural ISeq
	•	Vectors → Seq via index 0..len
	•	Maps → Seq of key/value entry pairs
	•	Sets → Seq of elements
	•	Strings → Seq of characters (if available)
	•	Lookup:
	•	Maps → keyed by any Value (with suitable hashing & equality)
	•	Vectors → index as key
	•	Sets → membership test
	•	Optionally: strings → index lookup
	•	Associative:
	•	Maps → associate new key/value
	•	Vectors → associate index → new vector with updated element
	•	Counted:
	•	Lists, vectors, maps, sets, strings (where possible)
	•	Ensure O(1) where you can (store cached size if needed), or be explicit when it’s O(n).
	•	Indexed:
	•	Vectors, strings, possibly lists (with O(n)) as a fallback
	•	Conj:
	•	Lists → cons at the front (like Clojure’s conj on lists)
	•	Vectors → append at the end
	•	Sets → add element
	•	Maps → expect a key/value pair (vector or cons) as input; add to map
	•	Callable (IFn-like):
	•	Functions / lambdas → normal call
	•	Maps, sets, vectors, keywords → optional:
	•	Map: (map key) = lookup
	•	Set: (set val) = membership test
	•	Vector: (vector idx) = nth
	•	Keyword: (keyword m) = lookup in map m
	•	Keep this minimal at first; you can just wire lambdas and maybe maps/keywords.
	•	Reduced:
	•	Add a Value::Reduced(Box<Value>) or similar wrapper.
	•	Provide helpers:
	•	is_reduced(&Value) -> bool
	•	unwrap_reduced(&Value) -> &Value
	•	You don’t need full transducer support yet; just implement the basic Reduced type.

3. Engine builtins

Expose a set of internal builtins that use these abstractions, which Lisp code can call:
	•	%seq, %first, %next
	•	%count
	•	%nth
	•	%get / %val-at
	•	%assoc
	•	%conj
	•	%reduced, %reduced?, %unwrap-reduced
	•	Optionally %call if your VM uses a uniform “call” entry point

These builtins will later be wrapped in a Clojure-flavoured stdlib as:
	•	seq, first, rest, next
	•	count
	•	nth
	•	get, assoc, conj
	•	reduced, reduced?
	•	And so on.

Please show:
	•	The Rust signatures for these builtins
	•	How they plug into the existing evaluator / opcode dispatch
	•	Examples of calling them from Lisp

4. Data structure implementations

For now, you can keep the implementations straightforward:
	•	Vector:
	•	Backed by Arc<Vec<Value>>
	•	Use copy-on-write semantics:
	•	If Arc is uniquely owned, mutate the underlying Vec (safe optimization)
	•	Otherwise, clone and modify
	•	This is good enough; we can swap to a bit-partitioned persistent vector later without changing the API.
	•	Map:
	•	Backed by Arc<HashMap<Value, Value>> with a custom Eq/Hash implementation for Value.
	•	Use copy-on-write like vector.
	•	Set:
	•	Backed by Arc<HashSet<Value>>, same pattern.

You should:
	•	Sketch or implement a ValueHash + ValueEq story (how to hash and compare values)
	•	Be explicit about which types are allowed as map/set keys; you can start with symbols, keywords, numbers, strings, and maybe vectors.

5. Testing / Examples

Add or describe tests that demonstrate:
	•	seq over list, vector, map, set, string
	•	count, nth, assoc, conj on multiple types
	•	get/lookup on map, vector, set
	•	Early termination via Reduced
	•	Optional: keyword-as-function and map-as-function usage

Concrete example snippets in Lisp:

; seq over vector
(seq [1 2 3])        ; => (1 2 3)

(first [1 2 3])      ; => 1
(rest  [1 2 3])      ; => (2 3)

(conj [1 2] 3)       ; => [1 2 3]
(conj '(2 3) 1)      ; => (1 2 3)

(assoc {:a 1} :b 2)  ; => {:a 1 :b 2}
(:a {:a 1 :b 2})     ; => 1 (via callable/ILookup)

(count {:a 1 :b 2})  ; => 2
(nth [10 20 30] 1)   ; => 20

They don’t all have to actually work at once yet, but design the abstractions so they can.

Style & constraints
	•	Follow Rust best practices (ownership, error handling, Arc safety).
	•	Avoid unnecessary unsafe; if you use it, justify it.
	•	Keep the abstractions cohesive and minimal; don’t over-engineer.
	•	Don’t break the existing public API or tests; if changes are required, call them out clearly.
	•	Favor clarity and extensibility; we’ll likely add more collection types later.

Deliverables
	1.	Proposed Rust traits / dispatch structure for the abstractions.
	2.	Updates to the Value enum and relevant data-structure types.
	3.	Implementations of the abstractions for major Value variants (list, vector, map, set, string).
	4.	Engine builtins wired into the evaluator / VM for:
	•	seq / first / next
	•	count, nth
	•	get / assoc / conj
	•	reduced
	5.	A few example Lisp snippets and/or unit tests demonstrating usage.

If something is ambiguous in consair’s current codebase, make a reasonable assumption, document it briefly, and proceed with a concrete design and implementation.


