use codspeed_criterion_compat::{Criterion, black_box, criterion_group, criterion_main};
use consair::interner::InternedSymbol;
use consair::language::AtomType;
use consair::{Environment, NumericType, Value, cons, eval, parse, register_stdlib};
use std::time::Duration;

// ============================================================================
// Parsing Benchmarks
// ============================================================================

fn bench_parse_small(c: &mut Criterion) {
    c.bench_function("parse small expr", |b| {
        b.iter(|| black_box(parse("(cons 1 2)").unwrap()))
    });
}

fn bench_parse_medium(c: &mut Criterion) {
    let expr = "(+ 1 2 3 4 5 6 7 8 9 10 (* 11 12) (- 13 14) (/ 15 16))";
    c.bench_function("parse medium expr", |b| {
        b.iter(|| black_box(parse(expr).unwrap()))
    });
}

fn bench_parse_large_list(c: &mut Criterion) {
    // Generate a list with 1000 elements
    let mut elements = vec!["(list".to_string()];
    for i in 0..1000 {
        elements.push(i.to_string());
    }
    elements.push(")".to_string());
    let expr = elements.join(" ");

    c.bench_function("parse large list (1000 elements)", |b| {
        b.iter(|| black_box(parse(&expr).unwrap()))
    });
}

fn bench_parse_deep_nesting(c: &mut Criterion) {
    // Generate deeply nested expression: (+ (+ (+ ... (+ 1 2) ...)))
    let mut expr = String::from("1");
    for _ in 0..100 {
        expr = format!("(+ {expr} 1)");
    }

    c.bench_function("parse deep nesting (100 levels)", |b| {
        b.iter(|| black_box(parse(&expr).unwrap()))
    });
}

fn bench_parse_quoted_list(c: &mut Criterion) {
    let expr = "'(1 2 3 4 5 6 7 8 9 10)";
    c.bench_function("parse quoted list", |b| {
        b.iter(|| black_box(parse(expr).unwrap()))
    });
}

// ============================================================================
// Evaluation Benchmarks
// ============================================================================

fn bench_eval_simple_arithmetic(c: &mut Criterion) {
    let mut env = Environment::new();
    register_stdlib(&mut env);

    c.bench_function("eval simple arithmetic", |b| {
        b.iter(|| {
            let expr = parse("(+ 1 2 3 4 5)").unwrap();
            black_box(eval(expr, &mut env.clone()).unwrap())
        })
    });
}

fn bench_eval_nested_arithmetic(c: &mut Criterion) {
    let mut env = Environment::new();
    register_stdlib(&mut env);

    c.bench_function("eval nested arithmetic", |b| {
        b.iter(|| {
            let expr = parse("(+ (* 2 3) (- 10 5) (/ 20 4))").unwrap();
            black_box(eval(expr, &mut env.clone()).unwrap())
        })
    });
}

fn bench_eval_lambda_creation(c: &mut Criterion) {
    let mut env = Environment::new();
    register_stdlib(&mut env);

    c.bench_function("eval lambda creation", |b| {
        b.iter(|| {
            let expr = parse("(lambda (x) (+ x 1))").unwrap();
            black_box(eval(expr, &mut env.clone()).unwrap())
        })
    });
}

fn bench_eval_lambda_invocation(c: &mut Criterion) {
    let mut env = Environment::new();
    register_stdlib(&mut env);

    c.bench_function("eval lambda invocation", |b| {
        b.iter(|| {
            let expr = parse("((lambda (x) (+ x 1)) 42)").unwrap();
            black_box(eval(expr, &mut env.clone()).unwrap())
        })
    });
}

// ============================================================================
// Recursive Function Benchmarks
// ============================================================================

