use crate::curve::{Studies, Study};
use crate::plot::utils;
use ordered_float::OrderedFloat;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct PlotCurveOpt {
    #[structopt(long, default_value = "plot-results/optimization-curve/")]
    pub out: std::path::PathBuf,

    /// Image width in pixels.
    #[structopt(long, default_value = "800")]
    pub width: usize,

    /// Image height in pixels.
    #[structopt(long, default_value = "600")]
    pub height: usize,

    #[structopt(long)]
    pub retain_temp_file: bool,

    /// Minimum value of Y axis.
    #[structopt(long)]
    pub ymin: Option<f64>,

    /// Maximum value of Y axis.
    #[structopt(long)]
    pub ymax: Option<f64>,

    /// Minimum value of X axis.
    #[structopt(long)]
    pub xmin: Option<f64>,

    /// Maximum value of X axis.
    #[structopt(long)]
    pub xmax: Option<f64>,

    /// Makes Y axis log scale.
    #[structopt(long)]
    pub ylogscale: bool,

    /// Displays errorbar showing standard deviation of optimization curve.
    #[structopt(long)]
    pub errorbar: bool,
}

impl PlotCurveOpt {
    pub fn plot(&self, reader: impl std::io::BufRead) -> anyhow::Result<()> {
        let studies: Studies = serde_json::from_reader(reader)?;
        std::fs::create_dir_all(&self.out)?;

        for (problem_id, studies) in studies {
            self.plot_curve(&problem_id, &studies)?;
        }

        Ok(())
    }

    fn plot_curve(
        &self,
        problem_id: &str,
        studies: &BTreeMap<String, Study>,
    ) -> anyhow::Result<()> {
        let filename_stem = utils::normalize_filename(problem_id);
        let data_file_path = self.out.join(format!("{}.dat", filename_stem));
        let script_file_path = self.out.join(format!("{}.gp", filename_stem));
        let png_file_path = self.out.join(format!("{}.png", filename_stem));

        self.generate_data_file(&data_file_path, studies)?;
        self.make_gnuplot_script(
            problem_id,
            &script_file_path,
            &data_file_path,
            &png_file_path,
            studies,
        )?;
        utils::execute_gnuplot(&script_file_path)?;

        if !self.retain_temp_file {
            std::fs::remove_file(data_file_path)?;
            std::fs::remove_file(script_file_path)?;
        }

        eprintln!("Generated: {:?}", png_file_path);

        Ok(())
    }

    fn make_gnuplot_script<P: AsRef<Path>>(
        &self,
        problem_id: &str,
        gp_path: P,
        dat_path: P,
        png_path: P,
        studies: &BTreeMap<String, Study>,
    ) -> anyhow::Result<()> {
        let file = std::fs::File::create(gp_path)?;
        let mut w = std::io::BufWriter::new(file);

        writeln!(w, "set title {:?}", problem_id)?;
        writeln!(
            w,
            "set ylabel {:?}",
            studies.values().next().expect("unreachable").objective.name
        )?;
        writeln!(
            w,
            "set xlabel {:?}",
            studies.values().next().expect("unreachable").span_name
        )?;
        writeln!(w, "set datafile missing \"NaN\"")?;

        if self.ylogscale {
            writeln!(w, "set logscale y")?;
        }

        writeln!(
            w,
            "set terminal pngcairo size {},{} noenhanced",
            self.width, self.height
        )?;
        writeln!(w, "set output {:?}", png_path.as_ref())?;

        if self.errorbar {
            writeln!(w, "set style fill transparent solid 0.2")?;
            writeln!(w, "set style fill noborder")?;
        }

        write!(
            w,
            "plot [{}:{}] [{}:{}]",
            self.xmin(studies),
            self.xmax(studies),
            self.ymin(studies),
            self.ymax(studies)
        )?;

        for i in 0..studies.len() {
            if i == 0 {
                write!(w, " {:?}", dat_path.as_ref())?;
            } else {
                write!(w, ", \"\"")?;
            }
            write!(w, " u ($0):{} w l t columnhead lc {}", (i * 2) + 1, i + 1)?;
            if self.errorbar {
                write!(
                    w,
                    ", \"\" u ($0 - 1):(${}-${}):(${}+${}) with filledcurves notitle lc {}",
                    (i * 2) + 1,
                    (i * 2) + 1 + 1,
                    (i * 2) + 1,
                    (i * 2) + 1 + 1,
                    i + 1
                )?;
            }
        }
        writeln!(w)?;

        Ok(())
    }

