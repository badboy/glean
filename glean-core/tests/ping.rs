// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod common;
use crate::common::*;

use std::collections::HashMap;

use glean_core::metrics::*;
use glean_core::CommonMetricData;
use glean_core::LabeledMetricData;
use glean_core::Lifetime;

#[test]
fn write_ping_to_disk() {
    let (mut glean, _temp) = new_glean(None);

    let ping = new_test_ping(&mut glean, "store1");

    // We need to store a metric as an empty ping is not stored.
    let counter = CounterMetric::new(CommonMetricData {
        name: "counter".into(),
        category: "local".into(),
        send_in_pings: vec!["store1".into()],
        ..Default::default()
    });
    counter.add_sync(&glean, 1);

    assert!(ping.submit_sync(&glean, None));

    assert_eq!(1, get_queued_pings(glean.get_data_path()).unwrap().len());
}

#[test]
fn disabling_upload_clears_pending_pings() {
    let (mut glean, _t) = new_glean(None);

    let ping = new_test_ping(&mut glean, "store1");

    // We need to store a metric as an empty ping is not stored.
    let counter = CounterMetric::new(CommonMetricData {
        name: "counter".into(),
        category: "local".into(),
        send_in_pings: vec!["store1".into()],
        ..Default::default()
    });

    counter.add_sync(&glean, 1);
    assert!(ping.submit_sync(&glean, None));
    assert_eq!(1, get_queued_pings(glean.get_data_path()).unwrap().len());
    // At this point no deletion_request ping should exist
    // (that is: it's directory should not exist at all)
    assert!(get_deletion_pings(glean.get_data_path()).is_err());

    glean.set_upload_enabled(false);
    assert_eq!(0, get_queued_pings(glean.get_data_path()).unwrap().len());
    // Disabling upload generates a deletion ping
    let dpings = get_deletion_pings(glean.get_data_path()).unwrap();
    assert_eq!(1, dpings.len());
    let payload = &dpings[0].1;
    assert_eq!(
        "set_upload_enabled",
        payload["ping_info"].as_object().unwrap()["reason"]
            .as_str()
            .unwrap()
    );

    glean.set_upload_enabled(true);
    assert_eq!(0, get_queued_pings(glean.get_data_path()).unwrap().len());

    counter.add_sync(&glean, 1);
    assert!(ping.submit_sync(&glean, None));
    assert_eq!(1, get_queued_pings(glean.get_data_path()).unwrap().len());
}

#[test]
fn deletion_request_only_when_toggled_from_on_to_off() {
    let (mut glean, _t) = new_glean(None);

    // Disabling upload generates a deletion ping
    glean.set_upload_enabled(false);
    let dpings = get_deletion_pings(glean.get_data_path()).unwrap();
    assert_eq!(1, dpings.len());
    let payload = &dpings[0].1;
    assert_eq!(
        "set_upload_enabled",
        payload["ping_info"].as_object().unwrap()["reason"]
            .as_str()
            .unwrap()
    );

    // Re-setting it to `false` should not generate an additional ping.
    // As we didn't clear the pending ping, that's the only one that sticks around.
    glean.set_upload_enabled(false);
    assert_eq!(1, get_deletion_pings(glean.get_data_path()).unwrap().len());

    // Toggling back to true won't generate a ping either.
    glean.set_upload_enabled(true);
    assert_eq!(1, get_deletion_pings(glean.get_data_path()).unwrap().len());
}

#[test]
fn empty_pings_with_flag_are_sent() {
    let (mut glean, _t) = new_glean(None);

    let ping1 = PingBuilder::new("custom-ping1")
        .with_send_if_empty(true)
        .build();
    glean.register_ping_type(&ping1);
    let ping2 = PingBuilder::new("custom-ping2").build();
    glean.register_ping_type(&ping2);

    // No data is stored in either of the custom pings

    // Sending this should succeed.
    assert!(ping1.submit_sync(&glean, None));
    assert_eq!(1, get_queued_pings(glean.get_data_path()).unwrap().len());

    // Sending this should fail.
    assert!(!ping2.submit_sync(&glean, None));
    assert_eq!(1, get_queued_pings(glean.get_data_path()).unwrap().len());
}

