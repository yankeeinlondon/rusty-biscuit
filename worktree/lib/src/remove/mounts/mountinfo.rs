//! Linux's `/proc/<pid>/mountinfo` format (see `proc_pid_mountinfo(5)`).
//! Parsing is bytes in, bytes out, so its tests run on every host; only the
//! conversion to [`Mount`] needs Unix paths.

use std::io;

#[cfg(unix)]
use super::Mount;

/// One mountinfo line's mount point and source, with the kernel's octal
/// escapes decoded.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Entry {
    pub(super) mount_point: Vec<u8>,
    pub(super) source: Vec<u8>,
}

impl Entry {
    #[cfg(unix)]
    pub(super) fn into_mount(self) -> Mount {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        Mount {
            mount_point: OsStr::from_bytes(&self.mount_point).into(),
            source: String::from_utf8_lossy(&self.source).into_owned(),
        }
    }
}

/// Every entry of a mountinfo table. Blank lines are skipped.
///
/// ## Errors
///
/// [`io::ErrorKind::InvalidData`] naming the first malformed or truncated
/// line: skipping it could hide a mount the deletion must not cross.
pub(super) fn parse(text: &[u8]) -> io::Result<Vec<Entry>> {
    text.split(|&byte| byte == b'\n')
        .enumerate()
        .filter(|(_, line)| !line.is_empty())
        .map(|(index, line)| {
            parse_line(line).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("mountinfo line {} is malformed", index + 1)))
        })
        .collect()
}

/// Fields: mount ID, parent ID, `major:minor`, root, mount point, options,
/// zero or more optional fields, `-`, then filesystem type, source, and
/// superblock options.
fn parse_line(line: &[u8]) -> Option<Entry> {
    const FIXED_FIELDS: usize = 6;
    let fields: Vec<&[u8]> = line.split(|&byte| byte == b' ').collect();
    let separator = FIXED_FIELDS + fields.get(FIXED_FIELDS..)?.iter().position(|field| *field == b"-")?;
    // A line cut short loses its superblock options first.
    if fields.len() < separator + 4 {
        return None;
    }
    let is_number = |field: &[u8]| !field.is_empty() && field.iter().all(u8::is_ascii_digit);
    if !is_number(fields[0]) || !is_number(fields[1]) || !fields[2].contains(&b':') || !fields[4].starts_with(b"/") {
        return None;
    }
    Some(Entry { mount_point: unescape(fields[4]), source: unescape(fields[separator + 2]) })
}

/// Decodes `\ooo` octal escapes (the kernel writes space, tab, newline, and
/// backslash this way); any other backslash is kept as is.
fn unescape(field: &[u8]) -> Vec<u8> {
    let mut decoded = Vec::with_capacity(field.len());
    let mut index = 0;
    while index < field.len() {
        let escaped = if field[index] == b'\\' { field.get(index + 1..index + 4).and_then(octal) } else { None };
        match escaped {
            Some(byte) => {
                decoded.push(byte);
                index += 4;
            }
            None => {
                decoded.push(field[index]);
                index += 1;
            }
        }
    }
    decoded
}

fn octal(digits: &[u8]) -> Option<u8> {
    let value = digits.iter().try_fold(0u16, |value, &digit| (b'0'..=b'7').contains(&digit).then(|| value * 8 + u16::from(digit - b'0')))?;
    u8::try_from(value).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(mount_point: &[u8], source: &[u8]) -> Entry {
        Entry { mount_point: mount_point.to_vec(), source: source.to_vec() }
    }

    #[test]
    fn a_plain_line_yields_its_mount_point_and_source() {
        let text = b"22 1 8:1 / / rw,relatime - ext4 /dev/sda1 rw,errors=remount-ro\n";

        assert_eq!(parse(text).unwrap(), [entry(b"/", b"/dev/sda1")]);
    }

    #[test]
    fn optional_fields_before_the_separator_are_skipped() {
        let text = b"36 35 98:0 /mnt1 /mnt2 rw,noatime shared:1 master:2 - ext3 /dev/root rw,errors=continue\n";

        assert_eq!(parse(text).unwrap(), [entry(b"/mnt2", b"/dev/root")]);
    }

    #[test]
    fn octal_escapes_are_decoded_in_the_mount_point_and_source() {
        let text = b"40 22 0:35 / /mnt/my\\040disk\\011tab\\012nl\\134bs rw - fuse src\\040dir rw\n";

        assert_eq!(parse(text).unwrap(), [entry(b"/mnt/my disk\ttab\nnl\\bs", b"src dir")]);
    }

    #[test]
    fn a_backslash_without_three_octal_digits_is_kept() {
        let text = b"40 22 0:35 / /mnt/a\\9b\\04 rw - tmpfs tmpfs rw\n";

        assert_eq!(parse(text).unwrap(), [entry(b"/mnt/a\\9b\\04", b"tmpfs")]);
    }

    /// A bind mount shares `/`'s device and source; only its root field
    /// (the bound subdirectory) and its mount point differ.
    #[test]
    fn a_same_device_bind_mount_is_listed() {
        let text = b"22 1 8:1 / / rw - ext4 /dev/sda1 rw\n\
                     51 22 8:1 /srv/vendor /home/me/wt/feat/vendor rw shared:1 - ext4 /dev/sda1 rw\n";

        assert_eq!(parse(text).unwrap(), [entry(b"/", b"/dev/sda1"), entry(b"/home/me/wt/feat/vendor", b"/dev/sda1")]);
    }

    #[test]
    fn blank_lines_and_a_missing_final_newline_are_accepted() {
        let text = b"\n22 1 8:1 / / rw - ext4 /dev/sda1 rw\n\n23 22 0:5 / /dev rw - devtmpfs udev rw";

        assert_eq!(parse(text).unwrap(), [entry(b"/", b"/dev/sda1"), entry(b"/dev", b"udev")]);
    }

    #[test]
    fn a_malformed_line_fails_the_whole_table_naming_the_line() {
        let text = b"22 1 8:1 / / rw - ext4 /dev/sda1 rw\nnot a mountinfo line at all\n";

        let error = parse(text).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("line 2"), "{error}");
    }

    /// Every prefix of a valid line that drops a field is rejected, so a
    /// truncated read never yields a half-parsed entry.
    #[test]
    fn a_truncated_line_is_never_half_parsed() {
        let line: &[u8] = b"36 35 98:0 /mnt1 /mnt2 rw,noatime master:1 - ext3 /dev/root rw";
        let last_field_start = line.iter().rposition(|&byte| byte == b' ').unwrap() + 1;

        for end in 1..last_field_start {
            assert!(parse(&line[..end]).is_err(), "{:?}", String::from_utf8_lossy(&line[..end]));
        }
    }

    #[test]
    fn a_line_with_too_few_fixed_fields_is_rejected() {
        assert!(parse(b"22 1 8:1 / - ext4 /dev/sda1 rw\n").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_non_utf8_mount_point_keeps_its_bytes() {
        use std::os::unix::ffi::OsStrExt;

        let mut parsed = parse(b"40 22 0:35 / /mnt/caf\\351 rw - tmpfs tmpfs rw\n").unwrap();
        let mount = parsed.remove(0).into_mount();

        assert_eq!(mount.mount_point.as_os_str().as_bytes(), b"/mnt/caf\xe9");
        assert_eq!(mount.source, "tmpfs");
    }
}