fn bench_recursive_factorial(c: &mut Criterion) {
    let mut env = Environment::new();
    register_stdlib(&mut env);

    // Define factorial function
    let setup = parse(
        r#"
        (label factorial (lambda (n)
            (cond
                ((= n 0) 1)
                (t (* n (factorial (- n 1)))))))
    "#,
    )
    .unwrap();
    eval(setup, &mut env).unwrap();

    c.bench_function("recursive factorial(10)", |b| {
        b.iter(|| {
            let expr = parse("(factorial 10)").unwrap();
            black_box(eval(expr, &mut env.clone()).unwrap())
        })
    });
}

fn bench_recursive_fibonacci(c: &mut Criterion) {
    let mut env = Environment::new();
    register_stdlib(&mut env);

    // Define fibonacci function
    let setup = parse(
        r#"
        (label fib (lambda (n)
            (cond
                ((= n 0) 0)
                ((= n 1) 1)
                (t (+ (fib (- n 1)) (fib (- n 2)))))))
    "#,
    )
    .unwrap();
    eval(setup, &mut env).unwrap();

    c.bench_function("recursive fibonacci(10)", |b| {
        b.iter(|| {
            let expr = parse("(fib 10)").unwrap();
            black_box(eval(expr, &mut env.clone()).unwrap())
        })
    });
}

fn bench_recursive_list_length(c: &mut Criterion) {
    let mut env = Environment::new();
    register_stdlib(&mut env);

    // Define length function
    let setup = parse(
        r#"
        (label length (lambda (lst)
            (cond
                ((atom lst) 0)
                (t (+ 1 (length (cdr lst)))))))
    "#,
    )
    .unwrap();
    eval(setup, &mut env).unwrap();

    c.bench_function("recursive list-length(100)", |b| {
        b.iter(|| {
            // Create a list of 100 elements
            let mut list = Value::Nil;
            for i in (0..100).rev() {
                list = cons(Value::Atom(AtomType::Number(NumericType::Int(i))), list);
            }
            env.define("test-list".to_string(), list);

            let expr = parse("(length test-list)").unwrap();
            black_box(eval(expr, &mut env.clone()).unwrap())
        })
    });
}

fn bench_recursive_sum_list(c: &mut Criterion) {
    let mut env = Environment::new();
    register_stdlib(&mut env);

    // Define sum function
    let setup = parse(
        r#"
        (label sum (lambda (lst)
            (cond
                ((atom lst) 0)
                (t (+ (car lst) (sum (cdr lst)))))))
    "#,
    )
    .unwrap();
    eval(setup, &mut env).unwrap();

    // Pre-create the list
    let mut list = Value::Nil;
    for i in (0..50).rev() {
        list = cons(Value::Atom(AtomType::Number(NumericType::Int(i))), list);
    }
    env.define("test-list".to_string(), list);

    c.bench_function("recursive sum-list(50)", |b| {
        b.iter(|| {
            let expr = parse("(sum test-list)").unwrap();
            black_box(eval(expr, &mut env.clone()).unwrap())
        })
    });
}

fn bench_tco_deep_recursion(c: &mut Criterion) {
    let mut env = Environment::new();
    register_stdlib(&mut env);

    // Define countdown function (tail recursive)
    let setup = parse(
        r#"
        (label countdown (lambda (n)
            (cond
                ((= n 0) 0)
                (t (countdown (- n 1))))))
    "#,
    )
    .unwrap();
    eval(setup, &mut env).unwrap();

    c.bench_function("TCO deep recursion(1000)", |b| {
        b.iter(|| {
            let expr = parse("(countdown 1000)").unwrap();
            black_box(eval(expr, &mut env.clone()).unwrap())
        })
    });
}

fn bench_mutual_recursion(c: &mut Criterion) {
    let mut env = Environment::new();
    register_stdlib(&mut env);

    // Define mutually recursive is-even/is-odd (must be done separately)
    let setup1 = parse(
        r#"
        (label is-even (lambda (n)
            (cond
                ((= n 0) t)
                (t (is-odd (- n 1))))))
    "#,
    )
    .unwrap();
    eval(setup1, &mut env).unwrap();

    let setup2 = parse(
        r#"
        (label is-odd (lambda (n)
            (cond
                ((= n 0) nil)
                (t (is-even (- n 1))))))
    "#,
    )
    .unwrap();
    eval(setup2, &mut env).unwrap();

    c.bench_function("mutual recursion is-even(100)", |b| {
        b.iter(|| {
            let expr = parse("(is-even 100)").unwrap();
            black_box(eval(expr, &mut env.clone()).unwrap())
        })
    });
}

