/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

import Foundation
import os.log

class Logger {
    private let log: OSLog

    init(tag: String) {
        self.log = OSLog(
            subsystem: Bundle.main.bundleIdentifier!,
            category: tag
        )
    }

    internal func info(_ msg: String) {
        log(msg, type: .info)
    }

    internal func debug(_ msg: String) {
        log(msg, type: .debug)
    }

    internal func error(_ msg: String) {
        log(msg, type: .error)
    }

    private func log(_ msg: String, type: OSLogType) {
        os_log("%@", log: self.log, type: type, msg)
    }
}
