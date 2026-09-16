import CHyperMind
import Foundation

private let resultCallback: @convention(c) (
    HmStatus, Int32, UnsafePointer<CChar>?, UnsafePointer<CChar>?, UnsafeMutableRawPointer?
) -> Void = { status, kernelCode, resultJSON, errorMessage, userData in
    guard let userData else { return }
    let box = Unmanaged<ContinuationBox>.fromOpaque(userData).takeRetainedValue()
    if status == HM_STATUS_OK, let resultJSON {
        box.resume(returning: String(cString: resultJSON))
        return
    }
    let message = errorMessage.map { String(cString: $0) }
        ?? "the call failed without reporting a message"
    let code = status == HM_STATUS_KERNEL ? kernelCode : nil
    box.resume(throwing: HyperMindError(status: status, kernelCode: code, message: message))
}

/// An embedded HyperMind actor reached through the C application binary
/// interface.
///
/// Ownership is explicit on both sides. The instance owns exactly one handle
/// and frees it exactly once in `deinit`. Shutting the actor down is the
/// separate, idempotent `close()`. Every result string is copied inside the C
/// callback before that callback returns, so nothing borrowed outlives it.
///
/// Cancellation is cooperative at the call boundary only: it is checked before
/// a call is issued and again after it resolves. The kernel round trip itself
/// is not interruptible, so cancelling a task in flight does not stop the work
/// already handed to the actor.
public final class HyperMindEngine: @unchecked Sendable {
    private let handle: OpaquePointer

    /// Opens the actor named by `credentials` under `stateDirectory`.
    public init(credentials: HyperMindCredentials, stateDirectory: String) throws {
        let configuration = credentials.configurationJSON(stateDirectory: stateDirectory)
        var opened: OpaquePointer?
        let status = configuration.withCString { hm_engine_open($0, &opened) }
        guard status == HM_STATUS_OK, let opened else {
            throw HyperMindError.fromLastError(status)
        }
        handle = opened
    }

    deinit {
        hm_engine_free(handle)
    }

    /// Shuts the actor down and releases its state directory. The call blocks
    /// and is idempotent; freeing the handle remains this instance's own job.
    public func close() throws {
        let status = hm_engine_close(handle)
        guard status == HM_STATUS_OK else {
            throw HyperMindError.fromLastError(status)
        }
    }

    /// Runs one verb and returns the serialized envelope.
    public func call(verb: String, argumentsJSON: String) async throws -> String {
        try Task.checkCancellation()
        let envelope = try await invoke(verb, argumentsJSON)
        try Task.checkCancellation()
        return envelope
    }

    private func invoke(_ verb: String, _ argumentsJSON: String) async throws -> String {
        try await withCheckedThrowingContinuation { continuation in
            let box = ContinuationBox(continuation)
            let userData = Unmanaged.passRetained(box).toOpaque()
            let status = verb.withCString { verbPointer in
                argumentsJSON.withCString { argumentsPointer in
                    hm_engine_call(handle, verbPointer, argumentsPointer, resultCallback, userData)
                }
            }
            guard status == HM_STATUS_OK else {
                Unmanaged<ContinuationBox>.fromOpaque(userData).release()
                box.resume(throwing: HyperMindError.fromLastError(status))
                return
            }
        }
    }

    public func remember(argumentsJSON: String) async throws -> String {
        try await call(verb: "remember", argumentsJSON: argumentsJSON)
    }

    public func recall(argumentsJSON: String) async throws -> String {
        try await call(verb: "recall", argumentsJSON: argumentsJSON)
    }

    public func activate(argumentsJSON: String) async throws -> String {
        try await call(verb: "activate", argumentsJSON: argumentsJSON)
    }

    public func believe(argumentsJSON: String) async throws -> String {
        try await call(verb: "believe", argumentsJSON: argumentsJSON)
    }

    public func retract(argumentsJSON: String) async throws -> String {
        try await call(verb: "retract", argumentsJSON: argumentsJSON)
    }

    public func dispute(argumentsJSON: String) async throws -> String {
        try await call(verb: "dispute", argumentsJSON: argumentsJSON)
    }

    public func intend(argumentsJSON: String) async throws -> String {
        try await call(verb: "intend", argumentsJSON: argumentsJSON)
    }

    public func bind(argumentsJSON: String) async throws -> String {
        try await call(verb: "bind", argumentsJSON: argumentsJSON)
    }

    public func predict(argumentsJSON: String) async throws -> String {
        try await call(verb: "predict", argumentsJSON: argumentsJSON)
    }

    public func outcome(argumentsJSON: String) async throws -> String {
        try await call(verb: "outcome", argumentsJSON: argumentsJSON)
    }

    public func attest(argumentsJSON: String) async throws -> String {
        try await call(verb: "attest", argumentsJSON: argumentsJSON)
    }

    public func consolidate(argumentsJSON: String) async throws -> String {
        try await call(verb: "consolidate", argumentsJSON: argumentsJSON)
    }

    public func inspect(argumentsJSON: String) async throws -> String {
        try await call(verb: "inspect", argumentsJSON: argumentsJSON)
    }

    public func forget(argumentsJSON: String) async throws -> String {
        try await call(verb: "forget", argumentsJSON: argumentsJSON)
    }
}
