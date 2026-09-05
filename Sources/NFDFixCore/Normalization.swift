import Foundation

public enum Normalization {
    public static func normalizedName(_ name: String) -> String? {
        let nfc = name.precomposedStringWithCanonicalMapping
        guard !nfc.unicodeScalars.elementsEqual(name.unicodeScalars) else { return nil }
        return nfc
    }
}
