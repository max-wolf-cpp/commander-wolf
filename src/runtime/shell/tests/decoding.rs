use super::*;

#[test]
fn strips_ansi_and_preserves_split_utf8() {
    let mut decoder = Decoder::default();
    assert_eq!(decoder.push(b"\x1b[31", false), "");
    assert_eq!(decoder.push(b"mred\x1b[0m\x1b]title\x07", false), "red");

    let bytes = "狼".as_bytes();
    assert_eq!(decoder.push(&bytes[..1], false), "");
    assert_eq!(decoder.push(&bytes[1..], false), "狼");
    assert_eq!(decoder.push(b"\x1b]52;c;payload\x1b\\safe", true), "safe");
}

#[test]
fn every_output_split_preserves_utf8_and_removes_ansi_sequences() {
    let source = "狼🙂\u{1b}[31mred\u{1b}[0m\u{1b}]title\u{1b}\\done".as_bytes();
    for boundary in 0..=source.len() {
        let mut decoder = Decoder::default();
        let mut text = decoder.push(&source[..boundary], false);
        text.push_str(&decoder.push(&source[boundary..], true));
        assert_eq!(text, "狼🙂reddone", "boundary {boundary}");
    }
}