    fn generate_data_file<P: AsRef<Path>>(
        &self,
        dat_path: P,
        studies: &BTreeMap<String, Study>,
    ) -> anyhow::Result<()> {
        let file = std::fs::File::create(dat_path)?;
        let mut w = std::io::BufWriter::new(file);

        for (optimizer, study) in studies {
            let name = format!("{} (n={})", optimizer, study.samples);
            write!(w, "{:?} {:?} ", name, name)?;
        }
        writeln!(w)?;

        let size = x_len(studies);
        for i in 0..size {
            for study in studies.values() {
                write!(
                    w,
                    "{} {} ",
                    study.best_values_avg.mean[i]
                        .map_or(Cow::Borrowed("NaN"), |v| Cow::Owned(v.to_string())),
                    study.best_values_avg.stddev[i]
                        .map_or(Cow::Borrowed("NaN"), |v| Cow::Owned(v.to_string()))
                )?;
            }
            writeln!(w)?;
        }

        Ok(())
    }

    fn ymax(&self, studies: &BTreeMap<String, Study>) -> String {
        let is_minimize = studies
            .values()
            .next()
            .expect("unreachable")
            .objective
            .direction
            .is_minimize();

        if let Some(y) = self.ymax {
            y.to_string()
        } else if is_minimize {
            let max_step = x_len(studies) - 1;
            let step = max_step / 5;

            if let Some(y) = studies
                .values()
                .filter_map(|s| s.best_values_avg.mean[step].map(OrderedFloat))
                .max()
            {
                let ymax = y.0.to_string();
                if ymax == self.ymin(studies) {
                    "".to_string()
                } else {
                    ymax
                }
            } else {
                "".to_string()
            }
        } else {
            let i = x_len(studies) - 1;
            let j = i / 5;

            if let Some(y) = studies
                .values()
                .filter_map(|s| {
                    Some(OrderedFloat(
                        s.best_values_avg.mean[i].expect("unreachable")
                            + (s.best_values_avg.mean[i].expect("unreachable")
                                - s.best_values_avg.mean[j].expect("unreachable"))
                                * 0.1,
                    ))
                })
                .max()
            {
                y.0.to_string()
            } else {
                "".to_string()
            }
        }
    }

    fn ymin(&self, studies: &BTreeMap<String, Study>) -> String {
        let is_minimize = studies
            .values()
            .next()
            .expect("unreachable")
            .objective
            .direction
            .is_minimize();
        if let Some(y) = self.ymin {
            y.to_string()
        } else if is_minimize {
            let i = x_len(studies) - 1;
            let j = i / 5;

            if let Some(y) = studies
                .values()
                .filter_map(|s| {
                    Some(OrderedFloat(
                        s.best_values_avg.mean[i].expect("unreachable")
                            + (s.best_values_avg.mean[i].expect("unreachable")
                                - s.best_values_avg.mean[j].expect("unreachable"))
                                * 0.1,
                    ))
                })
                .min()
            {
                y.0.to_string()
            } else {
                "".to_string()
            }
        } else {
            let max_step = x_len(studies) - 1;
            let step = max_step / 5;

            if let Some(y) = studies
                .values()
                .filter_map(|s| s.best_values_avg.mean[step].map(OrderedFloat))
                .min()
            {
                let ymin = y.0.to_string();
                if ymin == self.ymax(studies) {
                    "".to_string()
                } else {
                    ymin
                }
            } else {
                "".to_string()
            }
        }
    }

    fn xmin(&self, _studies: &BTreeMap<String, Study>) -> String {
        self.xmin
            .map(|v| v.to_string())
            .unwrap_or_else(|| "".to_string())
    }

    fn xmax(&self, _studies: &BTreeMap<String, Study>) -> String {
        self.xmax
            .map(|v| v.to_string())
            .unwrap_or_else(|| "".to_string())
    }
}

fn x_len(studies: &BTreeMap<String, Study>) -> usize {
    studies
        .values()
        .map(|study| study.best_values_avg.mean.len())
        .min()
        .expect("unreachable")
}
