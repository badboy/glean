use std::env;

use gungraun::{binary_benchmark, binary_benchmark_group, main};

#[binary_benchmark]
fn bench_binary() -> gungraun::Command {
    gungraun::Command::new(env::var("RAPID_METRICS_EXE").expect("need path in RAPID_METRICS_EXE"))
        .args(["-n", "500", "-s", "27230", "-t", "4", "tmp"])
        .build()
}

binary_benchmark_group!(
    name = glean;
    benchmarks = bench_binary
);

main!(binary_benchmark_groups = glean);
