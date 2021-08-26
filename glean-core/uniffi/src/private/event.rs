// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::sync::Arc;

use crate::private::MetricType;
use crate::{CommonMetricData, Glean};

/// Represents the recorded data for a single event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedEventData {
    /// The timestamp of when the event was recorded.
    ///
    /// This allows to order events from a single process run.
    pub timestamp: u64,

    /// The event's category.
    ///
    /// This is defined by users in the metrics file.
    pub category: String,

    /// The event's name.
    ///
    /// This is defined by users in the metrics file.
    pub name: String,

    /// A map of all extra data values.
    ///
    /// The set of allowed extra keys is defined by users in the metrics file.
    pub extra: Option<String>,
}

#[derive(Debug)]
pub struct EventMetric {
    meta: Arc<CommonMetricData>,
}

impl MetricType for EventMetric {
    fn meta(&self) -> &CommonMetricData {
        &self.meta
    }

    fn with_name(&self, name: String) -> Self {
        let mut meta = (*self.meta).clone();
        meta.name = name;
        Self {
            meta: Arc::new(meta),
        }
    }

    fn with_dynamic_label(&self, label: String) -> Self {
        let mut meta = (*self.meta).clone();
        meta.dynamic_label = Some(label);
        Self {
            meta: Arc::new(meta),
        }
    }
}

impl EventMetric {
    /// The public constructor used by automatically generated metrics.
    pub fn new(meta: CommonMetricData, allowed_extra_keys: Vec<String>) -> Self {
        Self {
            meta: Arc::new(meta),
        }
    }

    pub fn internal_record(&self, extra: Option<String>) {
        let metric = self.clone();
        crate::launch_with_glean(move |glean| {
            log::info!("Recording event with extra {:?}", extra);
        })
    }

    pub(crate) fn get_value(&self, glean: &Glean, ping_name: Option<&str>) -> Option<RecordedEventData> {
        let queried_ping_name = ping_name.unwrap_or_else(|| &self.meta().send_in_pings[0]);

        None
    }

    pub fn test_get_value(&self, ping_name: Option<String>) -> Option<RecordedEventData> {
        crate::block_on_dispatcher();
        crate::core::with_glean(|glean| self.get_value(glean, ping_name.as_deref()))
    }
}