// ============================================================================
// Environment Benchmarks
// ============================================================================

fn bench_env_lookup_shallow(c: &mut Criterion) {
    let mut env = Environment::new();
    register_stdlib(&mut env);
    env.define(
        "x".to_string(),
        Value::Atom(AtomType::Number(NumericType::Int(42))),
    );

    c.bench_function("env lookup shallow", |b| {
        b.iter(|| {
            let expr = parse("x").unwrap();
            black_box(eval(expr, &mut env.clone()).unwrap())
        })
    });
}

fn bench_env_lookup_nested(c: &mut Criterion) {
    // Create nested scopes using lambdas
    let code = r#"
((lambda (v0)
  ((lambda (v1)
    ((lambda (v2)
      ((lambda (v3)
        ((lambda (v4)
          ((lambda (v5)
            ((lambda (v6)
              ((lambda (v7)
                ((lambda (v8)
                  ((lambda (v9)
                    v0)
                   9))
                 8))
               7))
             6))
           5))
         4))
       3))
     2))
   1))
 0)
"#;

    let mut env = Environment::new();
    register_stdlib(&mut env);

    c.bench_function("env lookup nested (10 scopes)", |b| {
        b.iter(|| {
            let expr = parse(code).unwrap();
            black_box(eval(expr, &mut env.clone()).unwrap())
        })
    });
}

fn bench_env_define(c: &mut Criterion) {
    c.bench_function("env define", |b| {
        let mut counter = 0;
        b.iter(|| {
            let env = Environment::new();
            env.define(
                format!("var{counter}"),
                Value::Atom(AtomType::Number(NumericType::Int(counter))),
            );
            counter += 1;
            black_box(env)
        })
    });
}

// ============================================================================
// Numeric Operation Benchmarks
// ============================================================================

fn bench_numeric_int_add(c: &mut Criterion) {
    let a = NumericType::Int(12345);
    let b = NumericType::Int(67890);

    c.bench_function("numeric int add", |bencher| {
        bencher.iter(|| black_box(a.add(&b).unwrap()))
    });
}

fn bench_numeric_bigint_add(c: &mut Criterion) {
    let a = NumericType::Int(i64::MAX / 2);
    let b = NumericType::Int(i64::MAX / 2);
    // This will trigger BigInt promotion
    let big_a = a.add(&b).unwrap();

    c.bench_function("numeric bigint add", |bencher| {
        bencher.iter(|| black_box(big_a.add(&NumericType::Int(1000)).unwrap()))
    });
}

fn bench_numeric_ratio_add(c: &mut Criterion) {
    let a = NumericType::make_ratio(1, 3).unwrap();
    let b = NumericType::make_ratio(1, 4).unwrap();

    c.bench_function("numeric ratio add", |bencher| {
        bencher.iter(|| black_box(a.add(&b).unwrap()))
    });
}

fn bench_numeric_int_mul(c: &mut Criterion) {
    let a = NumericType::Int(12345);
    let b = NumericType::Int(67890);

    c.bench_function("numeric int mul", |bencher| {
        bencher.iter(|| black_box(a.mul(&b).unwrap()))
    });
}

fn bench_numeric_division_ratio(c: &mut Criterion) {
    let a = NumericType::Int(7);
    let b = NumericType::Int(3);

    c.bench_function("numeric division creating ratio", |bencher| {
        bencher.iter(|| black_box(a.div(&b).unwrap()))
    });
}

fn bench_numeric_overflow_promotion(c: &mut Criterion) {
    let a = NumericType::Int(i64::MAX);
    let b = NumericType::Int(1);

    c.bench_function("numeric overflow promotion", |bencher| {
        bencher.iter(|| black_box(a.add(&b).unwrap()))
    });
}

