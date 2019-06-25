/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
* file, You can obtain one at http://mozilla.org/MPL/2.0/. */

package mozilla.telemetry.glean.private

import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.ObsoleteCoroutinesApi
import mozilla.telemetry.glean.resetGlean
// import mozilla.components.service.glean.timing.TimingManager
import org.junit.After
// import org.junit.Assert.assertEquals
// import org.junit.Assert.assertTrue
// import org.junit.Assert.assertFalse
import org.junit.Before
import org.junit.Ignore
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import java.lang.NullPointerException

@ObsoleteCoroutinesApi
@ExperimentalCoroutinesApi
@RunWith(RobolectricTestRunner::class)
class TimingDistributionMetricTypeTest {

    @Before
    fun setUp() {
        resetGlean()
    }

    @After
    fun reset() {
        // TimingManager.testResetTimeSource()
    }

    @Ignore("TimingManager is not implemented yet. See bug 1554689")
    @Test
    fun `The API saves to its storage engine`() {
        /*
        // Define a timing distribution metric which will be stored in "store1"
        val metric = TimingDistributionMetricType(
            disabled = false,
            category = "telemetry",
            lifetime = Lifetime.Ping,
            name = "timing_distribution",
            sendInPings = listOf("store1"),
            timeUnit = TimeUnit.Millisecond
        )

        // Accumulate a few values
        for (i in 1L..3L) {
            TimingManager.getElapsedNanos = { 0 }
            metric.start(this)
            TimingManager.getElapsedNanos = { i * 1000000 } // ms to ns
            metric.stopAndAccumulate(this)
        }

        // Check that data was properly recorded.
        assertTrue(metric.testHasValue())
        val snapshot = metric.testGetValue()
        // Check the sum
        assertEquals(6L, snapshot.sum)
        // Check that the 1L fell into the first bucket
        assertEquals(1L, snapshot.values[0])
        // Check that the 2L fell into the second bucket
        assertEquals(1L, snapshot.values[1])
        // Check that the 3L fell into the third bucket
        assertEquals(1L, snapshot.values[2])
        */
    }

    @Ignore("TimingManager is not implemented yet. See bug 1554689")
    @Test
    fun `disabled timing distributions must not record data`() {
        /*
        // Define a timing distribution metric which will be stored in "store1"
        // It's lifetime is set to Lifetime.Ping so it should not record anything.
        val metric = TimingDistributionMetricType(
            disabled = true,
            category = "telemetry",
            lifetime = Lifetime.Ping,
            name = "timing_distribution",
            sendInPings = listOf("store1"),
            timeUnit = TimeUnit.Millisecond
        )

        // Attempt to store the timespan using set
        TimingManager.getElapsedNanos = { 0 }
        metric.start(this)
        TimingManager.getElapsedNanos = { 1 }
        metric.stopAndAccumulate(this)

        // Check that nothing was recorded.
        assertFalse("TimingDistributions without a lifetime should not record data.",
            metric.testHasValue())
        */
    }

    @Ignore("The testing API is not implemented.")
    @Test(expected = NullPointerException::class)
    fun `testGetValue() throws NullPointerException if nothing is stored`() {
        // Define a timing distribution metric which will be stored in "store1"
        val metric = TimingDistributionMetricType(
            disabled = false,
            category = "telemetry",
            lifetime = Lifetime.Ping,
            name = "timing_distribution",
            sendInPings = listOf("store1"),
            timeUnit = TimeUnit.Millisecond
        )
        metric.testGetValue()
    }

    @Ignore("TimingManager is not implemented yet. See bug 1554689")
    @Test
    fun `The API saves to secondary pings`() {
        /*
        // Define a timing distribution metric which will be stored in multiple stores
        val metric = TimingDistributionMetricType(
            disabled = false,
            category = "telemetry",
            lifetime = Lifetime.Ping,
            name = "timing_distribution",
            sendInPings = listOf("store1", "store2", "store3"),
            timeUnit = TimeUnit.Millisecond
        )

        // Accumulate a few values
        for (i in 1L..3L) {
            TimingManager.getElapsedNanos = { 0 }
            metric.start(this)
            TimingManager.getElapsedNanos = { i * 1000000 } // ms to ns
            metric.stopAndAccumulate(this)
        }

        // Check that data was properly recorded in the second ping.
        assertTrue(metric.testHasValue("store2"))
        val snapshot = metric.testGetValue("store2")
        // Check the sum
        assertEquals(6L, snapshot.sum)
        // Check that the 1L fell into the first bucket
        assertEquals(1L, snapshot.values[0])
        // Check that the 2L fell into the second bucket
        assertEquals(1L, snapshot.values[1])
        // Check that the 3L fell into the third bucket
        assertEquals(1L, snapshot.values[2])

        // Check that data was properly recorded in the third ping.
        assertTrue(metric.testHasValue("store3"))
        val snapshot2 = metric.testGetValue("store3")
        // Check the sum
        assertEquals(6L, snapshot2.sum)
        // Check that the 1L fell into the first bucket
        assertEquals(1L, snapshot2.values[0])
        // Check that the 2L fell into the second bucket
        assertEquals(1L, snapshot2.values[1])
        // Check that the 3L fell into the third bucket
        assertEquals(1L, snapshot2.values[2])
        */
    }
}