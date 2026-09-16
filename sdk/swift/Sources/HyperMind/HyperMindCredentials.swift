import Foundation

/// Identity and sizing material for one embedded actor. The hexadecimal fields
/// are sealed-identity secrets, so neither string reaches a description.
public struct HyperMindCredentials: Sendable {
    /// Nonzero actor identifier. The actor owns `<stateDirectory>/<actor>`.
    public var actor: UInt16
    /// Thirty-two hexadecimal characters naming the user identity.
    public var userHex: String
    /// Sixty-four hexadecimal characters carrying the key encryption key.
    public var kekHex: String
    /// Projection map size in bytes.
    public var projectionMapBytes: Int
    /// Whether provider selection reads the process environment.
    public var providersFromEnvironment: Bool

    public init(
        actor: UInt16,
        userHex: String,
        kekHex: String,
        projectionMapBytes: Int = 268_435_456,
        providersFromEnvironment: Bool = false
    ) {
        self.actor = actor
        self.userHex = userHex
        self.kekHex = kekHex
        self.projectionMapBytes = projectionMapBytes
        self.providersFromEnvironment = providersFromEnvironment
    }

    /// Renders the configuration object the C boundary accepts. The boundary
    /// rejects unknown keys, so every key below is one the header documents.
    public func configurationJSON(stateDirectory: String) -> String {
        let path = Self.quoted(stateDirectory)
        let user = Self.quoted(userHex)
        let kek = Self.quoted(kekHex)
        return "{\"path\":\(path),\"actor\":\(actor),\"user_hex\":\(user),\"kek_hex\":\(kek),"
            + "\"projection_map_bytes\":\(projectionMapBytes),"
            + "\"providers_from_environment\":\(providersFromEnvironment)}"
    }

    private static func quoted(_ value: String) -> String {
        var rendered = "\""
        for character in value.unicodeScalars {
            switch character {
            case "\"": rendered += "\\\""
            case "\\": rendered += "\\\\"
            case "\n": rendered += "\\n"
            case "\r": rendered += "\\r"
            case "\t": rendered += "\\t"
            default:
                if character.value < 0x20 {
                    rendered += String(format: "\\u%04x", character.value)
                } else {
                    rendered.unicodeScalars.append(character)
                }
            }
        }
        return rendered + "\""
    }
}

extension HyperMindCredentials: CustomStringConvertible, CustomDebugStringConvertible {
    public var description: String {
        "HyperMindCredentials(actor: \(actor), userHex: <redacted>, kekHex: <redacted>, "
            + "projectionMapBytes: \(projectionMapBytes), "
            + "providersFromEnvironment: \(providersFromEnvironment))"
    }

    public var debugDescription: String { description }
}
