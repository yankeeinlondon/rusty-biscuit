//! `load_data(..., {header: false})`: every CSV or TSV record is a row by
//! position, and a failure is located at the raw offset of the record and
//! cell it names (`/0/1` is the second cell of the first input record).
