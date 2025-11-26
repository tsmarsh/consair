You are an expert in:

- Rust performance engineering (cache behavior, layout, branch prediction)
- Runtime / VM implementation
- Data structure design (arrays, hash tables, slabs, arenas)
- Clojure’s runtime abstractions (seq, lookup, assoc, conj, etc.)

You are working on a Lisp called **consair**, written in Rust.

## Context

The current design work has identified a set of **abstractions** inspired by Clojure:

1. **Seq abstraction** – uniform iteration (`seq`, `first`, `next`)
2. **Lookup abstraction** – keyed/indexed retrieval (`get`/`val-at`)
3. **Associative abstraction** – keyed updates (`assoc`)
4. **Counted abstraction** – size queries (`count`)
5. **Indexed abstraction** – fast random access (`nth`)
6. **Conj abstraction** – polymorphic insertion (`conj`)
7. **Reduced abstraction** – early termination in reductions (`Reduced`)
8. **Callable abstraction** – values callable as functions (`IFn`-like)

Previously, we discussed **persistent**, Clojure-like data structures that support these abstractions.

Now we want something different:

> **Design and implement a FAST, single-threaded, MUTABLE data structure layer that satisfies these abstractions, optimized for raw speed rather than structural sharing / multi-core scaling.**

We still need the same *semantic* abstractions (seq, lookup, etc.), but we are happy for the underlying storage to be mutable, compact, and tuned for single-thread performance.

You can assume:

- VM is single-threaded for this “fast” engine.
- Safety still matters (no UB), but we’re allowed to use internal mutability.
- We can have one or more “engines” behind the same `Value` interface (e.g., a “persistent” backend later).

## Goals

1. **Design data structures that satisfy the abstractions above, with a heavy bias toward:**
   - Fast single-threaded performance
   - Contiguous memory where possible
   - Minimal allocations
   - Predictable access patterns

2. **Use mutability under the hood.**
   - Collections may be mutated in place.
   - We don’t need copy-on-write or full persistence.
   - Lisp-level semantics can still be “mostly functional,” but we accept that under the hood things are being changed.

3. **Expose the same abstraction APIs** (`seq`, `assoc`, `get`, `conj`, `count`, `nth`, etc.), so higher-level code (std lib, macros) doesn’t care whether the backing store is persistent or mutable.

## Constraints / Assumptions

