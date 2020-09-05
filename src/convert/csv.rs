use crate::utils::build_id_mapping;
use hporecord::{Record, StudyRecord};
use std::collections::BTreeMap;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct ConvertCsvOpt {
    #[structopt(long)]
    pub table_name: String,

    #[structopt(long, default_value = "result/csv/")]
    pub out: PathBuf,
}

impl ConvertCsvOpt {
    pub fn convert(&self, records: &[Record]) -> anyhow::Result<()> {
        let id_mapping = build_id_mapping(&self.table_name, records)?;

        // TODO: Handle categorical
        let mut tables = BTreeMap::new();
        for record in records {
            match record {
                Record::Study(study) => {
                    let table_name = id_mapping.get(&study.id).unwrap_or(&study.id);
                    if !tables.contains_key(table_name) {
                        tables.insert(table_name.clone(), Table::new(study));
                    } else {
                        // TODO: Add validation
                    }
                }
                Record::Eval(eval) => {
                    let table_name = id_mapping.get(&eval.study).expect("TODO: warn and skip");

                    if !eval.state.is_complete() {
                        continue;
                    }

                    let table = tables.get_mut(table_name).expect("unreachable");
                    let row = eval
                        .params
                        .iter()
                        .chain(eval.values.iter())
                        .copied()
                        .collect();
                    table.rows.push(row);
                }
            }
        }

        std::fs::create_dir_all(&self.out)?;
        for (name, table) in tables {
            let path = self.out.join(format!("{}.csv", name));
            let mut writer = csv::WriterBuilder::new().from_path(&path)?;
            writer.write_record(&table.column_names)?;
            for row in table.rows {
                writer.write_record(&row.iter().map(|v| v.to_string()).collect::<Vec<_>>())?;
            }
            eprintln!("Generated: {:?}", path);
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Table {
    column_names: Vec<String>,
    rows: Vec<Vec<f64>>,
}

impl Table {
    fn new(record: &StudyRecord) -> Self {
        Self {
            column_names: record
                .params
                .iter()
                .map(|p| p.name.clone())
                .chain(record.values.iter().map(|v| v.name.clone()))
                .collect(),
            rows: Vec::new(),
        }
    }
}
