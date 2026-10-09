use super::*;

/// Reads a child's pipe up to `MAX_OUTPUT_BYTES` and truncates, so an
/// untrusted command cannot exhaust memory by flooding its output (#917).
pub(super) fn drain_reader<R: std::io::Read>(reader: R) -> Vec<u8> {
    use std::io::Read;
    let mut buf = Vec::new();
    let _ = reader
        .take(MAX_OUTPUT_BYTES as u64 + 1)
        .read_to_end(&mut buf);
    buf.truncate(MAX_OUTPUT_BYTES);
    buf
}