#[test]
fn test_pings_submitted_metric() {
    let (mut glean, temp) = new_glean(None);

    let _guard = DropGuard::new(temp, |temp| {
        if std::thread::panicking() {
            let path = format!("{}/db/glean.sqlite", temp.path().display());
            std::process::Command::new("sqlite3")
                .arg(&path)
                .arg("SELECT * FROM telemetry")
                .stdout(std::io::stderr())
                .status()
                .unwrap();
        }
    });

    // Reconstructed here so we can test it without reaching into the library
    // internals.
    let pings_submitted = LabeledCounter::new(
        LabeledMetricData::Common {
            cmd: CommonMetricData {
                name: "pings_submitted".into(),
                category: "glean.validation".into(),
                send_in_pings: vec!["metrics".into(), "baseline".into()],
                lifetime: Lifetime::Ping,
                disabled: false,
                dynamic_label: None,
            },
        },
        None,
    );

    let metrics_ping = new_test_ping(&mut glean, "metrics");
    let baseline_ping = new_test_ping(&mut glean, "baseline");

    let custom_ping = PingBuilder::new("custom").with_send_if_empty(true).build();
    glean.register_ping_type(&custom_ping);

    // We need to store a metric as an empty ping is not stored.
    let counter = CounterMetric::new(CommonMetricData {
        name: "counter".into(),
        category: "local".into(),
        send_in_pings: vec!["metrics".into()],
        ..Default::default()
    });
    counter.add_sync(&glean, 1);

    assert!(metrics_ping.submit_sync(&glean, None));
    // A custom ping is just never recorded.
    assert!(custom_ping.submit_sync(&glean, None));

    // Check recording in the metrics ping
    assert_eq!(
        Some(1),
        pings_submitted.get("metrics").get_value(&glean, "metrics")
    );
    assert_eq!(
        None,
        pings_submitted.get("baseline").get_value(&glean, "metrics")
    );
    assert_eq!(
        None,
        pings_submitted.get("custom").get_value(&glean, "metrics")
    );

    // Check recording in the baseline ping
    assert_eq!(
        Some(1),
        pings_submitted.get("metrics").get_value(&glean, "baseline")
    );
    assert_eq!(
        None,
        pings_submitted
            .get("baseline")
            .get_value(&glean, "baseline")
    );
    assert_eq!(
        None,
        pings_submitted.get("custom").get_value(&glean, "baseline")
    );

    // Trigger 2 baseline pings.
    // This should record a count of 2 baseline pings in the metrics ping, but
    // it resets each time on the baseline ping, so we should only ever get 1
    // baseline ping recorded in the baseline ping itsef.
    assert!(baseline_ping.submit_sync(&glean, None));
    assert!(baseline_ping.submit_sync(&glean, None));
    // A custom ping is just never recorded.
    assert!(custom_ping.submit_sync(&glean, None));

    // Check recording in the metrics ping
    assert_eq!(
        Some(1),
        pings_submitted
            .get("metrics")
            .get_value(&glean, Some("metrics"))
    );
    assert_eq!(
        Some(2),
        pings_submitted
            .get("baseline")
            .get_value(&glean, Some("metrics"))
    );
    assert_eq!(
        None,
        pings_submitted
            .get("custom")
            .get_value(&glean, Some("metrics"))
    );

    // Check recording in the baseline ping
    assert_eq!(
        None,
        pings_submitted
            .get("metrics")
            .get_value(&glean, Some("baseline"))
    );
    assert_eq!(
        Some(1),
        pings_submitted
            .get("baseline")
            .get_value(&glean, Some("baseline"))
    );
    assert_eq!(
        None,
        pings_submitted
            .get("custom")
            .get_value(&glean, Some("baseline"))
    );
}

