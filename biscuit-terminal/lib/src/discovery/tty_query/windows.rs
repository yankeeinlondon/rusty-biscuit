//! The native Windows [`QueryPort`]: the console's own input and output
//! buffers (`CONIN$`, `CONOUT$`), read as input records.
//!
//! The console delivers terminal replies the same way it delivers typed keys,
//! as key events, so a reply is recovered by translating key-down characters
//! back into bytes. Every record consumed is kept with the bytes it produced:
//! what the exchange identifies as typeahead is written back into the input
//! buffer afterwards, so keys typed during a query still reach the program.

use std::collections::VecDeque;
use std::fs::File;
use std::io::Write;
use std::os::windows::io::AsRawHandle;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{ERROR_BROKEN_PIPE, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows_sys::Win32::System::Console::{
    CONSOLE_MODE, ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT,
    ENABLE_PROCESSED_OUTPUT, ENABLE_VIRTUAL_TERMINAL_INPUT, ENABLE_VIRTUAL_TERMINAL_PROCESSING,
    GetConsoleMode, GetNumberOfConsoleInputEvents, INPUT_RECORD, KEY_EVENT, PeekConsoleInputW,
    ReadConsoleInputW, SetConsoleMode, WriteConsoleInputW,
};
use windows_sys::Win32::System::Threading::WaitForSingleObject;

use super::{QueryPort, ReadOutcome, TtyQueryError};

/// Records read from the console per `ReadConsoleInputW` call.
const RECORD_BATCH: usize = 64;

pub(super) struct WindowsPort {
    input: File,
    output: File,
    input_mode: CONSOLE_MODE,
    output_mode: CONSOLE_MODE,
    log: RecordLog,
    /// Translated bytes not yet handed to the exchange.
    pending: VecDeque<u8>,
    /// Bytes handed to the exchange so far; they are a prefix of the log's.
    delivered: usize,
}

/// Opens the console buffers by name rather than through the std handles, so
/// a program whose stdio is redirected still queries its console. Fails with
/// [`TtyQueryError::Io`] when either is not a console.
pub(super) fn open_port() -> Result<WindowsPort, TtyQueryError> {
    let open = |name: &str| {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(name)
            .map_err(|e| TtyQueryError::Io(format!("open {name}: {e}")))
    };
    let input = open("CONIN$")?;
    let output = open("CONOUT$")?;
    let input_mode = console_mode(&input).map_err(|e| TtyQueryError::Io(format!("CONIN$: {e}")))?;
    let output_mode =
        console_mode(&output).map_err(|e| TtyQueryError::Io(format!("CONOUT$: {e}")))?;

    // Built before any mode changes, so a failure below restores both.
    let port = WindowsPort {
        input,
        output,
        input_mode,
        output_mode,
        log: RecordLog::default(),
        pending: VecDeque::new(),
        delivered: 0,
    };
    let raw_input = (input_mode | ENABLE_VIRTUAL_TERMINAL_INPUT)
        & !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT);
    set_console_mode(&port.input, raw_input)
        .map_err(|e| TtyQueryError::Io(format!("CONIN$ mode: {e}")))?;
    // A console that refuses VT processing (a legacy conhost) would print the
    // queries as text.
    set_console_mode(
        &port.output,
        output_mode | ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING,
    )
    .map_err(|e| TtyQueryError::Io(format!("CONOUT$ mode: {e}")))?;
    Ok(port)
}

impl Drop for WindowsPort {
    fn drop(&mut self) {
        let _ = set_console_mode(&self.input, self.input_mode);
        let _ = set_console_mode(&self.output, self.output_mode);
    }
}

impl QueryPort for WindowsPort {
    fn write_all(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.output.write_all(bytes)?;
        self.output.flush()
    }

