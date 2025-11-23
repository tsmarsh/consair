# Consair Benchmarks

This document describes how to run and interpret the performance benchmarks for Consair.

## Running Benchmarks

Run all benchmarks:

```bash
cargo bench
```

Run specific benchmark groups:

```bash
# Parsing benchmarks only
cargo bench --bench benchmarks -- parsing

# Evaluation benchmarks only
cargo bench --bench benchmarks -- eval

# Numeric operation benchmarks only
cargo bench --bench benchmarks -- numeric

# List operation benchmarks only
cargo bench --bench benchmarks -- list

# String and symbol benchmarks only
cargo bench --bench benchmarks -- string
```

Run a specific benchmark:

```bash
cargo bench --bench benchmarks -- "parse small expr"
```

## Establishing a Baseline

Save the current results as a baseline for future comparison:

```bash
cargo bench --bench benchmarks -- --save-baseline main
```

## Comparing Performance

After making changes, compare against the baseline:

```bash
cargo bench --bench benchmarks -- --baseline main
```

This will show percentage changes for each benchmark.

## Benchmark Categories

### Parsing Benchmarks

- **parse small expr**: Simple expression parsing `(cons 1 2)`
- **parse medium expr**: Medium complexity with nested operations
- **parse large list**: 1000-element list parsing
- **parse deep nesting**: 100 levels of nesting
- **parse quoted list**: Quote syntax performance

### Evaluation Benchmarks

- **eval simple arithmetic**: Basic arithmetic operations
- **eval nested arithmetic**: Multiple nested operations
- **eval lambda creation**: Lambda definition overhead
- **eval lambda invocation**: Lambda application overhead
- **env lookup shallow**: Variable lookup in shallow scope
- **env lookup nested**: Variable lookup through 10 nested scopes
- **env define**: Environment variable definition
- **eval comprehensive program**: Complex program with closures
- **eval cond expression**: Conditional evaluation
- **eval nested lambda**: Nested lambda scopes

### Numeric Operation Benchmarks

- **numeric int add**: Integer addition
- **numeric bigint add**: BigInt addition (after overflow)
- **numeric ratio add**: Rational number addition
- **numeric int mul**: Integer multiplication
- **numeric division creating ratio**: Division producing ratios
- **numeric overflow promotion**: Int → BigInt promotion
- **numeric comparison**: Numeric comparisons
- **numeric cross-type comparison**: Int vs Ratio comparison

### List Operation Benchmarks

- **list cons**: Cons cell creation
- **list car/cdr**: List access operations
- **list build large**: Building 100-element list
- **list traverse**: Traversing 100-element list

### String and Symbol Benchmarks

- **string parse basic**: Basic string parsing
- **string parse unicode**: Unicode string handling
- **string parse escaped**: Escape sequence processing
- **symbol intern**: New symbol interning
- **symbol intern repeated**: Repeated symbol lookup

## Interpreting Results

Criterion outputs:

```
parse small expr        time:   [1.2345 µs 1.2456 µs 1.2567 µs]
                        change: [-2.3456% -1.2345% +0.1234%] (p = 0.12 > 0.05)
                        No change in performance detected.
```

- **time**: [lower bound, estimate, upper bound] with 95% confidence
- **change**: Performance change vs baseline (if comparing)
- **p-value**: Statistical significance (p < 0.05 = significant change)

## HTML Reports

Criterion generates detailed HTML reports in:

```
target/criterion/
```

Open `target/criterion/report/index.html` in a browser to view:

- Performance charts over time
- Regression analysis
- Probability density functions
- Outlier detection

## Performance Targets

As of the latest run, expected performance ranges:

### Parsing
- Small expressions: < 5 µs
- Medium expressions: < 20 µs
- Large lists (1000 elements): < 500 µs
- Deep nesting (100 levels): < 200 µs

### Evaluation
- Simple arithmetic: < 10 µs
- Lambda creation: < 5 µs
- Lambda invocation: < 10 µs
- Environment lookup: < 2 µs

### Numeric Operations
- Int arithmetic: < 100 ns
- BigInt arithmetic: < 500 ns
- Ratio arithmetic: < 500 ns
- Overflow promotion: < 200 ns

### List Operations
- Cons: < 100 ns
- Car/Cdr: < 5 µs (including eval overhead)
- Build 100 elements: < 50 µs
- Traverse 100 elements: < 20 µs

## Detecting Regressions

A regression is typically considered significant if:

1. Performance degrades by >10%
2. The change has statistical significance (p < 0.05)
3. The change is consistent across multiple runs

To verify a suspected regression:

```bash
# Run baseline
git checkout main
cargo bench --bench benchmarks -- --save-baseline main

# Test your changes
git checkout your-branch
cargo bench --bench benchmarks -- --baseline main
```

## Profiling

For detailed profiling, you can use tools like:

### Flamegraph

```bash
cargo install flamegraph
cargo flamegraph --bench benchmarks
```

### Perf (Linux)

```bash
cargo bench --bench benchmarks -- --profile-time=10
perf record -g target/release/deps/benchmarks-*
perf report
```

## Notes

- Each benchmark runs with 100 samples and 10-second measurement time
- Results include statistical analysis for confidence
- Benchmarks automatically warm up before measurement
- Criterion filters outliers and provides robust statistics
- Note: Recursive function benchmarks are currently excluded due to environment scoping complexity

## Continuous Integration

To track performance over time in CI:

1. Save baseline on main branch merge
2. Compare feature branches against main baseline
3. Fail CI if performance degrades >15%
4. Store baseline artifacts for historical comparison

Example CI workflow:

```yaml
- name: Benchmark
  run: |
    cargo bench --bench benchmarks -- --save-baseline ci
    cargo bench --bench benchmarks -- --baseline ci
```