- The language is **single-threaded** in this mode → you can avoid `Arc` and use `Box`, `Rc`, or raw `&mut` access inside the evaluator.
- GC-free: we still prefer deterministic memory management and do *not* introduce a tracing GC.
- The Lisp `Value` enum is something like:

  ```rust
  enum Value {
      Nil,
      Bool(bool),
      Number(f64),             // or some numeric union
      Symbol(SymbolId),
      Keyword(KeywordId),
      String(StringId),        // possibly interned
      Cons(ConsId),
      Vector(VectorId),
      Map(MapId),
      Set(SetId),
      Lambda(LambdaId),
      Macro(MacroId),
      Reduced(Box<Value>),
      // ...
  }

But you may adjust it if a different representation is more efficient. Using integer IDs/handles into arenas/slabs is absolutely fine.
	•	You can assume a single-threaded arena / slab allocator is acceptable for cons cells, vectors, maps, etc.

Requirements

1. High-level design

Propose a “fast backend” for consair collections which:
	•	Satisfies the abstractions: seq, lookup, assoc, counted, indexed, conj, reduced, callable
	•	Uses mutable, imperative data structures under the hood:
	•	For vectors: contiguous Vec<Value>-like storage
	•	For maps/sets: hash tables with fast hashing (FxHash or similar)
	•	For lists: either linked cons cells in an arena, or use vectors internally where appropriate

Clearly explain:
	•	How each abstraction is implemented for lists, vectors, maps, sets, strings.
	•	Where you trade immutability for speed.
	•	Where you store lengths / metadata for O(1) count.

2. Concrete data structures

Design concrete Rust types for:

2.1 Vectors
	•	Backed by a contiguous buffer (Vec<Value> or custom).
	•	Mutable in place: push, set(index, value).
	•	Support:
	•	nth / Indexed
	•	count / Counted
	•	seq / iteration
	•	assoc (index → write)
	•	conj (append)

Consider:
	•	Optional small-vector optimization (inline small arrays) if you think it’s worth it.
	•	Keeping the layout JIT-friendly: contiguous, no extra indirection per element.

2.2 Maps
	•	Backed by a hash table (e.g., HashMap<Value, Value> or a more efficient table like hashbrown).
	•	Mutable in place: insert / remove directly.
	•	Support:
	•	get / ILookup
	•	assoc / Associative
	•	contains?
	•	conj (typically expects a key/value pair)
	•	seq returning key/value pairs
	•	count

You should:
	•	Define or sketch a Value hashing and equality strategy (e.g., a custom Hasher + Eq impl).
	•	Note which Value variants are allowed as keys initially (symbols, keywords, numbers, strings, maybe vectors).

2.3 Sets
	•	Implemented as a hash set on top of the map structure, or its own hash table.
	•	Mutable:
	•	conj adds element
	•	disj removes element
	•	Supports:
	•	contains?
	•	seq
	•	count
	•	Optional: callable semantics (set x) → membership test.

2.4 Lists / Cons cells
Because we care about speed, you can:
	•	Keep classic cons cells but allocate them in an arena/slab.
	•	Or treat most “lists” as vectors under the hood and only use cons cells where necessary.

Requirements:
	•	Support seq naturally.
	•	Support conj as “add at front”.
	•	Support count:
	•	You can choose O(n) count for lists if that’s acceptable.
	•	Or maintain lazy length caching in the cons cells if needed.

Describe how cons cells are represented and managed (arena vs direct Box).

2.5 Strings
	•	Represented by an owned String or interned storage.
	•	Support:
	•	seq → sequence of characters (or code points)
	•	count → length in chars or bytes (be explicit)
	•	Optional: nth by index

3. Abstraction implementation

For each abstraction, specify:

3.1 Seq abstraction
	•	A small enum or trait representing an active sequence:

enum Seq {
    List(ListSeqState),
    Vector(VectorSeqState),
    Map(MapSeqState),
    Set(SetSeqState),
    String(StringSeqState),
    // ...
}


	•	How seq(Value) constructs an initial Seq.
	•	How first(&Seq) and next(&mut Seq) work per collection type.
	•	Glue functions in the evaluator:

fn builtin_seq(args: &[Value]) -> Result<Value, Error>;
fn builtin_first(args: &[Value]) -> Result<Value, Error>;
fn builtin_next(args: &[Value]) -> Result<Value, Error>;



3.2 Lookup abstraction
	•	How get is implemented for:
	•	Map: hash lookup
	•	Vector: index
	•	Set: membership
	•	String: index
	•	Provide a single internal helper:

fn val_at(target: &Value, key: &Value, default: Option<&Value>) -> Value;



3.3 Associative abstraction
	•	For Map: assoc inserts/overwrites key → value.
	•	For Vector: assoc expands vector or errors if out of bounds (you choose, but document it).

Provide:

fn assoc(target: &mut Value, key: Value, val: Value) -> Result<Value, Error>;

Note: target may be mutated in place; it’s okay to return the same Value.

3.4 Counted abstraction
	•	Each type should give an O(1) or clearly defined O(n) count.
	•	Maps/sets/vectors/strings should store length explicitly.
	•	Lists: choose O(n) or caching strategy.

Provide:

fn count(v: &Value) -> usize;

3.5 Indexed abstraction
	•	Implement nth for vectors, strings, and optionally lists.
	•	Document behavior for out-of-bounds (error vs default value).

fn nth(coll: &Value, index: usize, default: Option<&Value>) -> Result<Value, Error>;

3.6 Conj abstraction
Define conj semantics:
	•	List: front
	•	Vector: append
	•	Map: key/value pair
	•	Set: add

fn conj(coll: &mut Value, item: Value) -> Result<Value, Error>;

Be explicit about type errors and expectations (e.g., map expects a pair).

3.7 Reduced abstraction
	•	Implement a Value::Reduced(Box<Value>).
	•	Add helpers:

fn reduced(v: Value) -> Value;          // wrap
fn is_reduced(v: &Value) -> bool;       // test
fn unreduced(v: &Value) -> &Value;      // unwrap



These will support fast, early termination in reductions.

3.8 Callable abstraction
	•	Implement function invocation for:
	•	Lambda values
	•	Optionally: maps, sets, vectors, keywords (for “Clojure feel”)

Specify the calling convention and how the evaluator dispatches:

fn apply(fn_val: &Value, args: &[Value]) -> Result<Value, Error>;

4. Implementation strategy

Give concrete guidance and code-level sketches for:
	•	How to organize the “fast mutable” data structure module (e.g., fast_vec.rs, fast_map.rs).
	•	How to integrate this with the existing evaluator:
	•	Which builtins need to be added/updated.
	•	How to keep the interface stable so that a future “persistent” backend could swap in.

Optionally propose:
	•	A config flag or feature (fast-engine) that picks this backend.
	•	Simple microbenchmarks to verify that:
	•	vector indexing / append
	•	map lookup / insert
are competitive with idiomatic Rust data structures.

5. Tests / Examples

Include or describe unit tests for each abstraction on each collection type, such as:
	•	seq over list/vector/map/set/string.
	•	count correctness and performance expectations.
	•	nth correctness and O(1) behavior on vectors.
	•	assoc and conj on maps/vectors/lists/sets.
	•	get semantics with and without defaults.
	•	Reduced in a simple reduce implementation.

Example Lisp snippets that should work efficiently:

; vector usage
(def v (vector 1 2 3))
(conj v 4)          ; => [1 2 3 4]
(nth v 2)           ; => 3
(count v)           ; => 4

; map usage
(def m (hash-map :a 1 :b 2))
(get m :a)          ; => 1
(assoc m :c 3)      ; => {:a 1 :b 2 :c 3}
(count m)           ; => 3

; seq over map
(seq m)             ; => ([:a 1] [:b 2] [:c 3]) or similar

; set membership
(def s (hash-set 1 2 3))
(conj s 4)
(contains? s 2)     ; => true

; Reduced / early termination
(reduce (fn [acc x]
          (if (> x 10)
              (reduced acc)
              (+ acc x)))
        0
        [1 2 3 11 12])
; => 6, and iteration stops at 11

Style
	•	Favor simple, direct, cache-friendly designs over “clever” persistent structures.
	•	Use Vec / HashMap / HashSet or equivalent efficient containers.
	•	Keep everything single-threaded; avoid Arc unless you have a compelling reason.
	•	Minimize allocations and indirections; think about how the JIT will iterate over these structures.
	•	You do not need to implement everything fully; a well-specified, partial implementation is acceptable as long as it’s coherent and extendable.

Your output should include:
	1.	Concrete Rust type definitions for the core collections.
	2.	The functions/methods that implement the abstractions for each type.
	3.	How these are accessed/exposed via Value and evaluator builtins.
	4.	Notes on performance characteristics and tradeoffs.


