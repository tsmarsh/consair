# Add Comprehensive Benchmarks

## Problem

No performance benchmarks exist to measure or track performance of core operations. This makes it impossible to detect regressions or measure optimization improvements.

## Impact

- No baseline for performance
- Can't measure optimization impact
- Regressions go undetected
- Can't compare implementations

## Prompt for Implementation

```
Add comprehensive benchmarks using criterion.rs to measure and track performance:

1. No benchmarks currently exist
2. Need to establish baseline and track regressions

Please:
- Add criterion dependency to Cargo.toml:
  ```toml
  [dev-dependencies]
  criterion = { version = "0.5", features = ["html_reports"] }

  [[bench]]
  name = "benchmarks"
  harness = false
  ```

- Create benches/benchmarks.rs with benchmarks for:

  **Parsing Benchmarks:**
  ```rust
  fn bench_parse_small(c: &mut Criterion) {
      c.bench_function("parse small expr", |b| {
          b.iter(|| parse_str("(cons 1 2)"))
      });
  }

  fn bench_parse_large(c: &mut Criterion) {
      // Parse 1000+ element list
  }

  fn bench_parse_nested(c: &mut Criterion) {
      // Deep nesting (100+ levels)
  }
  ```

  **Evaluation Benchmarks:**
  ```rust
  fn bench_eval_arithmetic(c: &mut Criterion) {
      // (+ 1 2 3 4 5 ...) with varying counts
  }

  fn bench_eval_recursion(c: &mut Criterion) {
      // Recursive factorial with varying depths
  }

  fn bench_eval_lambda(c: &mut Criterion) {
      // Lambda creation and invocation
  }

  fn bench_env_lookup(c: &mut Criterion) {
      // Variable lookup at varying scope depths
  }
  ```

  **Numeric Benchmarks:**
  ```rust
  fn bench_numeric_add(c: &mut Criterion) {
      // Int, BigInt, Ratio, BigRatio additions
  }

  fn bench_numeric_overflow(c: &mut Criterion) {
      // Operations that trigger promotion
  }

  fn bench_numeric_division(c: &mut Criterion) {
      // Division creating ratios
  }
  ```

  **List Operation Benchmarks:**
  ```rust
  fn bench_list_cons(c: &mut Criterion) {
      // cons operations, structure sharing
  }

  fn bench_list_car_cdr(c: &mut Criterion) {
      // Traversing long lists
  }
  ```

- Configure criterion for statistical rigor:
  ```rust
  criterion_group! {
      name = benches;
      config = Criterion::default()
          .sample_size(100)
          .measurement_time(Duration::from_secs(10));
      targets = bench_parse_small, ...
  }
  ```

- Add benchmark documentation:
  * How to run: `cargo bench`
  * How to compare: `cargo bench --bench benchmarks -- --save-baseline before`
  * How to interpret results
  * Expected performance ranges

- Consider adding flamegraph profiling:
  ```bash
  cargo install flamegraph
  cargo bench --bench benchmarks --profile-time 10
  ```

- Add performance regression tracking in CI (optional):
  * Save baselines
  * Compare on PR
  * Alert on significant regressions (>10%)

## Success Criteria

- [ ] Benchmarks for all core operations
- [ ] Statistical significance (sample size ≥ 100)
- [ ] HTML reports generated
- [ ] Documentation on running benchmarks
- [ ] Baselines established
- [ ] Can detect 10%+ regressions
