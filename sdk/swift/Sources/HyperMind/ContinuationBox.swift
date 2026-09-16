import Foundation

/// Gives one `CheckedContinuation` a stable heap address so it can round-trip
/// through the C `void *` user-data pointer. The box is retained before the
/// call is issued and released exactly once, either inside the callback or on
/// the synchronous refusal path, never both.
final class ContinuationBox: @unchecked Sendable {
    private let continuation: CheckedContinuation<String, Error>

    init(_ continuation: CheckedContinuation<String, Error>) {
        self.continuation = continuation
    }

    func resume(returning envelope: String) {
        continuation.resume(returning: envelope)
    }

    func resume(throwing error: Error) {
        continuation.resume(throwing: error)
    }
}
