// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ffi::CString;

use glean_core::{ErrorType, Glean};
use inherent::inherent;

use crate::{dispatcher, new_metric};

/// This implements the developer facing API for recording string metrics.
///
/// Instances of this class type are automatically generated by the parsers
/// at build time, allowing developers to record values that were previously
/// registered in the metrics.yaml file.
#[derive(Clone)]
pub struct StringMetric(pub(crate) u64);

impl StringMetric {
    /// The public constructor used by automatically generated metrics.
    pub fn new(meta: glean_core::CommonMetricData) -> Self {
        Self(new_metric!(glean_new_string_metric, meta))
    }

    /// Internal only, synchronous API for setting a string value.
    pub(crate) fn set_sync<S: Into<std::string::String>>(&self, _glean: &Glean, value: S) {
        let id = self.0;
        let new_value = value.into();
        let value = CString::new(new_value).unwrap();
        crate::sys::with_glean(|glean| unsafe { glean.glean_string_set(id, value.as_ptr()) });
    }
}

#[inherent(pub)]
impl glean_core::traits::String for StringMetric {
    fn set<S: Into<std::string::String>>(&self, value: S) {
        let id = self.0;
        let new_value = value.into();
        dispatcher::launch(move || {
            let value = CString::new(new_value).unwrap();
            crate::sys::with_glean(|glean| unsafe { glean.glean_string_set(id, value.as_ptr()) });
        });
        todo!()
    }

    fn test_get_value<'a, S: Into<Option<&'a str>>>(
        &self,
        _ping_name: S,
    ) -> Option<std::string::String> {
        crate::block_on_dispatcher();
        None
    }

    fn test_get_num_recorded_errors<'a, S: Into<Option<&'a str>>>(
        &self,
        _error: ErrorType,
        _ping_name: S,
    ) -> i32 {
        crate::block_on_dispatcher();
        0
    }
}