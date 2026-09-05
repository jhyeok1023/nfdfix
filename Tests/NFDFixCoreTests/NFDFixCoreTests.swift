import Testing
@testable import NFDFixCore

@Test("NFD 파일명을 NFC로 변환")
func decomposedNameIsRecomposed() {
    let nfd = "\u{1112}\u{1161}\u{11AB}\u{1100}\u{1173}\u{11AF}.txt"
    let nfc = "\u{D55C}\u{AE00}.txt"
    let result = Normalization.normalizedName(nfd)
    #expect(result?.unicodeScalars.elementsEqual(nfc.unicodeScalars) == true)
}

@Test("이미 NFC인 파일명은 변환하지 않음")
func alreadyComposedNameReturnsNil() {
    #expect(Normalization.normalizedName("\u{D55C}\u{AE00}.txt") == nil)
}
