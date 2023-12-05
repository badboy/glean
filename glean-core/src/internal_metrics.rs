// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::borrow::Cow;

use super::{metrics::*, CommonMetricData, Lifetime};

#[derive(Debug)]
pub struct CoreMetrics {
    pub client_id: UuidMetric,
    pub first_run_date: DatetimeMetric,
    pub os: StringMetric,
}

#[derive(Debug)]
pub struct AdditionalMetrics {
    /// The number of times we encountered an IO error
    /// when writing a pending ping to disk.
    pub io_errors: CounterMetric,

    /// A count of the pings submitted, by ping type.
    pub pings_submitted: LabeledMetric<CounterMetric>,

    /// Time waited for the uploader at shutdown.
    pub shutdown_wait: TimingDistributionMetric,

    /// Time waited for the dispatcher to unblock during shutdown.
    pub shutdown_dispatcher_wait: TimingDistributionMetric,

    /// An experimentation identifier derived and provided by the application
    /// for the purpose of experimentation enrollment.
    pub experimentation_id: StringMetric,
}

impl CoreMetrics {
    pub fn new() -> CoreMetrics {
        CoreMetrics {
            client_id: UuidMetric::new(CommonMetricData {
                identifier: "client_id".into(),
                send_in_pings: vec!["glean_client_info".into()],
                lifetime: Lifetime::User,
                disabled: false,
                dynamic_label: None,
            }),

            first_run_date: DatetimeMetric::new(
                CommonMetricData {
                    identifier: "first_run_date".into(),
                    send_in_pings: vec!["glean_client_info".into()],
                    lifetime: Lifetime::User,
                    disabled: false,
                    dynamic_label: None,
                },
                TimeUnit::Day,
            ),

            os: StringMetric::new(CommonMetricData {
                identifier: "os".into(),
                send_in_pings: vec!["glean_client_info".into()],
                lifetime: Lifetime::Application,
                disabled: false,
                dynamic_label: None,
            }),
        }
    }
}

impl AdditionalMetrics {
    pub fn new() -> AdditionalMetrics {
        AdditionalMetrics {
            io_errors: CounterMetric::new(CommonMetricData {
                identifier: "glean.error.io".into(),
                send_in_pings: vec!["metrics".into()],
                lifetime: Lifetime::Ping,
                disabled: false,
                dynamic_label: None,
            }),

            pings_submitted: LabeledMetric::<CounterMetric>::new(
                CommonMetricData {
                    identifier: "glean.validation.pings_submitted".into(),
                    send_in_pings: vec!["metrics".into(), "baseline".into()],
                    lifetime: Lifetime::Ping,
                    disabled: false,
                    dynamic_label: None,
                },
                None,
            ),

            shutdown_wait: TimingDistributionMetric::new(
                CommonMetricData {
                    identifier: "glean.validation.shutdown_wait".into(),
                    send_in_pings: vec!["metrics".into()],
                    lifetime: Lifetime::Ping,
                    disabled: false,
                    dynamic_label: None,
                },
                TimeUnit::Millisecond,
            ),

            shutdown_dispatcher_wait: TimingDistributionMetric::new(
                CommonMetricData {
                    identifier: "glean.validation.shutdown_dispatcher_wait".into(),
                    send_in_pings: vec!["metrics".into()],
                    lifetime: Lifetime::Ping,
                    disabled: false,
                    dynamic_label: None,
                },
                TimeUnit::Millisecond,
            ),

            // This uses a `send_in_pings` that contains "all-ping".
            // This works because all of our other current "all-pings" metrics
            // have special handling internally and are not actually processed
            // into a store quite like this identifier is.
            //
            // This could become an issue if we ever decide to start generating
            // code from the internal Glean metrics.yaml (there aren't currently
            // any plans for this).
            experimentation_id: StringMetric::new(CommonMetricData {
                identifier: "glean.client.annotation.experimentation_id".into(),
                send_in_pings: vec!["all-pings".into()],
                lifetime: Lifetime::Application,
                disabled: false,
                dynamic_label: None,
            }),
        }
    }
}

