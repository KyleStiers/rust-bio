// Copyright 2014 Johannes Köster.
// Licensed under the MIT license (http://opensource.org/licenses/MIT)
// This file may not be copied, modified, or distributed
// except according to those terms.

//! FastQ reading and writing.

use std::io;
use std::io::prelude::*;
use std::ascii::AsciiExt;


/// A FastQ reader.
pub struct FastqReader<R: io::Read> {
    reader: io::BufReader<R>,
    sep_line: String
}


impl<R: io::Read> FastqReader<R> {
    /// Create a new FastQ reader.
    pub fn new(reader: R) -> Self {
        FastqReader { reader: io::BufReader::new(reader), sep_line: String::new() }
    }

    /// Read into a given record.
    /// Returns an error if the record in incomplete or syntax is violated.
    /// The content of the record can be checked via the record object.
    pub fn read(&mut self, record: &mut Record) -> io::Result<()> {
        record.clear();
        try!(self.reader.read_line(&mut record.header));

        if !record.header.is_empty() {
            if !record.header.starts_with("@") {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Expected @ at record start.",
                    None,
                ));
            }
            try!(self.reader.read_line(&mut record.seq));
            try!(self.reader.read_line(&mut self.sep_line));
            try!(self.reader.read_line(&mut record.qual));
            if record.qual.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Incomplete record.",
                    Some("Each FastQ record has to consist of 4 lines: header, sequence, separator and qualities.".to_string()),
                ))
            }
        }

        Ok(())
    }

    /// Return an iterator over the records of this FastQ file.
    pub fn records(self) -> Records<R> {
        Records { reader: self }
    }
}


/// A FastQ record.
pub struct Record {
    header: String,
    seq: String,
    qual: String,
}


impl Record {
    /// Create a new, empty FastQ record.
    pub fn new() -> Self {
        Record {
            header: String::new(),
            seq: String::new(),
            qual: String::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.header.is_empty() && self.seq.is_empty() && self.qual.is_empty()
    }

    /// Check validity of FastQ record.
    pub fn check(&self) -> Result<(), &str> {
        if self.id().is_none() {
            return Err("Expecting id for FastQ record.");
        }
        if !self.seq.is_ascii() {
            return Err("Non-ascii character found in sequence.");
        }
        if !self.qual.is_ascii() {
            return Err("Non-ascii character found in qualities.");
        }
        if self.seq().len() != self.qual().len() {
            return Err("Unequal length of sequence an qualities.");
        }

        Ok(())
    }

    /// Return the id of the record.
    pub fn id(&self) -> Option<&str> {
        self.header[1..].words().next()
    }

    /// Return descriptions if present.
    pub fn desc(&self) -> Vec<&str> {
        self.header[1..].words().skip(1).collect()
    }

    /// Return the sequence of the record.
    pub fn seq(&self) -> &[u8] {
        self.seq.trim_right().as_bytes()
    }

    /// Return the base qualities of the record.
    pub fn qual(&self) -> &[u8] {
        self.qual.trim_right().as_bytes()
    }

    fn clear(&mut self) {
        self.header.clear();
        self.seq.clear();
        self.qual.clear();
    }
}


/// An iterator over the records of a FastQ file.
pub struct Records<R: io::Read> {
    reader: FastqReader<R>,
}


impl<R: io::Read> Iterator for Records<R> {
    type Item = io::Result<Record>;

    fn next(&mut self) -> Option<io::Result<Record>> {
        let mut record = Record::new();
        match self.reader.read(&mut record) {
            Ok(()) if record.is_empty() => None,
            Ok(())   => Some(Ok(record)),
            Err(err) => Some(Err(err))
        }
    }
}


/// A FastQ writer.
pub struct FastqWriter<W: io::Write> {
    writer: io::BufWriter<W>,
}


impl<W: io::Write> FastqWriter<W> {
    /// Create a new FastQ writer.
    pub fn new(writer: W) -> Self {
        FastqWriter { writer: io::BufWriter::new(writer) }
    }

    /// Directly write a FastQ record.
    pub fn write_record(&mut self, record: Record) -> io::Result<()> {
        self.write(record.id().unwrap_or(""), &record.desc(), record.seq(), record.qual())
    }

    /// Write a FastQ record with given values.
    ///
    /// # Arguments
    ///
    /// * `id` - the record id
    /// * `desc` - the optional descriptions
    /// * `seq` - the sequence
    /// * `qual` - the qualities
    pub fn write(&mut self, id: &str, desc: &[&str], seq: &[u8], qual: &[u8]) -> io::Result<()> {
        try!(self.writer.write(b"@"));
        try!(self.writer.write(id.as_bytes()));
        if !desc.is_empty() {
            for d in desc {
                try!(self.writer.write(b" "));
                try!(self.writer.write(d.as_bytes()));
            }
        }
        try!(self.writer.write(b"\n"));
        try!(self.writer.write(seq));
        try!(self.writer.write(b"\n+\n"));
        try!(self.writer.write(qual));
        try!(self.writer.write(b"\n"));

        Ok(())
    }

    /// Flush the writer, ensuring that everything is written.
    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    const FASTQ_FILE: &'static [u8] = b"@id desc
ACCGTAGGCTGA
+
IIIIIIJJJJJJ
";

    #[test]
    fn test_reader() {
        let reader = FastqReader::new(FASTQ_FILE);
        let records: Vec<io::Result<Record>> = reader.records().collect();
        assert!(records.len() == 1);
        for res in records {
            let record = res.ok().unwrap();
            assert_eq!(record.check(), Ok(()));
            assert_eq!(record.id(), Some("id"));
            assert_eq!(record.desc(), ["desc"]);
            assert_eq!(record.seq(), b"ACCGTAGGCTGA");
            assert_eq!(record.qual(), b"IIIIIIJJJJJJ");
        }
    }

    #[test]
    fn test_writer() {
        let mut writer = FastqWriter::new(Vec::new());
        writer.write("id", &["desc"], b"ACCGTAGGCTGA", b"IIIIIIJJJJJJ").ok().expect("Expected successful write");
        writer.flush().ok().expect("Expected successful write");
        assert_eq!(writer.writer.get_ref(), &FASTQ_FILE);
    }
}