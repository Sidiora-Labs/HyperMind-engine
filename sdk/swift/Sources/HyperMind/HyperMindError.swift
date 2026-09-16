import CHyperMind
import Foundation

/// A failure reported across the C boundary.
///
/// `status` is the boundary tier the header declares. `kernelCode` carries the
/// numeric kernel error discriminant and is `nil` for every status other than
/// `HM_STATUS_KERNEL`. `message` is the stable kernel error name on a kernel
/// failure and the recorded boundary reason otherwise.
public struct HyperMindError: Error, CustomStringConvertible {
    public let status: HmStatus
    public let kernelCode: Int32?
    public let message: String

    public init(status: HmStatus, kernelCode: Int32?, message: String) {
        self.status = status
        self.kernelCode = kernelCode
        self.message = message
    }

    /// Builds an error for a synchronous refusal by copying the reason the
    /// library recorded on this thread. The borrowed pointer is copied before
    /// any further entry point can overwrite it.
    public static func fromLastError(_ status: HmStatus) -> HyperMindError {
        let recorded = hm_last_error_message().map { String(cString: $0) }
        return HyperMindError(
            status: status,
            kernelCode: nil,
            message: recorded ?? "the boundary refused the call without recording a reason"
        )
    }

    public var description: String {
        let code = kernelCode.map { ", kernelCode: \($0)" } ?? ""
        return "HyperMindError(status: \(status.rawValue)\(code), message: \(message))"
    }
}