fn bench_numeric_comparison(c: &mut Criterion) {
    let a = NumericType::Int(12345);
    let b = NumericType::Int(67890);

    c.bench_function("numeric comparison", |bencher| {
        bencher.iter(|| black_box(a < b))
    });
}

fn bench_numeric_cross_type_comparison(c: &mut Criterion) {
    let int_val = NumericType::Int(5);
    let ratio_val = NumericType::make_ratio(10, 2).unwrap(); // = 5

    c.bench_function("numeric cross-type comparison", |bencher| {
        bencher.iter(|| black_box(int_val == ratio_val))
    });
}

// ============================================================================
// List Operation Benchmarks
// ============================================================================

fn bench_list_cons(c: &mut Criterion) {
    let car = Value::Atom(AtomType::Number(NumericType::Int(42)));
    let cdr = Value::Nil;

    c.bench_function("list cons", |b| {
        b.iter(|| black_box(cons(car.clone(), cdr.clone())))
    });
}

fn bench_list_car(c: &mut Criterion) {
    let list = cons(
        Value::Atom(AtomType::Number(NumericType::Int(42))),
        Value::Nil,
    );

    let mut env = Environment::new();
    register_stdlib(&mut env);
    env.define("list".to_string(), list);

    c.bench_function("list car", |b| {
        b.iter(|| {
            let expr = parse("(car list)").unwrap();
            black_box(eval(expr, &mut env.clone()).unwrap())
        })
    });
}

fn bench_list_cdr(c: &mut Criterion) {
    let list = cons(
        Value::Atom(AtomType::Number(NumericType::Int(1))),
        cons(
            Value::Atom(AtomType::Number(NumericType::Int(2))),
            Value::Nil,
        ),
    );

    let mut env = Environment::new();
    register_stdlib(&mut env);
    env.define("list".to_string(), list);

    c.bench_function("list cdr", |b| {
        b.iter(|| {
            let expr = parse("(cdr list)").unwrap();
            black_box(eval(expr, &mut env.clone()).unwrap())
        })
    });
}

fn bench_list_build_large(c: &mut Criterion) {
    c.bench_function("list build large (100 elements)", |b| {
        b.iter(|| {
            let mut list = Value::Nil;
            for i in (0..100).rev() {
                list = cons(Value::Atom(AtomType::Number(NumericType::Int(i))), list);
            }
            black_box(list)
        })
    });
}

fn bench_list_traverse(c: &mut Criterion) {
    // Build a list of 100 elements
    let mut list = Value::Nil;
    for i in (0..100).rev() {
        list = cons(Value::Atom(AtomType::Number(NumericType::Int(i))), list);
    }

    c.bench_function("list traverse (100 elements)", |b| {
        b.iter(|| {
            let mut current = list.clone();
            let mut count = 0;
            while let Value::Cons(cell) = current {
                count += 1;
                current = cell.cdr.clone();
            }
            black_box(count)
        })
    });
}

// ============================================================================
// Symbol Interning Benchmarks
// ============================================================================

fn bench_symbol_intern(c: &mut Criterion) {
    c.bench_function("symbol intern", |b| {
        let mut counter = 0;
        b.iter(|| {
            let sym = format!("symbol{counter}");
            counter += 1;
            black_box(InternedSymbol::new(&sym))
        })
    });
}

fn bench_symbol_intern_repeated(c: &mut Criterion) {
    c.bench_function("symbol intern repeated", |b| {
        b.iter(|| black_box(InternedSymbol::new("common-symbol")))
    });
}

// ============================================================================
// String Operation Benchmarks
// ============================================================================