#[derive(Debug)]
pub struct UploadMetrics {
    pub ping_upload_failure: LabeledMetric<CounterMetric>,
    pub discarded_exceeding_pings_size: MemoryDistributionMetric,
    pub pending_pings_directory_size: MemoryDistributionMetric,
    pub deleted_pings_after_quota_hit: CounterMetric,
    pub pending_pings: CounterMetric,
    pub send_success: TimingDistributionMetric,
    pub send_failure: TimingDistributionMetric,
    pub in_flight_pings_dropped: CounterMetric,
    pub missing_send_ids: CounterMetric,
}

impl UploadMetrics {
    pub fn new() -> UploadMetrics {
        UploadMetrics {
            ping_upload_failure: LabeledMetric::<CounterMetric>::new(
                CommonMetricData {
                    identifier: "glean.upload.ping_upload_failure".into(),
                    send_in_pings: vec!["metrics".into()],
                    lifetime: Lifetime::Ping,
                    disabled: false,
                    dynamic_label: None,
                },
                Some(vec![
                    Cow::from("status_code_4xx"),
                    Cow::from("status_code_5xx"),
                    Cow::from("status_code_unknown"),
                    Cow::from("unrecoverable"),
                    Cow::from("recoverable"),
                ]),
            ),

            discarded_exceeding_pings_size: MemoryDistributionMetric::new(
                CommonMetricData {
                    identifier: "glean.upload.discarded_exceeding_ping_size".into(),
                    send_in_pings: vec!["metrics".into()],
                    lifetime: Lifetime::Ping,
                    disabled: false,
                    dynamic_label: None,
                },
                MemoryUnit::Kilobyte,
            ),

            pending_pings_directory_size: MemoryDistributionMetric::new(
                CommonMetricData {
                    identifier: "glean.upload.pending_pings_directory_size".into(),
                    send_in_pings: vec!["metrics".into()],
                    lifetime: Lifetime::Ping,
                    disabled: false,
                    dynamic_label: None,
                },
                MemoryUnit::Kilobyte,
            ),

            deleted_pings_after_quota_hit: CounterMetric::new(CommonMetricData {
                identifier: "glean.upload.deleted_pings_after_quota_hit".into(),
                send_in_pings: vec!["metrics".into()],
                lifetime: Lifetime::Ping,
                disabled: false,
                dynamic_label: None,
            }),

            pending_pings: CounterMetric::new(CommonMetricData {
                identifier: "glean.upload.pending_pings".into(),
                send_in_pings: vec!["metrics".into()],
                lifetime: Lifetime::Ping,
                disabled: false,
                dynamic_label: None,
            }),

            send_success: TimingDistributionMetric::new(
                CommonMetricData {
                    identifier: "glean.upload.send_success".into(),
                    send_in_pings: vec!["metrics".into()],
                    lifetime: Lifetime::Ping,
                    disabled: false,
                    dynamic_label: None,
                },
                TimeUnit::Millisecond,
            ),

            send_failure: TimingDistributionMetric::new(
                CommonMetricData {
                    identifier: "glean.upload.send_failure".into(),
                    send_in_pings: vec!["metrics".into()],
                    lifetime: Lifetime::Ping,
                    disabled: false,
                    dynamic_label: None,
                },
                TimeUnit::Millisecond,
            ),

            in_flight_pings_dropped: CounterMetric::new(CommonMetricData {
                identifier: "glean.upload.in_flight_pings_dropped".into(),
                send_in_pings: vec!["metrics".into()],
                lifetime: Lifetime::Ping,
                disabled: false,
                dynamic_label: None,
            }),

            missing_send_ids: CounterMetric::new(CommonMetricData {
                identifier: "glean.upload.missing_send_ids".into(),
                send_in_pings: vec!["metrics".into()],
                lifetime: Lifetime::Ping,
                disabled: false,
                dynamic_label: None,
            }),
        }
    }
}

#[derive(Debug)]
pub struct DatabaseMetrics {
    pub size: MemoryDistributionMetric,
}

impl DatabaseMetrics {
    pub fn new() -> DatabaseMetrics {
        DatabaseMetrics {
            size: MemoryDistributionMetric::new(
                CommonMetricData {
                    identifier: "glean.database.size".into(),
                    send_in_pings: vec!["metrics".into()],
                    lifetime: Lifetime::Ping,
                    disabled: false,
                    dynamic_label: None,
                },
                MemoryUnit::Byte,
            ),
        }
    }
}
