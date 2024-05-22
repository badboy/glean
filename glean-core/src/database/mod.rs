// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::fs;
use std::num::NonZeroU64;
use std::panic::AssertUnwindSafe;
use std::path::Path;
use std::str;

/// Unwrap a `Result`s `Ok` value or do the specified action.
///
/// This is an alternative to the question-mark operator (`?`),
/// when the other action should not be to return the error.
macro_rules! unwrap_or {
    ($expr:expr, $or:expr) => {
        match $expr {
            Ok(x) => x,
            Err(_) => {
                $or;
            }
        }
    };
}

use rusqlite::{params, Connection, OpenFlags};

use crate::common_metric_data::CommonMetricDataInternal;
use crate::metrics::Metric;
use crate::Glean;
use crate::Lifetime;
use crate::Result;

const SCHEMA: &str = r#"
BEGIN;
CREATE TABLE IF NOT EXISTS telemetry
(
  id TEXT NOT NULL,
  ping TEXT NOT NULL,
  lifetime TEXT NOT NULL,
  value BLOB,
  updated_at TEXT NOT NULL DEFAULT (DATETIME('now')),
  UNIQUE(id, ping)
);
CREATE TABLE IF NOT EXISTS pings
(
  id TEXT NOT NULL,
  ping TEXT NOT NULL,
  payload TEXT NOT NULL,
  state TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT (DATETIME('now')),
  UNIQUE(id, ping)
);
COMMIT;
"#;

pub struct Database {
    /// The database connection.
    ///
    /// FIXME: It's probably not unwind safe.
    conn: AssertUnwindSafe<Connection>,
}

impl std::fmt::Debug for Database {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("Database")
            .field("conn", &self.conn)
            .finish()
    }
}

impl Database {
    /// Initializes the data store.
    ///
    /// This opens the underlying SQLite store and creates
    /// the underlying directory structure.
    pub fn new(data_path: &Path, _delay_ping_lifetime_io: bool) -> Result<Self> {
        let path = data_path.join("db");
        log::debug!("Database path: {:?}", path.display());

        fs::create_dir_all(&path)?;
        let sql_path = path.join("telemetry.db");
        let conn = Connection::open_with_flags(
            sql_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )?;

        // as per application-servers, components/places/src/db/db.rs
        #[cfg(target_os = "android")]
        {
            // `temp_store = 2` is required on Android to force the DB to keep temp
            // files in memory, since on Android there's no tmp partition. See
            // https://github.com/mozilla/mentat/issues/505.
            db.set_pragma("temp_store", 2)?;
        }
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "wal_autocheckpoint", 62)?;
        conn.pragma_update(None, "page_size", 32768)?;
        conn.pragma_update(None, "cache_size", -6144)?;
        conn.execute_batch(SCHEMA)?;
        conn.set_prepared_statement_cache_capacity(128);

        let db = Self {
            conn: AssertUnwindSafe(conn),
        };