fn bench_string_parse(c: &mut Criterion) {
    c.bench_function("string parse basic", |b| {
        b.iter(|| black_box(parse(r#""Hello, World!""#).unwrap()))
    });
}

fn bench_string_parse_unicode(c: &mut Criterion) {
    c.bench_function("string parse unicode", |b| {
        b.iter(|| black_box(parse(r#""Hello, 世界! 🌍""#).unwrap()))
    });
}

fn bench_string_parse_escaped(c: &mut Criterion) {
    c.bench_function("string parse escaped", |b| {
        b.iter(|| black_box(parse(r#""Line 1\nLine 2\tTabbed""#).unwrap()))
    });
}

// ============================================================================
// Comprehensive Evaluation Benchmarks
// ============================================================================

fn bench_eval_comprehensive(c: &mut Criterion) {
    // Use inline lambdas instead of label for simpler benchmarking
    let mut env = Environment::new();
    register_stdlib(&mut env);

    c.bench_function("eval comprehensive program", |b| {
        b.iter(|| {
            let expr = parse(
                "((lambda (square) ((lambda (sum-squares) (sum-squares 3 4)) \
                 (lambda (a b) (+ (square a) (square b))))) \
                 (lambda (x) (* x x)))",
            )
            .unwrap();
            black_box(eval(expr, &mut env.clone()).unwrap())
        })
    });
}

fn bench_eval_cond(c: &mut Criterion) {
    let mut env = Environment::new();
    register_stdlib(&mut env);

    c.bench_function("eval cond expression", |b| {
        b.iter(|| {
            let expr = parse("(cond ((< 5 3) 'less) ((> 5 3) 'greater) (t 'equal))").unwrap();
            black_box(eval(expr, &mut env.clone()).unwrap())
        })
    });
}

fn bench_eval_nested_lambda(c: &mut Criterion) {
    let mut env = Environment::new();
    register_stdlib(&mut env);

    // Nested lambda instead of let (McCarthy-style)
    c.bench_function("eval nested lambda", |b| {
        b.iter(|| {
            let expr = parse("((lambda (x) ((lambda (y) (+ x y)) 20)) 10)").unwrap();
            black_box(eval(expr, &mut env.clone()).unwrap())
        })
    });
}

// ============================================================================
// Criterion Configuration and Main
// ============================================================================

criterion_group! {
    name = parsing_benches;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(10));
    targets =
        bench_parse_small,
        bench_parse_medium,
        bench_parse_large_list,
        bench_parse_deep_nesting,
        bench_parse_quoted_list
}

criterion_group! {
    name = eval_benches;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(10));
    targets =
        bench_eval_simple_arithmetic,
        bench_eval_nested_arithmetic,
        bench_eval_lambda_creation,
        bench_eval_lambda_invocation,
        bench_env_lookup_shallow,
        bench_env_lookup_nested,
        bench_env_define,
        bench_eval_comprehensive,
        bench_eval_cond,
        bench_eval_nested_lambda
}

criterion_group! {
    name = numeric_benches;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(10));
    targets =
        bench_numeric_int_add,
        bench_numeric_bigint_add,
        bench_numeric_ratio_add,
        bench_numeric_int_mul,
        bench_numeric_division_ratio,
        bench_numeric_overflow_promotion,
        bench_numeric_comparison,
        bench_numeric_cross_type_comparison
}

criterion_group! {
    name = list_benches;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(10));
    targets =
        bench_list_cons,
        bench_list_car,
        bench_list_cdr,
        bench_list_build_large,
        bench_list_traverse
}

criterion_group! {
    name = string_benches;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(10));
    targets =
        bench_string_parse,
        bench_string_parse_unicode,
        bench_string_parse_escaped,
        bench_symbol_intern,
        bench_symbol_intern_repeated
}

criterion_group! {
    name = recursive_benches;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(10));
    targets =
        bench_recursive_factorial,
        bench_recursive_fibonacci,
        bench_recursive_list_length,
        bench_recursive_sum_list,
        bench_tco_deep_recursion,
        bench_mutual_recursion
}

criterion_main!(
    parsing_benches,
    eval_benches,
    numeric_benches,
    list_benches,
    string_benches,
    recursive_benches
);
