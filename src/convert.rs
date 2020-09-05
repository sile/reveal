use hporecord::Record;
use structopt::StructOpt;

pub mod csv;

#[derive(Debug, StructOpt)]
pub enum ConvertOpt {
    Csv(self::csv::ConvertCsvOpt),
}

impl ConvertOpt {
    pub fn convert(&self, records: &[Record]) -> anyhow::Result<()> {
        match self {
            Self::Csv(opt) => opt.convert(records),
        }
    }
}
