/* This Source Code Form is subject to the terms of the Mozilla Public
* License, v. 2.0. If a copy of the MPL was not distributed with this
* file, You can obtain one at http://mozilla.org/MPL/2.0/. */

package mozilla.telemetry.glean.private

import androidx.annotation.VisibleForTesting
import mozilla.telemetry.glean.internal.PingType as GleanPingType

/**
 * A ping's reason codes.
 *
 * Reason codes are expressed as an enum
 * and can be converted back to their ordinal representation,
 * which also maps to their string representation.
 *
 * This is automatically implemented for generated enums.
 */
interface ReasonCode {
    fun code(): Int {
        error("can't determine reason code")
    }
}

/**
 * An enum with no values for convenient use as the default set of reason codes.
 */
@Suppress("EmptyClassBlock")
enum class NoReasonCodes(
    /**
     * @suppress
     */
    val value: Int,
) : ReasonCode {
    // deliberately empty
}

/**
 * This implements the developer facing API for custom pings.
 *
 * Instances of this class type are automatically generated by the parsers at build time.
 *
 * The Ping API only exposes the [send] method, which schedules a ping for sending.
 *
 * @property reasonCodes The list of acceptable reason codes for this ping.
 */
class PingType<ReasonCodesEnum> (
    name: String,
    includeClientId: Boolean,
    sendIfEmpty: Boolean,
    preciseTimestamps: Boolean,
    includeInfoSections: Boolean,
    val reasonCodes: List<String>,
) where ReasonCodesEnum : Enum<ReasonCodesEnum>, ReasonCodesEnum : ReasonCode {
    private var testCallback: ((ReasonCodesEnum?) -> Unit)? = null
    private val innerPing: GleanPingType

    init {
        this.innerPing = GleanPingType(
            name = name,
            includeClientId = includeClientId,
            sendIfEmpty = sendIfEmpty,
            preciseTimestamps = preciseTimestamps,
            includeInfoSections = includeInfoSections,
            reasonCodes = reasonCodes,
        )
    }

    /**
     * **Test-only API**
     *
     * Attach a callback to be called right before a new ping is submitted.
     * The provided function is called exactly once before submitting a ping.
     *
     * Note: The callback will be called on any call to submit.
     * A ping might not be sent afterwards, e.g. if the ping is otherwise empty (and
     * `send_if_empty` is `false`).
     */
    @VisibleForTesting(otherwise = VisibleForTesting.NONE)
    @Synchronized
    fun testBeforeNextSubmit(cb: (ReasonCodesEnum?) -> Unit) {
        this.testCallback = cb
    }

    /**
     * Collect and submit the ping for eventual upload.
     *
     * While the collection of metrics into pings happens synchronously, the
     * ping queuing and ping uploading happens asyncronously.
     * There are no guarantees that this will happen immediately.
     *
     * If the ping currently contains no content, it will not be queued.
     *
     * @param reason The reason the ping is being submitted.
     */
    @JvmOverloads
    fun submit(reason: ReasonCodesEnum? = null) {
        this.testCallback?.let {
            it(reason)
        }
        this.testCallback = null

        val reasonString = reason?.let { this.reasonCodes[it.code()] }
        this.innerPing.submit(reasonString)
    }
}
