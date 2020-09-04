use crate::importance::{Importance, Importances};
use crate::plot::utils::execute_gnuplot;
use serde_json;
use std::collections::BTreeMap;
use std::io::Write;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum PlotImportanceOpt {
    StackedBar(PlotStackedBarOpt),
}

impl PlotImportanceOpt {
    pub fn plot(&self, reader: impl std::io::BufRead) -> anyhow::Result<()> {
        let importances: Importances = serde_json::from_reader(reader)?;
        match self {
            Self::StackedBar(opt) => opt.plot(importances),
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct PlotStackedBarOpt {
    #[structopt(long, default_value = "plot-results/importance-stacked-bar/")]
    pub out: std::path::PathBuf,

    /// Image width in pixels.
    #[structopt(long, default_value = "1200")]
    pub width: usize,

    /// Image height in pixels.
    #[structopt(long, default_value = "600")]
    pub height: usize,

    #[structopt(long)]
    pub retain_temp_file: bool,
}

impl PlotStackedBarOpt {
    pub fn plot(&self, importances: Importances) -> anyhow::Result<()> {
        let mut groups = BTreeMap::<_, Vec<_>>::new();
        for (study_id, importances) in &importances {
            let mut key = importances
                .iter()
                .map(|im| im.params.as_slice())
                .collect::<Vec<_>>();
            key.sort();
            groups
                .entry(key)
                .or_default()
                .push((study_id.as_str(), importances.as_slice()));
        }

        std::fs::create_dir_all(&self.out)?;
        for (i, (key, importances)) in groups.into_iter().enumerate() {
            self.plot_stacked_bar(i, &key, &importances)?;
        }
        Ok(())
    }

    fn generate_data_file(
        &self,
        path: &std::path::PathBuf,
        params_list: &[&[String]],
        studies_list: &[(&str, &[Importance])],
    ) -> anyhow::Result<()> {
        let file = std::fs::File::create(path)?;
        let mut writer = std::io::BufWriter::new(file);

        write!(writer, "Study")?;
        for params in params_list {
            write!(writer, " {:?}", params.join("&"))?;
        }
        writeln!(writer)?;

        for (study_id, importances) in studies_list {
            write!(writer, "{:?}", study_id)?;
            let total = importances.iter().map(|im| im.importance.mean).sum::<f64>();
            for params in params_list {
                let importance = importances
                    .iter()
                    .find(|im| im.params == *params)
                    .expect("unreachable")
                    .importance;
                write!(writer, " {}", importance.mean / total)?;
            }
            writeln!(writer)?;
        }

        Ok(())
    }

    fn make_gnuplot_script(
        &self,
        path: &std::path::PathBuf,
        data_path: &std::path::PathBuf,
        png_path: &std::path::PathBuf,
        x_count: usize,
    ) -> anyhow::Result<()> {
        let file = std::fs::File::create(path)?;
        let mut writer = std::io::BufWriter::new(file);

        writeln!(writer, "set title \"Parameter Importance\"")?;
        writeln!(writer, "set key invert reverse Left outside")?;
        writeln!(writer, "set key autotitle columnheader")?;
        writeln!(writer, "set yrange [0:1]")?;
        writeln!(writer, "set auto x")?;
        writeln!(writer, "unset xtics")?;
        writeln!(writer, "set xtics nomirror rotate by -45 scale 0")?;
        writeln!(writer, "set style data histogram")?;
        writeln!(writer, "set style histogram rowstacked")?;
        writeln!(writer, "set style fill solid border -1")?;
        writeln!(writer, "set boxwidth 0.8")?;
        writeln!(
            writer,
            "set terminal pngcairo size {},{} noenhanced",
            self.width, self.height
        )?;
        writeln!(writer, "set output {:?};", png_path)?;
        write!(writer, "plot {:?} using 2:xtic(1)", data_path)?;
        if x_count > 1 {
            write!(writer, ", for [i=3:{}] '' using i", 3 + (x_count - 2))?;
        }
        writeln!(writer)?;

        Ok(())
    }

    fn plot_stacked_bar(
        &self,
        number: usize,
        params_list: &[&[String]],
        studies_list: &[(&str, &[Importance])],
    ) -> anyhow::Result<()> {
        let data_file_path = self.out.join(format!("{}.dat", number));
        let script_file_path = self.out.join(format!("{}.gp", number));
        let png_file_path = self.out.join(format!("{}.png", number));

        self.generate_data_file(&data_file_path, params_list, studies_list)?;

        self.make_gnuplot_script(
            &script_file_path,
            &data_file_path,
            &png_file_path,
            params_list.len(),
        )?;

        execute_gnuplot(&script_file_path)?;

        if !self.retain_temp_file {
            std::fs::remove_file(data_file_path)?;
            std::fs::remove_file(script_file_path)?;
        }

        eprintln!("Generated: {:?}", png_file_path);

        Ok(())
    }
}