    fn read_within(
        &mut self,
        buffer: &mut [u8],
        timeout: Duration,
    ) -> std::io::Result<ReadOutcome> {
        let deadline = Instant::now() + timeout;
        while self.pending.is_empty() {
            let remaining = deadline.saturating_duration_since(Instant::now());
            // Below INFINITE (u32::MAX), which would never time out.
            let ms = u32::try_from(remaining.as_millis()).unwrap_or(u32::MAX - 1);
            // SAFETY: `input` is an open console input handle for the call.
            match unsafe { WaitForSingleObject(handle(&self.input), ms) } {
                WAIT_OBJECT_0 => {}
                WAIT_TIMEOUT => return Ok(ReadOutcome::TimedOut),
                _ => return Err(std::io::Error::last_os_error()),
            }
            let mut records = [INPUT_RECORD::default(); RECORD_BATCH];
            let count = match pending_events(&self.input)? {
                0 => 0,
                waiting => {
                    let mut read = 0u32;
                    let wanted = waiting.min(RECORD_BATCH as u32);
                    // SAFETY: `records` holds `RECORD_BATCH` writable records and
                    // `wanted` never exceeds that; at least one event is queued,
                    // so the call does not block.
                    let ok = unsafe {
                        ReadConsoleInputW(
                            handle(&self.input),
                            records.as_mut_ptr(),
                            wanted,
                            &mut read,
                        )
                    };
                    if ok == 0 {
                        let error = std::io::Error::last_os_error();
                        // The console's host has gone; waiting cannot help.
                        if error.raw_os_error() == Some(ERROR_BROKEN_PIPE as i32) {
                            return Ok(ReadOutcome::Hangup);
                        }
                        return Err(error);
                    }
                    read as usize
                }
            };
            for record in &records[..count] {
                self.pending.extend(self.log.push(*record));
            }
            if self.pending.is_empty() && Instant::now() >= deadline {
                return Ok(ReadOutcome::TimedOut);
            }
        }
        let n = buffer.len().min(self.pending.len());
        for (slot, byte) in buffer.iter_mut().zip(self.pending.drain(..n)) {
            *slot = byte;
        }
        self.delivered += n;
        Ok(ReadOutcome::Data(n))
    }

    fn input_pending(&mut self) -> std::io::Result<bool> {
        let waiting = pending_events(&self.input)?;
        if waiting == 0 {
            return Ok(false);
        }
        let mut records = vec![INPUT_RECORD::default(); waiting as usize];
        let mut read = 0u32;
        // SAFETY: `records` holds `waiting` writable records; peeking consumes
        // nothing.
        let ok = unsafe {
            PeekConsoleInputW(
                handle(&self.input),
                records.as_mut_ptr(),
                waiting,
                &mut read,
            )
        };
        if ok == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(records[..read as usize]
            .iter()
            .any(|record| key_down_char(record).is_some_and(|c| c != 0)))
    }

    /// Re-injection appends to the input buffer, so a key typed between the
    /// exchange's last read and this call lands ahead of the returned ones.
    fn return_typeahead(&mut self, response: &[u8], typeahead: &[std::ops::Range<usize>]) {
        debug_assert!(response.len() <= self.delivered);
        let records = self.log.typeahead_records(response.len(), typeahead);
        if records.is_empty() {
            return;
        }
        let mut written = 0u32;
        // SAFETY: `records` is a valid slice of `len` input records for the
        // call; the handle has write access.
        let ok = unsafe {
            WriteConsoleInputW(
                handle(&self.input),
                records.as_ptr(),
                records.len() as u32,
                &mut written,
            )
        };
        if ok == 0 || written as usize != records.len() {
            tracing::debug!(
                error = %std::io::Error::last_os_error(),
                written,
                wanted = records.len(),
                "typeahead not fully returned to the console"
            );
        }
    }
}

fn handle(file: &File) -> HANDLE {
    file.as_raw_handle() as HANDLE
}

