use std::{
    ffi::OsString,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
};

pub const URI_LIST: &str = "text/uri-list";
const FILE_SCHEME: &str = "file://";
const HEX_DIGITS: &[u8; 16] = b"0123456789ABCDEF";

pub fn uri_list(path: &Path) -> Vec<u8> {
    let bytes = path.as_os_str().as_bytes();
    let mut list = Vec::with_capacity(FILE_SCHEME.len() + bytes.len() + 2);
    list.extend_from_slice(FILE_SCHEME.as_bytes());
    for &byte in bytes {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte) {
            list.push(byte);
        } else {
            list.extend_from_slice(&[
                b'%',
                HEX_DIGITS[usize::from(byte >> 4)],
                HEX_DIGITS[usize::from(byte & 0xF)],
            ]);
        }
    }
    list.extend_from_slice(b"\r\n");
    list
}

/// The local paths a `text/uri-list` names, percent-decoded, skipping
/// comments and anything that is not a `file://` URI on this machine.
pub fn file_paths(list: &[u8]) -> impl Iterator<Item = PathBuf> + '_ {
    list.split(|&byte| byte == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
        .filter(|line| !line.starts_with(b"#"))
        .filter_map(|line| line.strip_prefix(FILE_SCHEME.as_bytes()))
        .filter_map(|rest| rest.strip_prefix(b"localhost").or(Some(rest)))
        .filter(|path| path.starts_with(b"/"))
        .map(|path| PathBuf::from(OsString::from_vec(percent_decode(path))))
}

fn percent_decode(encoded: &[u8]) -> Vec<u8> {
    let hex = |byte: u8| (byte as char).to_digit(16);
    let mut decoded = Vec::with_capacity(encoded.len());
    let mut index = 0;
    while let Some(&byte) = encoded.get(index) {
        let escaped = (byte == b'%')
            .then(|| {
                Some((
                    hex(*encoded.get(index + 1)?)?,
                    hex(*encoded.get(index + 2)?)?,
                ))
            })
            .flatten();
        match escaped {
            Some((high, low)) => {
                decoded.push((high * 16 + low) as u8);
                index += 3;
            }
            None => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    decoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_uri_list_reads_back_as_the_paths_it_was_made_from() {
        let path = Path::new("/usr/share/applications/my app%.desktop");
        let mut list = b"# from crownos-home\r\n".to_vec();
        list.extend(uri_list(path));
        list.extend(b"https://example.com/x\r\nfile://localhost/opt/b.desktop\r\n");
        let paths: Vec<_> = file_paths(&list).collect();
        assert_eq!(paths, [path.to_owned(), PathBuf::from("/opt/b.desktop")]);
    }

    #[test]
    fn reserved_bytes_are_percent_encoded() {
        let list = uri_list(Path::new("/usr/share/applications/my app%.desktop"));
        assert_eq!(
            list,
            b"file:///usr/share/applications/my%20app%25.desktop\r\n"
        );
    }
}