        Ok(db)
    }

    /// Get the initial database file size.
    pub fn file_size(&self) -> Option<NonZeroU64> {
        None
    }

    /// Get the rkv load state.
    pub fn rkv_load_state(&self) -> Option<String> {
        None
    }

    /// Iterates with the provided transaction function
    /// over the requested data from the given storage.
    ///
    /// * If the storage is unavailable, the transaction function is never invoked.
    /// * If the read data cannot be deserialized it will be silently skipped.
    ///
    /// # Arguments
    ///
    /// * `lifetime` - The metric lifetime to iterate over.
    /// * `storage_name` - The storage name to iterate over.
    /// * `metric_key` - The metric key to iterate over. All metrics iterated over
    ///   will have this prefix. For example, if `metric_key` is of the form `{category}.`,
    ///   it will iterate over all metrics in the given category. If the `metric_key` is of the
    ///   form `{category}.{name}/`, the iterator will iterate over all specific metrics for
    ///   a given labeled metric. If not provided, the entire storage for the given lifetime
    ///   will be iterated over.
    /// * `transaction_fn` - Called for each entry being iterated over. It is
    ///   passed two arguments: `(metric_id: &[u8], metric: &Metric)`.
    ///
    /// # Panics
    ///
    /// This function will **not** panic on database errors.
    pub fn iter_store_from<F>(
        &self,
        lifetime: Lifetime,
        storage_name: &str,
        metric_key: Option<&str>,
        mut transaction_fn: F,
    ) where
        F: FnMut(&[u8], &Metric),
    {
        let iter_sql = r#"
        SELECT id, value
        FROM telemetry
        WHERE
            lifetime = ?1
            AND ping = ?2
        "#;

        let iter_sql = if metric_key.is_some() {
            format!("{} AND id LIKE ?3 || '%'", iter_sql)
        } else {
            iter_sql.to_string()
        };
        let mut stmt = unwrap_or!(self.conn.prepare_cached(&iter_sql), return);
        let mut rows = if let Some(metric_key) = metric_key {
            unwrap_or!(
                stmt.query(params![
                    lifetime.as_str().to_string(),
                    storage_name,
                    metric_key
                ]),
                return
            )
        } else {
            unwrap_or!(
                stmt.query(params![lifetime.as_str().to_string(), storage_name]),
                return
            )
        };

        while let Ok(Some(row)) = rows.next() {
            let metric_id: String = unwrap_or!(row.get(0), continue);
            let blob: Vec<u8> = unwrap_or!(row.get(1), continue);
            let metric: Metric = match bincode::deserialize(&blob) {
                Ok(metric) => metric,
                _ => continue,
            };
            transaction_fn(metric_id.as_bytes(), &metric);
        }
    }

    /// Determines if the storage has the given metric.
    ///
    /// If data cannot be read it is assumed that the storage does not have the metric.
    ///
    /// # Arguments
    ///
    /// * `lifetime` - The lifetime of the metric.
    /// * `storage_name` - The storage name to look in.
    /// * `metric_identifier` - The metric identifier.
    ///
    /// # Panics
    ///
    /// This function will **not** panic on database errors.
    pub fn has_metric(
        &self,
        lifetime: Lifetime,
        storage_name: &str,
        metric_identifier: &str,
    ) -> bool {
        let has_metric_sql = r#"
        SELECT id
        FROM telemetry
        WHERE
            lifetime = ?1
            AND ping = ?2
            AND id = ?3
        "#;

        let mut stmt = unwrap_or!(self.conn.prepare_cached(has_metric_sql), return false);
        let mut metric_iter = unwrap_or!(
            stmt.query([lifetime.as_str(), storage_name, metric_identifier]),
            return false
        );

        metric_iter.next().map(|m| m.is_some()).unwrap_or(false)
    }

    /// Records a metric in the underlying storage system.
    pub fn record(&self, glean: &Glean, data: &CommonMetricDataInternal, value: &Metric) {
        // If upload is disabled we don't want to record.
        if !glean.is_upload_enabled() {
            return;
        }

        let name = data.identifier(glean);

        for ping_name in data.storage_names() {
            if let Err(e) = self.record_per_lifetime(data.inner.lifetime, ping_name, &name, value) {
                log::error!(
                    "Failed to record metric '{}' into {}: {:?}",
                    data.base_identifier(),
                    ping_name,
                    e
                );
            }
        }
    }

    /// Records a metric in the underlying storage system, for a single lifetime.
    ///
    /// # Returns
    ///
    /// If the storage is unavailable or the write fails, no data will be stored and an error will be returned.
    ///
    /// Otherwise `Ok(())` is returned.
    ///
    /// # Panics
    ///
    /// This function will **not** panic on database errors.
    fn record_per_lifetime(
        &self,
        lifetime: Lifetime,
        storage_name: &str,
        key: &str,
        metric: &Metric,
    ) -> Result<()> {
        let insert_sql = r#"
        INSERT INTO
            telemetry (id, ping, lifetime, value, updated_at)
        VALUES
            (?1, ?2, ?3, ?4, DATETIME('now'))
        ON CONFLICT(id, ping) DO UPDATE SET
            lifetime = excluded.lifetime,
            value = excluded.value,
            updated_at = excluded.updated_at
        "#;

        let mut stmt = self.conn.prepare_cached(insert_sql)?;
        let encoded = bincode::serialize(&metric).expect("IMPOSSIBLE: Serializing metric failed");
        stmt.execute(params![key, storage_name, lifetime.as_str(), encoded])?;

        Ok(())
    }

    /// Records the provided value, with the given lifetime,
    /// after applying a transformation function.
    pub fn record_with<F>(&self, glean: &Glean, data: &CommonMetricDataInternal, mut transform: F)
    where
        F: FnMut(Option<Metric>) -> Metric,
    {
        // If upload is disabled we don't want to record.
        if !glean.is_upload_enabled() {
            return;
        }

        let name = data.identifier(glean);
        for ping_name in data.storage_names() {
            if let Err(e) =
                self.record_per_lifetime_with(data.inner.lifetime, ping_name, &name, &mut transform)
            {
                log::error!(
                    "Failed to record metric '{}' into {}: {:?}",
                    data.base_identifier(),
                    ping_name,
                    e
                );
            }
        }
    }

    /// Records a metric in the underlying storage system,
    /// after applying the given transformation function, for a single lifetime.
    ///
    /// # Returns
    ///
    /// If the storage is unavailable or the write fails, no data will be stored and an error will be returned.
    ///
    /// Otherwise `Ok(())` is returned.
    ///
    /// # Panics
    ///
    /// This function will **not** panic on database errors.
    fn record_per_lifetime_with<F>(
        &self,
        lifetime: Lifetime,
        storage_name: &str,
        key: &str,
        mut transform: F,
    ) -> Result<()>
    where
        F: FnMut(Option<Metric>) -> Metric,
    {
        let find_sql = r#"
        SELECT value
        FROM telemetry
        WHERE
            lifetime = ?1
            AND ping = ?2
            AND id = ?3
        LIMIT 1
        "#;

        let tx = self.conn.unchecked_transaction()?;
        let new_value = {
            let mut stmt = tx.prepare_cached(&find_sql)?;
            let mut rows = stmt.query(params![lifetime.as_str().to_string(), storage_name, key])?;

            if let Ok(Some(row)) = rows.next() {
                let blob: Vec<u8> = row.get(0)?;
                let old_value = bincode::deserialize(&blob).ok();
                transform(old_value)
            } else {
                transform(None)
            }
        };

        let insert_sql = r#"
        INSERT INTO
            telemetry (id, ping, lifetime, value, updated_at)
        VALUES
            (?1, ?2, ?3, ?4, DATETIME('now'))
        ON CONFLICT(id, ping) DO UPDATE SET
            lifetime = excluded.lifetime,
            value = excluded.value,
            updated_at = excluded.updated_at
        "#;

        {
            let mut stmt = tx.prepare_cached(insert_sql)?;
            let encoded =
                bincode::serialize(&new_value).expect("IMPOSSIBLE: Serializing metric failed");
            stmt.execute(params![key, storage_name, lifetime.as_str(), encoded])?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Clears a storage (only Ping Lifetime).
    ///
    /// # Returns
    ///
    /// * If the storage is unavailable an error is returned.
    /// * If any individual delete fails, an error is returned, but other deletions might have
    ///   happened.
    ///
    /// Otherwise `Ok(())` is returned.
    ///
    /// # Panics
    ///
    /// This function will **not** panic on database errors.
    pub fn clear_ping_lifetime_storage(&self, storage_name: &str) -> Result<()> {
        let clear_sql = "DELETE FROM telemetry WHERE lifetime = ?1 AND ping = ?2";
        let mut stmt = self.conn.prepare_cached(clear_sql)?;
        stmt.execute([Lifetime::Ping.as_str(), storage_name])?;
        Ok(())
    }

    /// Removes a single metric from the storage.
    ///
    /// # Arguments
    ///
    /// * `lifetime` - the lifetime of the storage in which to look for the metric.
    /// * `storage_name` - the name of the storage to store/fetch data from.
    /// * `metric_id` - the metric category + name.
    ///
    /// # Returns
    ///
    /// * If the storage is unavailable an error is returned.
    /// * If the metric could not be deleted, an error is returned.
    ///
    /// Otherwise `Ok(())` is returned.
    ///
    /// # Panics
    ///
    /// This function will **not** panic on database errors.
    pub fn remove_single_metric(
        &self,
        lifetime: Lifetime,
        storage_name: &str,
        metric_id: &str,
    ) -> Result<()> {
        let clear_sql = "DELETE FROM telemetry WHERE lifetime = ?1 AND ping = ?2 AND id = ?3";
        let mut stmt = self.conn.prepare_cached(clear_sql)?;
        stmt.execute([lifetime.as_str(), storage_name, metric_id])?;
        Ok(())
    }

    /// Clears all the metrics in the database, for the provided lifetime.
    ///
    /// Errors are logged.
    ///
    /// # Panics
    ///
    /// * This function will **not** panic on database errors.
    pub fn clear_lifetime(&self, lifetime: Lifetime) {
        let clear_sql = "DELETE FROM telemetry WHERE lifetime = ?1";
        let mut stmt = unwrap_or!(self.conn.prepare_cached(clear_sql), return);
        let res = stmt.execute([lifetime.as_str()]);

        if let Err(e) = res {
            log::warn!("Could not clear store for lifetime {:?}: {:?}", lifetime, e);
        }
    }

    /// Clears all metrics in the database.
    ///
    /// Errors are logged.
    ///
    /// # Panics
    ///
    /// * This function will **not** panic on database errors.
    pub fn clear_all(&self) {
        for lifetime in [Lifetime::User, Lifetime::Ping, Lifetime::Application].iter() {
            self.clear_lifetime(*lifetime);
        }
    }

    /// Persists ping_lifetime_data to disk.
    ///
    /// Does nothing in case there is nothing to persist.
    ///
    /// # Panics
    ///
    /// * This function will **not** panic on database errors.
    pub fn persist_ping_lifetime_data(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test1)]
mod test {
    use super::*;
    use crate::tests::new_glean;
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[test]
    fn test_panicks_if_fails_dir_creation() {
        let path = Path::new("/!#\"'@#°ç");
        assert!(Database::new(path, false).is_err());
    }

    #[test]
    #[cfg(windows)]
    fn windows_invalid_utf16_panicfree() {
        use std::ffi::OsString;
        use std::os::windows::prelude::*;

        // Here the values 0x0066 and 0x006f correspond to 'f' and 'o'
        // respectively. The value 0xD800 is a lone surrogate half, invalid
        // in a UTF-16 sequence.
        let source = [0x0066, 0x006f, 0xD800, 0x006f];
        let os_string = OsString::from_wide(&source[..]);
        let os_str = os_string.as_os_str();
        let dir = tempdir().unwrap();
        let path = dir.path().join(os_str);

        let res = Database::new(&path, false);

        assert!(
            res.is_ok(),
            "Database should succeed at {}: {:?}",
            path.display(),
            res
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn linux_invalid_utf8_panicfree() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        // Here, the values 0x66 and 0x6f correspond to 'f' and 'o'
        // respectively. The value 0x80 is a lone continuation byte, invalid
        // in a UTF-8 sequence.
        let source = [0x66, 0x6f, 0x80, 0x6f];
        let os_str = OsStr::from_bytes(&source[..]);
        let dir = tempdir().unwrap();
        let path = dir.path().join(os_str);

        let res = Database::new(&path, false);
        assert!(
            res.is_ok(),
            "Database should not fail at {}: {:?}",
            path.display(),
            res
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn macos_invalid_utf8_panicfree() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        // Here, the values 0x66 and 0x6f correspond to 'f' and 'o'
        // respectively. The value 0x80 is a lone continuation byte, invalid
        // in a UTF-8 sequence.
        let source = [0x66, 0x6f, 0x80, 0x6f];
        let os_str = OsStr::from_bytes(&source[..]);
        let dir = tempdir().unwrap();
        let path = dir.path().join(os_str);

        let res = Database::new(&path, false);
        assert!(
            res.is_err(),
            "Database should not fail at {}: {:?}",
            path.display(),
            res
        );
    }

    #[test]
    fn test_data_dir_rkv_inits() {
        let dir = tempdir().unwrap();
        Database::new(dir.path(), false).unwrap();

        assert!(dir.path().exists());
    }

    #[test]
    fn test_ping_lifetime_metric_recorded() {
        // Init the database in a temporary directory.
        let dir = tempdir().unwrap();
        let db = Database::new(dir.path(), false).unwrap();

        assert!(db.ping_lifetime_data.is_none());

        // Attempt to record a known value.
        let test_value = "test-value";
        let test_storage = "test-storage";
        let test_metric_id = "telemetry_test.test_name";
        db.record_per_lifetime(
            Lifetime::Ping,
            test_storage,
            test_metric_id,
            &Metric::String(test_value.to_string()),
        )
        .unwrap();

        // Verify that the data is correctly recorded.
        let mut found_metrics = 0;
        let mut snapshotter = |metric_id: &[u8], metric: &Metric| {
            found_metrics += 1;
            let metric_id = String::from_utf8_lossy(metric_id).into_owned();
            assert_eq!(test_metric_id, metric_id);
            match metric {
                Metric::String(s) => assert_eq!(test_value, s),
                _ => panic!("Unexpected data found"),
            }
        };

        db.iter_store_from(Lifetime::Ping, test_storage, None, &mut snapshotter);
        assert_eq!(1, found_metrics, "We only expect 1 Lifetime.Ping metric.");
    }

    #[test]
    fn test_application_lifetime_metric_recorded() {
        // Init the database in a temporary directory.
        let dir = tempdir().unwrap();
        let db = Database::new(dir.path(), false).unwrap();

        // Attempt to record a known value.
        let test_value = "test-value";
        let test_storage = "test-storage1";
        let test_metric_id = "telemetry_test.test_name";
        db.record_per_lifetime(
            Lifetime::Application,
            test_storage,
            test_metric_id,
            &Metric::String(test_value.to_string()),
        )
        .unwrap();

        // Verify that the data is correctly recorded.
        let mut found_metrics = 0;
        let mut snapshotter = |metric_id: &[u8], metric: &Metric| {
            found_metrics += 1;
            let metric_id = String::from_utf8_lossy(metric_id).into_owned();
            assert_eq!(test_metric_id, metric_id);
            match metric {
                Metric::String(s) => assert_eq!(test_value, s),
                _ => panic!("Unexpected data found"),
            }
        };

        db.iter_store_from(Lifetime::Application, test_storage, None, &mut snapshotter);
        assert_eq!(
            1, found_metrics,
            "We only expect 1 Lifetime.Application metric."
        );
    }

    #[test]
    fn test_user_lifetime_metric_recorded() {
        // Init the database in a temporary directory.
        let dir = tempdir().unwrap();
        let db = Database::new(dir.path(), false).unwrap();

        // Attempt to record a known value.
        let test_value = "test-value";
        let test_storage = "test-storage2";
        let test_metric_id = "telemetry_test.test_name";
        db.record_per_lifetime(
            Lifetime::User,
            test_storage,
            test_metric_id,
            &Metric::String(test_value.to_string()),
        )
        .unwrap();

        // Verify that the data is correctly recorded.
        let mut found_metrics = 0;
        let mut snapshotter = |metric_id: &[u8], metric: &Metric| {
            found_metrics += 1;
            let metric_id = String::from_utf8_lossy(metric_id).into_owned();
            assert_eq!(test_metric_id, metric_id);
            match metric {
                Metric::String(s) => assert_eq!(test_value, s),
                _ => panic!("Unexpected data found"),
            }
        };

        db.iter_store_from(Lifetime::User, test_storage, None, &mut snapshotter);
        assert_eq!(1, found_metrics, "We only expect 1 Lifetime.User metric.");
    }

    #[test]
    fn test_clear_ping_storage() {
        // Init the database in a temporary directory.
        let dir = tempdir().unwrap();
        let db = Database::new(dir.path(), false).unwrap();

        // Attempt to record a known value for every single lifetime.
        let test_storage = "test-storage";
        db.record_per_lifetime(
            Lifetime::User,
            test_storage,
            "telemetry_test.test_name_user",
            &Metric::String("test-value-user".to_string()),
        )
        .unwrap();
        db.record_per_lifetime(
            Lifetime::Ping,
            test_storage,
            "telemetry_test.test_name_ping",
            &Metric::String("test-value-ping".to_string()),
        )
        .unwrap();
        db.record_per_lifetime(
            Lifetime::Application,
            test_storage,
            "telemetry_test.test_name_application",
            &Metric::String("test-value-application".to_string()),
        )
        .unwrap();

        // Take a snapshot for the data, all the lifetimes.
        {
            let mut snapshot: HashMap<String, String> = HashMap::new();
            let mut snapshotter = |metric_id: &[u8], metric: &Metric| {
                let metric_id = String::from_utf8_lossy(metric_id).into_owned();
                match metric {
                    Metric::String(s) => snapshot.insert(metric_id, s.to_string()),
                    _ => panic!("Unexpected data found"),
                };
            };

            db.iter_store_from(Lifetime::User, test_storage, None, &mut snapshotter);
            db.iter_store_from(Lifetime::Ping, test_storage, None, &mut snapshotter);
            db.iter_store_from(Lifetime::Application, test_storage, None, &mut snapshotter);

            assert_eq!(3, snapshot.len(), "We expect all lifetimes to be present.");
            assert!(snapshot.contains_key("telemetry_test.test_name_user"));
            assert!(snapshot.contains_key("telemetry_test.test_name_ping"));
            assert!(snapshot.contains_key("telemetry_test.test_name_application"));
        }

        // Clear the Ping lifetime.
        db.clear_ping_lifetime_storage(test_storage).unwrap();

        // Take a snapshot again and check that we're only clearing the Ping lifetime.
        {
            let mut snapshot: HashMap<String, String> = HashMap::new();
            let mut snapshotter = |metric_id: &[u8], metric: &Metric| {
                let metric_id = String::from_utf8_lossy(metric_id).into_owned();
                match metric {
                    Metric::String(s) => snapshot.insert(metric_id, s.to_string()),
                    _ => panic!("Unexpected data found"),
                };
            };

            db.iter_store_from(Lifetime::User, test_storage, None, &mut snapshotter);
            db.iter_store_from(Lifetime::Ping, test_storage, None, &mut snapshotter);
            db.iter_store_from(Lifetime::Application, test_storage, None, &mut snapshotter);

            assert_eq!(2, snapshot.len(), "We only expect 2 metrics to be left.");
            assert!(snapshot.contains_key("telemetry_test.test_name_user"));
            assert!(snapshot.contains_key("telemetry_test.test_name_application"));
        }
    }

    #[test]
    fn test_remove_single_metric() {
        // Init the database in a temporary directory.
        let dir = tempdir().unwrap();
        let db = Database::new(dir.path(), false).unwrap();

        let test_storage = "test-storage-single-lifetime";
        let metric_id_pattern = "telemetry_test.single_metric";

        // Write sample metrics to the database.
        let lifetimes = [Lifetime::User, Lifetime::Ping, Lifetime::Application];

        for lifetime in lifetimes.iter() {
            for value in &["retain", "delete"] {
                db.record_per_lifetime(
                    *lifetime,
                    test_storage,
                    &format!("{}_{}", metric_id_pattern, value),
                    &Metric::String((*value).to_string()),
                )
                .unwrap();
            }
        }

        // Remove "telemetry_test.single_metric_delete" from each lifetime.
        for lifetime in lifetimes.iter() {
            db.remove_single_metric(
                *lifetime,
                test_storage,
                &format!("{}_delete", metric_id_pattern),
            )
            .unwrap();
        }

        // Verify that "telemetry_test.single_metric_retain" is still around for all lifetimes.
        for lifetime in lifetimes.iter() {
            let mut found_metrics = 0;
            let mut snapshotter = |metric_id: &[u8], metric: &Metric| {
                found_metrics += 1;
                let metric_id = String::from_utf8_lossy(metric_id).into_owned();
                assert_eq!(format!("{}_retain", metric_id_pattern), metric_id);
                match metric {
                    Metric::String(s) => assert_eq!("retain", s),
                    _ => panic!("Unexpected data found"),
                }
            };

            // Check the User lifetime.
            db.iter_store_from(*lifetime, test_storage, None, &mut snapshotter);
            assert_eq!(
                1, found_metrics,
                "We only expect 1 metric for this lifetime."
            );
        }
    }

    #[test]
    fn test_delayed_ping_lifetime_persistence() {
        // Init the database in a temporary directory.
        let dir = tempdir().unwrap();
        let db = Database::new(dir.path(), true).unwrap();
        let test_storage = "test-storage";

        assert!(db.ping_lifetime_data.is_some());

        // Attempt to record a known value.
        let test_value1 = "test-value1";
        let test_metric_id1 = "telemetry_test.test_name1";
        db.record_per_lifetime(
            Lifetime::Ping,
            test_storage,
            test_metric_id1,
            &Metric::String(test_value1.to_string()),
        )
        .unwrap();

        // Attempt to persist data.
        db.persist_ping_lifetime_data().unwrap();

        // Attempt to record another known value.
        let test_value2 = "test-value2";
        let test_metric_id2 = "telemetry_test.test_name2";
        db.record_per_lifetime(
            Lifetime::Ping,
            test_storage,
            test_metric_id2,
            &Metric::String(test_value2.to_string()),
        )
        .unwrap();

        {
            // At this stage we expect `test_value1` to be persisted and in memory,
            // since it was recorded before calling `persist_ping_lifetime_data`,
            // and `test_value2` to be only in memory, since it was recorded after.
            let store: SingleStore = db
                .rkv
                .open_single(Lifetime::Ping.as_str(), StoreOptions::create())
                .unwrap();
            let reader = db.rkv.read().unwrap();

            // Verify that test_value1 is in rkv.
            assert!(store
                .get(&reader, format!("{}#{}", test_storage, test_metric_id1))
                .unwrap_or(None)
                .is_some());
            // Verifiy that test_value2 is **not** in rkv.
            assert!(store
                .get(&reader, format!("{}#{}", test_storage, test_metric_id2))
                .unwrap_or(None)
                .is_none());

            let data = match &db.ping_lifetime_data {
                Some(ping_lifetime_data) => ping_lifetime_data,
                None => panic!("Expected `ping_lifetime_data` to exist here!"),
            };
            let data = data.read().unwrap();
            // Verify that test_value1 is also in memory.
            assert!(data
                .get(&format!("{}#{}", test_storage, test_metric_id1))
                .is_some());
            // Verify that test_value2 is in memory.
            assert!(data
                .get(&format!("{}#{}", test_storage, test_metric_id2))
                .is_some());
        }

        // Attempt to persist data again.
        db.persist_ping_lifetime_data().unwrap();

        {
            // At this stage we expect `test_value1` and `test_value2` to
            // be persisted, since both were created before a call to
            // `persist_ping_lifetime_data`.
            let store: SingleStore = db
                .rkv
                .open_single(Lifetime::Ping.as_str(), StoreOptions::create())
                .unwrap();
            let reader = db.rkv.read().unwrap();

            // Verify that test_value1 is in rkv.
            assert!(store
                .get(&reader, format!("{}#{}", test_storage, test_metric_id1))
                .unwrap_or(None)
                .is_some());
            // Verifiy that test_value2 is also in rkv.
            assert!(store
                .get(&reader, format!("{}#{}", test_storage, test_metric_id2))
                .unwrap_or(None)
                .is_some());

            let data = match &db.ping_lifetime_data {
                Some(ping_lifetime_data) => ping_lifetime_data,
                None => panic!("Expected `ping_lifetime_data` to exist here!"),
            };
            let data = data.read().unwrap();
            // Verify that test_value1 is also in memory.
            assert!(data
                .get(&format!("{}#{}", test_storage, test_metric_id1))
                .is_some());
            // Verify that test_value2 is also in memory.
            assert!(data
                .get(&format!("{}#{}", test_storage, test_metric_id2))
                .is_some());
        }
    }

    #[test]
    fn test_load_ping_lifetime_data_from_memory() {
        // Init the database in a temporary directory.
        let dir = tempdir().unwrap();

        let test_storage = "test-storage";
        let test_value = "test-value";
        let test_metric_id = "telemetry_test.test_name";

        {
            let db = Database::new(dir.path(), true).unwrap();

            // Attempt to record a known value.
            db.record_per_lifetime(
                Lifetime::Ping,
                test_storage,
                test_metric_id,
                &Metric::String(test_value.to_string()),
            )
            .unwrap();

            // Verify that test_value is in memory.
            let data = match &db.ping_lifetime_data {
                Some(ping_lifetime_data) => ping_lifetime_data,
                None => panic!("Expected `ping_lifetime_data` to exist here!"),
            };
            let data = data.read().unwrap();
            assert!(data
                .get(&format!("{}#{}", test_storage, test_metric_id))
                .is_some());

            // Attempt to persist data.
            db.persist_ping_lifetime_data().unwrap();

            // Verify that test_value is now in rkv.
            let store: SingleStore = db
                .rkv
                .open_single(Lifetime::Ping.as_str(), StoreOptions::create())
                .unwrap();
            let reader = db.rkv.read().unwrap();
            assert!(store
                .get(&reader, format!("{}#{}", test_storage, test_metric_id))
                .unwrap_or(None)
                .is_some());
        }

        // Now create a new instace of the db and check if data was
        // correctly loaded from rkv to memory.
        {
            let db = Database::new(dir.path(), true).unwrap();

            // Verify that test_value is in memory.
            let data = match &db.ping_lifetime_data {
                Some(ping_lifetime_data) => ping_lifetime_data,
                None => panic!("Expected `ping_lifetime_data` to exist here!"),
            };
            let data = data.read().unwrap();
            assert!(data
                .get(&format!("{}#{}", test_storage, test_metric_id))
                .is_some());

            // Verify that test_value is also in rkv.
            let store: SingleStore = db
                .rkv
                .open_single(Lifetime::Ping.as_str(), StoreOptions::create())
                .unwrap();
            let reader = db.rkv.read().unwrap();
            assert!(store
                .get(&reader, format!("{}#{}", test_storage, test_metric_id))
                .unwrap_or(None)
                .is_some());
        }
    }

    #[test]
    fn doesnt_record_when_upload_is_disabled() {
        let (mut glean, dir) = new_glean(None);

        // Init the database in a temporary directory.

        let test_storage = "test-storage";
        let test_data = CommonMetricDataInternal::new("category", "name", test_storage);
        let test_metric_id = test_data.identifier(&glean);

        // Attempt to record metric with the record and record_with functions,
        // this should work since upload is enabled.
        let db = Database::new(dir.path(), true).unwrap();
        db.record(&glean, &test_data, &Metric::String("record".to_owned()));
        db.iter_store_from(
            Lifetime::Ping,
            test_storage,
            None,
            &mut |metric_id: &[u8], metric: &Metric| {
                assert_eq!(
                    String::from_utf8_lossy(metric_id).into_owned(),
                    test_metric_id
                );
                match metric {
                    Metric::String(v) => assert_eq!("record", *v),
                    _ => panic!("Unexpected data found"),
                }
            },
        );

        db.record_with(&glean, &test_data, |_| {
            Metric::String("record_with".to_owned())
        });
        db.iter_store_from(
            Lifetime::Ping,
            test_storage,
            None,
            &mut |metric_id: &[u8], metric: &Metric| {
                assert_eq!(
                    String::from_utf8_lossy(metric_id).into_owned(),
                    test_metric_id
                );
                match metric {
                    Metric::String(v) => assert_eq!("record_with", *v),
                    _ => panic!("Unexpected data found"),
                }
            },
        );

        // Disable upload
        glean.set_upload_enabled(false);

        // Attempt to record metric with the record and record_with functions,
        // this should work since upload is now **disabled**.
        db.record(&glean, &test_data, &Metric::String("record_nop".to_owned()));
        db.iter_store_from(
            Lifetime::Ping,
            test_storage,
            None,
            &mut |metric_id: &[u8], metric: &Metric| {
                assert_eq!(
                    String::from_utf8_lossy(metric_id).into_owned(),
                    test_metric_id
                );
                match metric {
                    Metric::String(v) => assert_eq!("record_with", *v),
                    _ => panic!("Unexpected data found"),
                }
            },
        );
        db.record_with(&glean, &test_data, |_| {
            Metric::String("record_with_nop".to_owned())
        });
        db.iter_store_from(
            Lifetime::Ping,
            test_storage,
            None,
            &mut |metric_id: &[u8], metric: &Metric| {
                assert_eq!(
                    String::from_utf8_lossy(metric_id).into_owned(),
                    test_metric_id
                );
                match metric {
                    Metric::String(v) => assert_eq!("record_with", *v),
                    _ => panic!("Unexpected data found"),
                }
            },
        );
    }

    mod safe_mode {
        use std::fs::File;

        use super::*;
        use rkv::Value;

        #[test]
        fn empty_data_file() {
            let dir = tempdir().unwrap();

            // Create database directory structure.
            let database_dir = dir.path().join("db");
            fs::create_dir_all(&database_dir).expect("create database dir");

            // Create empty database file.
            let safebin = database_dir.join("data.safe.bin");
            let f = File::create(safebin).expect("create database file");
            drop(f);

            let db = Database::new(dir.path(), false).unwrap();

            assert!(dir.path().exists());
            assert!(
                matches!(db.rkv_load_state, RkvLoadState::Err(_)),
                "Load error recorded"
            );
        }

        #[test]
        fn corrupted_data_file() {
            let dir = tempdir().unwrap();

            // Create database directory structure.
            let database_dir = dir.path().join("db");
            fs::create_dir_all(&database_dir).expect("create database dir");

            // Create empty database file.
            let safebin = database_dir.join("data.safe.bin");
            fs::write(safebin, "<broken>").expect("write to database file");

            let db = Database::new(dir.path(), false).unwrap();

            assert!(dir.path().exists());
            assert!(
                matches!(db.rkv_load_state, RkvLoadState::Err(_)),
                "Load error recorded"
            );
        }

        #[test]
        fn migration_works_on_startup() {
            let dir = tempdir().unwrap();

            let database_dir = dir.path().join("db");
            let datamdb = database_dir.join("data.mdb");
            let lockmdb = database_dir.join("lock.mdb");
            let safebin = database_dir.join("data.safe.bin");

            assert!(!safebin.exists());
            assert!(!datamdb.exists());
            assert!(!lockmdb.exists());

            let store_name = "store1";
            let metric_name = "bool";
            let key = Database::get_storage_key(store_name, Some(metric_name));

            // Ensure some old data in the LMDB format exists.
            {
                fs::create_dir_all(&database_dir).expect("create dir");
                let rkv_db = rkv::Rkv::new::<rkv::backend::Lmdb>(&database_dir).expect("rkv env");

                let store = rkv_db
                    .open_single("ping", StoreOptions::create())
                    .expect("opened");
                let mut writer = rkv_db.write().expect("writer");
                let metric = Metric::Boolean(true);
                let value = bincode::serialize(&metric).expect("serialized");
                store
                    .put(&mut writer, &key, &Value::Blob(&value))
                    .expect("wrote");
                writer.commit().expect("committed");

                assert!(datamdb.exists());
                assert!(lockmdb.exists());
                assert!(!safebin.exists());
            }

            // First open should migrate the data.
            {
                let db = Database::new(dir.path(), false).unwrap();
                let safebin = database_dir.join("data.safe.bin");
                assert!(safebin.exists(), "safe-mode file should exist");
                assert!(!datamdb.exists(), "LMDB data should be deleted");
                assert!(!lockmdb.exists(), "LMDB lock should be deleted");

                let mut stored_metrics = vec![];
                let mut snapshotter = |name: &[u8], metric: &Metric| {
                    let name = str::from_utf8(name).unwrap().to_string();
                    stored_metrics.push((name, metric.clone()))
                };
                db.iter_store_from(Lifetime::Ping, "store1", None, &mut snapshotter);

                assert_eq!(1, stored_metrics.len());
                assert_eq!(metric_name, stored_metrics[0].0);
                assert_eq!(&Metric::Boolean(true), &stored_metrics[0].1);
            }

            // Next open should not re-create the LMDB files.
            {
                let db = Database::new(dir.path(), false).unwrap();
                let safebin = database_dir.join("data.safe.bin");
                assert!(safebin.exists(), "safe-mode file exists");
                assert!(!datamdb.exists(), "LMDB data should not be recreated");
                assert!(!lockmdb.exists(), "LMDB lock should not be recreated");

                let mut stored_metrics = vec![];
                let mut snapshotter = |name: &[u8], metric: &Metric| {
                    let name = str::from_utf8(name).unwrap().to_string();
                    stored_metrics.push((name, metric.clone()))
                };
                db.iter_store_from(Lifetime::Ping, "store1", None, &mut snapshotter);

                assert_eq!(1, stored_metrics.len());
                assert_eq!(metric_name, stored_metrics[0].0);
                assert_eq!(&Metric::Boolean(true), &stored_metrics[0].1);
            }
        }

        #[test]
        fn migration_doesnt_overwrite() {
            let dir = tempdir().unwrap();

            let database_dir = dir.path().join("db");
            let datamdb = database_dir.join("data.mdb");
            let lockmdb = database_dir.join("lock.mdb");
            let safebin = database_dir.join("data.safe.bin");

            assert!(!safebin.exists());
            assert!(!datamdb.exists());
            assert!(!lockmdb.exists());

            let store_name = "store1";
            let metric_name = "counter";
            let key = Database::get_storage_key(store_name, Some(metric_name));

            // Ensure some old data in the LMDB format exists.
            {
                fs::create_dir_all(&database_dir).expect("create dir");
                let rkv_db = rkv::Rkv::new::<rkv::backend::Lmdb>(&database_dir).expect("rkv env");

                let store = rkv_db
                    .open_single("ping", StoreOptions::create())
                    .expect("opened");
                let mut writer = rkv_db.write().expect("writer");
                let metric = Metric::Counter(734); // this value will be ignored
                let value = bincode::serialize(&metric).expect("serialized");
                store
                    .put(&mut writer, &key, &Value::Blob(&value))
                    .expect("wrote");
                writer.commit().expect("committed");

                assert!(datamdb.exists());
                assert!(lockmdb.exists());
            }

            // Ensure some data exists in the new database.
            {
                fs::create_dir_all(&database_dir).expect("create dir");
                let rkv_db =
                    rkv::Rkv::new::<rkv::backend::SafeMode>(&database_dir).expect("rkv env");

                let store = rkv_db
                    .open_single("ping", StoreOptions::create())
                    .expect("opened");
                let mut writer = rkv_db.write().expect("writer");
                let metric = Metric::Counter(2);
                let value = bincode::serialize(&metric).expect("serialized");
                store
                    .put(&mut writer, &key, &Value::Blob(&value))
                    .expect("wrote");
                writer.commit().expect("committed");

                assert!(safebin.exists());
            }

            // First open should try migration and ignore it, because destination is not empty.
            // It also deletes the leftover LMDB database.
            {
                let db = Database::new(dir.path(), false).unwrap();
                let safebin = database_dir.join("data.safe.bin");
                assert!(safebin.exists(), "safe-mode file should exist");
                assert!(!datamdb.exists(), "LMDB data should be deleted");
                assert!(!lockmdb.exists(), "LMDB lock should be deleted");

                let mut stored_metrics = vec![];
                let mut snapshotter = |name: &[u8], metric: &Metric| {
                    let name = str::from_utf8(name).unwrap().to_string();
                    stored_metrics.push((name, metric.clone()))
                };
                db.iter_store_from(Lifetime::Ping, "store1", None, &mut snapshotter);

                assert_eq!(1, stored_metrics.len());
                assert_eq!(metric_name, stored_metrics[0].0);
                assert_eq!(&Metric::Counter(2), &stored_metrics[0].1);
            }
        }

        #[test]
        fn migration_ignores_broken_database() {
            let dir = tempdir().unwrap();

            let database_dir = dir.path().join("db");
            let datamdb = database_dir.join("data.mdb");
            let lockmdb = database_dir.join("lock.mdb");
            let safebin = database_dir.join("data.safe.bin");

            assert!(!safebin.exists());
            assert!(!datamdb.exists());
            assert!(!lockmdb.exists());

            let store_name = "store1";
            let metric_name = "counter";
            let key = Database::get_storage_key(store_name, Some(metric_name));

            // Ensure some old data in the LMDB format exists.
            {
                fs::create_dir_all(&database_dir).expect("create dir");
                fs::write(&datamdb, "bogus").expect("dbfile created");

                assert!(datamdb.exists());
            }

            // Ensure some data exists in the new database.
            {
                fs::create_dir_all(&database_dir).expect("create dir");
                let rkv_db =
                    rkv::Rkv::new::<rkv::backend::SafeMode>(&database_dir).expect("rkv env");

                let store = rkv_db
                    .open_single("ping", StoreOptions::create())
                    .expect("opened");
                let mut writer = rkv_db.write().expect("writer");
                let metric = Metric::Counter(2);
                let value = bincode::serialize(&metric).expect("serialized");
                store
                    .put(&mut writer, &key, &Value::Blob(&value))
                    .expect("wrote");
                writer.commit().expect("committed");
            }

            // First open should try migration and ignore it, because destination is not empty.
            // It also deletes the leftover LMDB database.
            {
                let db = Database::new(dir.path(), false).unwrap();
                let safebin = database_dir.join("data.safe.bin");
                assert!(safebin.exists(), "safe-mode file should exist");
                assert!(!datamdb.exists(), "LMDB data should be deleted");
                assert!(!lockmdb.exists(), "LMDB lock should be deleted");

                let mut stored_metrics = vec![];
                let mut snapshotter = |name: &[u8], metric: &Metric| {
                    let name = str::from_utf8(name).unwrap().to_string();
                    stored_metrics.push((name, metric.clone()))
                };
                db.iter_store_from(Lifetime::Ping, "store1", None, &mut snapshotter);

                assert_eq!(1, stored_metrics.len());
                assert_eq!(metric_name, stored_metrics[0].0);
                assert_eq!(&Metric::Counter(2), &stored_metrics[0].1);
            }
        }

        #[test]
        fn migration_ignores_empty_database() {
            let dir = tempdir().unwrap();

            let database_dir = dir.path().join("db");
            let datamdb = database_dir.join("data.mdb");
            let lockmdb = database_dir.join("lock.mdb");
            let safebin = database_dir.join("data.safe.bin");

            assert!(!safebin.exists());
            assert!(!datamdb.exists());
            assert!(!lockmdb.exists());

            // Ensure old LMDB database exists, but is empty.
            {
                fs::create_dir_all(&database_dir).expect("create dir");
                let rkv_db = rkv::Rkv::new::<rkv::backend::Lmdb>(&database_dir).expect("rkv env");
                drop(rkv_db);
                assert!(datamdb.exists());
                assert!(lockmdb.exists());
            }

            // First open should try migration, but find no data.
            // safe-mode does not write an empty database to disk.
            // It also deletes the leftover LMDB database.
            {
                let _db = Database::new(dir.path(), false).unwrap();
                let safebin = database_dir.join("data.safe.bin");
                assert!(!safebin.exists(), "safe-mode file should exist");
                assert!(!datamdb.exists(), "LMDB data should be deleted");
                assert!(!lockmdb.exists(), "LMDB lock should be deleted");
            }
        }
    }
}