fn console_mode(file: &File) -> std::io::Result<CONSOLE_MODE> {
    let mut mode: CONSOLE_MODE = 0;
    // SAFETY: `file` is an open handle and `mode` is a valid out pointer.
    if unsafe { GetConsoleMode(handle(file), &mut mode) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(mode)
}

fn set_console_mode(file: &File, mode: CONSOLE_MODE) -> std::io::Result<()> {
    // SAFETY: `file` is an open console handle.
    if unsafe { SetConsoleMode(handle(file), mode) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

fn pending_events(input: &File) -> std::io::Result<u32> {
    let mut count = 0u32;
    // SAFETY: `input` is an open console input handle; `count` is a valid out
    // pointer.
    if unsafe { GetNumberOfConsoleInputEvents(handle(input), &mut count) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(count)
}

/// The fields of a key record this port reads.
#[derive(Clone, Copy)]
struct KeyEvent {
    down: bool,
    virtual_key: u16,
    unit: u16,
    repeat: u16,
}

impl KeyEvent {
    /// Which key a key-up releases: the same virtual key and character.
    fn identity(self) -> (u16, u16) {
        (self.virtual_key, self.unit)
    }
}

fn key_event(record: &INPUT_RECORD) -> Option<KeyEvent> {
    if u32::from(record.EventType) != KEY_EVENT {
        return None;
    }
    // SAFETY: `EventType` is KEY_EVENT, so `KeyEvent` is the active variant;
    // `UnicodeChar` is the variant ReadConsoleInputW (W) fills.
    let (event, unit) = unsafe {
        (
            record.Event.KeyEvent,
            record.Event.KeyEvent.uChar.UnicodeChar,
        )
    };
    Some(KeyEvent {
        down: event.bKeyDown != 0,
        virtual_key: event.wVirtualKeyCode,
        unit,
        repeat: event.wRepeatCount,
    })
}

/// The UTF-16 unit of a key-down record; `None` for every other record.
fn key_down_char(record: &INPUT_RECORD) -> Option<u16> {
    key_event(record).filter(|key| key.down).map(|key| key.unit)
}

fn is_key_up(record: &INPUT_RECORD) -> bool {
    key_event(record).is_some_and(|key| !key.down)
}

/// Every record consumed during one exchange, and which of them produced each
/// translated byte.
#[derive(Default)]
struct RecordLog {
    records: Vec<INPUT_RECORD>,
    /// Per translated byte, the first and last record that produced it (two
    /// for a character split across a surrogate pair).
    sources: Vec<(usize, usize)>,
    /// Per record, the key-down a key-up record releases.
    released: Vec<Option<usize>>,
    /// Key-downs not yet released.
    held: Vec<usize>,
    /// A high surrogate waiting for its low half.
    high: Option<(u16, usize)>,
}

impl RecordLog {
    /// Records `record` and returns the bytes it completes.
    fn push(&mut self, record: INPUT_RECORD) -> Vec<u8> {
        let index = self.records.len();
        self.records.push(record);
        let mut released = None;
        match key_event(&record) {
            Some(key) if key.down => self.held.push(index),
            Some(key) => {
                let at = self.held.iter().rposition(|&down| {
                    key_event(&self.records[down]).map(KeyEvent::identity) == Some(key.identity())
                });
                released = at.map(|at| self.held.remove(at));
            }
            None => {}
        }
        self.released.push(released);

        let Some(key) = key_event(&record).filter(|key| key.down && key.unit != 0) else {
            return Vec::new();
        };
        let (unit, repeat) = (key.unit, usize::from(key.repeat).max(1));
        let mut bytes = Vec::new();
        match unit {
            0xD800..=0xDBFF => {
                if let Some((_, orphan)) = self.high.take() {
                    self.emit(char::REPLACEMENT_CHARACTER, (orphan, orphan), 1, &mut bytes);
                }
                self.high = Some((unit, index));
            }
            0xDC00..=0xDFFF => match self.high.take() {
                Some((high, first)) => {
                    let c = char::decode_utf16([high, unit])
                        .next()
                        .and_then(Result::ok)
                        .unwrap_or(char::REPLACEMENT_CHARACTER);
                    self.emit(c, (first, index), repeat, &mut bytes);
                }
                None => self.emit(char::REPLACEMENT_CHARACTER, (index, index), 1, &mut bytes),
            },
            _ => {
                if let Some((_, orphan)) = self.high.take() {
                    self.emit(char::REPLACEMENT_CHARACTER, (orphan, orphan), 1, &mut bytes);
                }
                let c = char::from_u32(u32::from(unit)).unwrap_or(char::REPLACEMENT_CHARACTER);
                self.emit(c, (index, index), repeat, &mut bytes);
            }
        }
        bytes
    }

    fn emit(&mut self, c: char, source: (usize, usize), repeat: usize, bytes: &mut Vec<u8>) {
        let mut utf8 = [0u8; 4];
        let encoded = c.encode_utf8(&mut utf8).as_bytes();
        for _ in 0..repeat {
            bytes.extend_from_slice(encoded);
            self.sources
                .extend(std::iter::repeat_n(source, encoded.len()));
        }
    }

    /// The records to hand back to the console, in the order they were read.
    ///
    /// A record goes back unless every byte it produced lies in `examined`
    /// (the bytes the exchange looked at) outside `typeahead`, i.e. inside a
    /// terminal reply. Bytes past `examined` were read but never looked at,
    /// so they go back too. Records that produced no byte (focus, mouse,
    /// modifier-only keys) go back; a key-up goes back with its key-down.
    fn typeahead_records(
        &self,
        examined: usize,
        typeahead: &[std::ops::Range<usize>],
    ) -> Vec<INPUT_RECORD> {
        let mut keep = vec![false; self.records.len()];
        let mut produced = vec![false; self.records.len()];
        for (byte, &(first, last)) in self.sources.iter().enumerate() {
            let returned = byte >= examined || typeahead.iter().any(|r| r.contains(&byte));
            for index in first..=last {
                produced[index] = true;
                keep[index] |= returned;
            }
        }
        // A surrogate half still waiting for its partner produced nothing yet.
        for (index, record) in self.records.iter().enumerate() {
            if !produced[index] && !is_key_up(record) {
                keep[index] = true;
            }
        }
        for (index, released) in self.released.iter().enumerate() {
            if is_key_up(&self.records[index]) {
                keep[index] = released.is_none_or(|down| keep[down]);
            }
        }
        self.records
            .iter()
            .zip(keep)
            .filter_map(|(record, keep)| keep.then_some(*record))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::System::Console::{FOCUS_EVENT, KEY_EVENT_RECORD};

    fn key(down: bool, unit: u16) -> INPUT_RECORD {
        let mut record = INPUT_RECORD {
            EventType: KEY_EVENT as u16,
            ..Default::default()
        };
        record.Event.KeyEvent = KEY_EVENT_RECORD {
            bKeyDown: i32::from(down),
            wRepeatCount: 1,
            ..Default::default()
        };
        record.Event.KeyEvent.uChar.UnicodeChar = unit;
        record
    }

    fn focus() -> INPUT_RECORD {
        INPUT_RECORD {
            EventType: FOCUS_EVENT as u16,
            ..Default::default()
        }
    }

    fn typed(text: &str) -> Vec<INPUT_RECORD> {
        text.encode_utf16()
            .flat_map(|unit| [key(true, unit), key(false, unit)])
            .collect()
    }

    fn log_of(records: &[INPUT_RECORD]) -> (RecordLog, Vec<u8>) {
        let mut log = RecordLog::default();
        let bytes = records.iter().flat_map(|r| log.push(*r)).collect();
        (log, bytes)
    }

    fn units(records: &[INPUT_RECORD]) -> Vec<(bool, u16)> {
        records
            .iter()
            .filter_map(|r| key_event(r).map(|key| (key.down, key.unit)))
            .collect()
    }

    #[test]
    fn key_downs_translate_to_utf8_and_key_ups_add_nothing() {
        let (_, bytes) = log_of(&typed("a\x1b[?1cé"));
        assert_eq!(bytes, "a\x1b[?1cé".as_bytes());
    }

    #[test]
    fn surrogate_pairs_join_into_one_character() {
        let (log, bytes) = log_of(&typed("😀"));
        assert_eq!(bytes, "😀".as_bytes());
        assert!(
            log.sources
                .iter()
                .all(|&(first, last)| (first, last) == (0, 2))
        );
    }

    #[test]
    fn zero_characters_produce_no_bytes() {
        let (_, bytes) = log_of(&[key(true, 0), key(false, 0)]);
        assert!(bytes.is_empty());
    }

    #[test]
    fn typeahead_keys_go_back_with_their_releases_and_replies_do_not() {
        let mut records = typed("ab");
        records.push(focus());
        records.extend(typed("\x1b[?62c"));
        let (log, bytes) = log_of(&records);
        let ranges = super::super::typeahead_ranges(&bytes);
        let back = log.typeahead_records(bytes.len(), &ranges);
        assert_eq!(
            units(&back),
            vec![
                (true, 'a' as u16),
                (false, 'a' as u16),
                (true, 'b' as u16),
                (false, 'b' as u16)
            ]
        );
        assert_eq!(back.len(), 5, "the focus record goes back too");
    }

    #[test]
    fn bytes_past_the_examined_prefix_go_back() {
        let mut records = typed("\x1b[?62c");
        records.extend(typed("z"));
        let (log, bytes) = log_of(&records);
        let back = log.typeahead_records(bytes.len() - 1, &[]);
        assert_eq!(units(&back), vec![(true, 'z' as u16), (false, 'z' as u16)]);
    }

    #[test]
    fn modifier_only_keys_go_back() {
        let (log, _) = log_of(&[key(true, 0), key(false, 0)]);
        assert_eq!(log.typeahead_records(0, &[]).len(), 2);
    }
}
