#[cfg(feature = "gungraun")]
use std::env;

#[cfg(feature = "gungraun")]
use gungraun::{binary_benchmark, binary_benchmark_group, main};

#[cfg(not(feature = "gungraun"))]
fn main() {
    eprintln!("--feature gungraun required. Linux only.");
}

#[cfg(feature = "gungraun")]
#[binary_benchmark]
fn bench_binary() -> gungraun::Command {
    gungraun::Command::new(env::var("RAPID_METRICS_EXE").expect("need path in RAPID_METRICS_EXE"))
        .args(["-n", "500", "-s", "27230", "-t", "4", "tmp"])
        .build()
}

#[cfg(feature = "gungraun")]
binary_benchmark_group!(
    name = glean;
    benchmarks = bench_binary
);

#[cfg(feature = "gungraun")]
main!(binary_benchmark_groups = glean);
