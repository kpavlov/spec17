import Testing

@Test("[REQ_001] underscore tag") func a() {
    #expect(true)
}
@Test("[PAR-1.2, FR-001] dotted and Spec Kit tags") func b() {
    #expect(true)
}
@Test("[REQ_003.1] dotted underscore tag") func d() {
    #expect(true)
}
@Test("[UTF-8] is no tag") func c() {
    #expect(true)
}
