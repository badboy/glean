// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::env;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use once_cell::sync::Lazy;
use tempfile::Builder;

use glean::{private::PingType, ClientInfoMetrics, Configuration};

pub mod glean_metrics {
    use glean::{private::CounterMetric, CommonMetricData, Lifetime};

    #[allow(non_upper_case_globals)]
    pub static sample_counter: once_cell::sync::Lazy<CounterMetric> =
        once_cell::sync::Lazy::new(|| {
            CounterMetric::new(CommonMetricData {
                name: "sample_counter".into(),
                category: "test.metrics".into(),
                send_in_pings: vec!["prototype".into()],
                disabled: false,
                lifetime: Lifetime::Ping,
                ..Default::default()
            })
        });
}

#[allow(non_upper_case_globals)]
pub static PrototypePing: Lazy<PingType> =
    Lazy::new(|| PingType::new("prototype", true, true, vec![]));

fn main() {
    env_logger::init();

    let mut args = env::args().skip(1);

    let data_path = if let Some(path) = args.next() {
        PathBuf::from(path)
    } else {
        let root = Builder::new().prefix("simple-db").tempdir().unwrap();
        root.path().to_path_buf()
    };

    let delay_ping_lifetime_io = args.next().map(|a| a == "delay").unwrap_or(false);
    let submit_ping = !args.next().map(|a| a == "skip").unwrap_or(false);

    let cfg = Configuration {
        data_path,
        application_id: "org.mozilla.glean_core.example".into(),
        upload_enabled: true,
        max_events: None,
        delay_ping_lifetime_io,
        channel: None,
        server_endpoint: Some("invalid-test-host".into()),
        uploader: None,
        use_core_mps: true,
    };

    let client_info = ClientInfoMetrics {
        app_build: env!("CARGO_PKG_VERSION").to_string(),
        app_display_version: env!("CARGO_PKG_VERSION").to_string(),
    };

    glean::register_ping_type(&PrototypePing);
    glean::initialize(cfg, client_info);

    // Let's give Glean just a bit to initialize.
    thread::sleep(Duration::from_millis(500));

    glean_metrics::sample_counter.add(1);

    if submit_ping {
        log::info!("Asked to submit a ping.");
        glean::submit_ping_by_name("prototype", None);
    } else {
        log::info!("No ping submitted.");
    }

    glean::shutdown();
}
