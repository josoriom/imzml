use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;

const EMPTY_BINARY_TAG: &[u8] = b"<binary/>";
const NORMALIZED_BINARY_TAG: &[u8] = b"<binary></binary>";
const BLOCK_SIZE: usize = 1024 * 1024;

pub(crate) fn normalize_imzml_file(input_path: &Path, output_path: &Path) -> io::Result<()> {
    let reader = BufReader::with_capacity(BLOCK_SIZE, File::open(input_path)?);
    let writer = BufWriter::with_capacity(BLOCK_SIZE, File::create(output_path)?);
    normalize_stream(reader, writer, BLOCK_SIZE)
}

fn normalize_stream<R: Read, W: Write>(
    mut reader: R,
    mut writer: W,
    block_size: usize,
) -> io::Result<()> {
    let mut buffer = vec![0u8; block_size];
    let mut pending: Vec<u8> = Vec::with_capacity(block_size + EMPTY_BINARY_TAG.len());

    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        pending.extend_from_slice(&buffer[..read]);
        replace_empty_binary_tags(&mut pending, &mut writer, false)?;
    }

    replace_empty_binary_tags(&mut pending, &mut writer, true)?;
    writer.flush()
}

fn replace_empty_binary_tags(
    pending: &mut Vec<u8>,
    writer: &mut dyn Write,
    is_final: bool,
) -> io::Result<()> {
    let tag = EMPTY_BINARY_TAG;
    let mut cursor = 0;
    let mut written = 0;

    while cursor < pending.len() {
        let Some(found) = find_byte(&pending[cursor..], b'<') else {
            cursor = pending.len();
            break;
        };
        let open = cursor + found;
        if open + tag.len() > pending.len() {
            cursor = if is_final { pending.len() } else { open };
            break;
        }
        if pending[open..].starts_with(tag) {
            writer.write_all(&pending[written..open])?;
            writer.write_all(NORMALIZED_BINARY_TAG)?;
            cursor = open + tag.len();
            written = cursor;
        } else {
            cursor = open + 1;
        }
    }

    writer.write_all(&pending[written..cursor])?;
    pending.drain(..cursor);
    Ok(())
}

fn find_byte(bytes: &[u8], target: u8) -> Option<usize> {
    bytes.iter().position(|&byte| byte == target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn normalize(input: &str, block_size: usize) -> String {
        let mut output = Vec::new();
        normalize_stream(Cursor::new(input.as_bytes()), &mut output, block_size).unwrap();
        String::from_utf8(output).unwrap()
    }

    #[test]
    fn replaces_empty_binary_tag() {
        assert_eq!(normalize("<binary/>", 64), "<binary></binary>");
    }

    #[test]
    fn leaves_other_tags_alone() {
        assert_eq!(
            normalize("<cvParam name=\"x\"/><binary/>", 64),
            "<cvParam name=\"x\"/><binary></binary>"
        );
    }

    #[test]
    fn leaves_full_binary_alone() {
        assert_eq!(normalize("<binary></binary>", 64), "<binary></binary>");
    }

    #[test]
    fn replaces_tag_split_across_every_block_boundary() {
        let input = "head<binary/>mid<binary/>tail";
        let expected = "head<binary></binary>mid<binary></binary>tail";
        for block_size in 1..=input.len() {
            assert_eq!(normalize(input, block_size), expected, "block_size={block_size}");
        }
    }
}
