use anyhow::Context;
use hporecord::Record;
use std::io::BufRead;

pub fn read_records<T: BufRead>(reader: T) -> Records<T> {
    Records {
        line_reader: reader,
        line_no: 0,
        line: String::new(),
    }
}

#[derive(Debug)]
pub struct Records<T> {
    line_reader: T,
    line_no: usize,
    line: String,
}

impl<T: BufRead> Records<T> {
    fn read_record(&mut self) -> anyhow::Result<Option<Record>> {
        self.line.clear();
        self.line_no += 1;
        let size = self
            .line_reader
            .read_line(&mut self.line)
            .with_context(|| format!("line={}", self.line_no))?;
        if size == 0 {
            Ok(None)
        } else {
            let record = serde_json::from_str(&self.line)
                .with_context(|| format!("line={}", self.line_no))?;
            Ok(Some(record))
        }
    }
}

impl<T: BufRead> Iterator for Records<T> {
    type Item = anyhow::Result<Record>;

    fn next(&mut self) -> Option<Self::Item> {
        self.read_record().transpose()
    }
}