#[test]
fn events_ping_with_metric_but_no_events_is_not_sent() {
    let (mut glean, _t) = new_glean(None);

    let events_ping = new_test_ping(&mut glean, "events");
    let counter = CounterMetric::new(CommonMetricData {
        name: "counter".into(),
        category: "local".into(),
        send_in_pings: vec!["events".into()],
        ..Default::default()
    });
    counter.add_sync(&glean, 1);

    // Sending this should fail.
    assert!(!events_ping.submit_sync(&glean, None));
    assert!(get_queued_pings(glean.get_data_path()).is_err());

    let event = EventMetric::new(
        CommonMetricData {
            name: "name".into(),
            category: "category".into(),
            send_in_pings: vec!["events".into()],
            ..Default::default()
        },
        vec![],
    );
    event.record_sync(&glean, 0, HashMap::new(), 0);

    // Sending this should now succeed.
    assert!(events_ping.submit_sync(&glean, None));
    assert_eq!(1, get_queued_pings(glean.get_data_path()).unwrap().len());
}

#[test]
fn test_scheduled_pings_are_sent() {
    let (mut glean, _t) = new_glean(None);

    let piggyback_ping = PingBuilder::new("piggyback")
        .with_send_if_empty(true)
        .build();
    glean.register_ping_type(&piggyback_ping);

    let trigger_ping = PingBuilder::new("trigger")
        .with_send_if_empty(true)
        .with_schedules_pings(vec!["piggyback".into()])
        .build();
    glean.register_ping_type(&trigger_ping);

    assert!(trigger_ping.submit_sync(&glean, None));
    assert_eq!(2, get_queued_pings(glean.get_data_path()).unwrap().len());
}

#[test]
#[ignore] // This metric is disabled by default now, so we can skip this test. See Bug 1928161
fn database_write_timings_get_recorded() {
    let (mut glean, _t) = new_glean(None);

    let metrics_ping = new_test_ping(&mut glean, "metrics");
    glean.register_ping_type(&metrics_ping);

    // We need to store a metric to record something.
    let counter = CounterMetric::new(CommonMetricData {
        name: "counter".into(),
        category: "local".into(),
        send_in_pings: vec!["metrics".into()],
        ..Default::default()
    });
    counter.add_sync(&glean, 1);

    assert!(metrics_ping.submit_sync(&glean, None));

    let mut queued_pings = get_queued_pings(glean.get_data_path()).unwrap();
    assert_eq!(1, queued_pings.len(), "missing metrics ping");

    let json = queued_pings.pop().unwrap().1;
    let write_time = &json["metrics"]["timing_distribution"]["glean.database.write_time"];
    assert!(
        0 < write_time["sum"].as_i64().unwrap(),
        "writing should take some time"
    );
}

// DROPGUARD

use std::fmt::{self, Debug};
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

pub struct DropGuard<T, F>
where
    F: FnOnce(T),
{
    inner: ManuallyDrop<T>,
    f: ManuallyDrop<F>,
}

impl<T, F> DropGuard<T, F>
where
    F: FnOnce(T),
{
    #[must_use]
    pub const fn new(inner: T, f: F) -> Self {
        Self {
            inner: ManuallyDrop::new(inner),
            f: ManuallyDrop::new(f),
        }
    }

    #[inline]
    pub fn dismiss(self) -> T {
        // First we ensure that dropping the guard will not trigger
        // its destructor
        let mut this = ManuallyDrop::new(self);

        // Next we manually read the stored value from the guard.
        //
        // SAFETY: this is safe because we've taken ownership of the guard.
        let value = unsafe { ManuallyDrop::take(&mut this.inner) };

        // Finally we drop the stored closure. We do this *after* having read
        // the value, so that even if the closure's `drop` function panics,
        // unwinding still tries to drop the value.
        //
        // SAFETY: this is safe because we've taken ownership of the guard.
        unsafe { ManuallyDrop::drop(&mut this.f) };
        value
    }
}

impl<T, F> Deref for DropGuard<T, F>
where
    F: FnOnce(T),
{
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T, F> DerefMut for DropGuard<T, F>
where
    F: FnOnce(T),
{
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T, F> Drop for DropGuard<T, F>
where
    F: FnOnce(T),
{
    fn drop(&mut self) {
        // SAFETY: `DropGuard` is in the process of being dropped.
        let inner = unsafe { ManuallyDrop::take(&mut self.inner) };

        // SAFETY: `DropGuard` is in the process of being dropped.
        let f = unsafe { ManuallyDrop::take(&mut self.f) };

        f(inner);
    }
}

impl<T, F> Debug for DropGuard<T, F>
where
    T: Debug,
    F: FnOnce(T),
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}
