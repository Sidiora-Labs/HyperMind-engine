import Foundation

enum ContextSHA256 {
    private static let constants: [UInt32] = [
        0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
        0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
        0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
        0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
        0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
        0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
        0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
        0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2
    ]
    private static func rotate(_ value: UInt32, _ bits: UInt32) -> UInt32 { (value >> bits) | (value << (32 - bits)) }
    static func hex(_ input: String) -> String {
        var bytes = Array(input.utf8)
        let length = UInt64(bytes.count) * 8
        bytes.append(0x80)
        while bytes.count % 64 != 56 { bytes.append(0) }
        for shift in stride(from: 56, through: 0, by: -8) { bytes.append(UInt8(truncatingIfNeeded: length >> UInt64(shift))) }
        var hash: [UInt32] = [0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19]
        for offset in stride(from: 0, to: bytes.count, by: 64) {
            var words = [UInt32](repeating: 0, count: 64)
            for i in 0..<16 { let j = offset + i * 4; words[i] = UInt32(bytes[j]) << 24 | UInt32(bytes[j+1]) << 16 | UInt32(bytes[j+2]) << 8 | UInt32(bytes[j+3]) }
            for i in 16..<64 {
                let a = words[i-15], b = words[i-2]
                let small0 = rotate(a,7) ^ rotate(a,18) ^ (a >> 3)
                let small1 = rotate(b,17) ^ rotate(b,19) ^ (b >> 10)
                words[i] = words[i-16] &+ small0 &+ words[i-7] &+ small1
            }
            var a=hash[0], b=hash[1], c=hash[2], d=hash[3], e=hash[4], f=hash[5], g=hash[6], h=hash[7]
            for i in 0..<64 {
                let big1 = rotate(e,6) ^ rotate(e,11) ^ rotate(e,25)
                let choose = (e & f) ^ ((~e) & g)
                let t1 = h &+ big1 &+ choose &+ constants[i] &+ words[i]
                let big0 = rotate(a,2) ^ rotate(a,13) ^ rotate(a,22)
                let majority = (a & b) ^ (a & c) ^ (b & c)
                let t2 = big0 &+ majority
                h=g; g=f; f=e; e=d &+ t1; d=c; c=b; b=a; a=t1 &+ t2
            }
            let final = [a,b,c,d,e,f,g,h]
            for i in 0..<8 { hash[i] = hash[i] &+ final[i] }
        }
        return hash.map { String(format: "%08x", $0) }.joined()
    }
}
