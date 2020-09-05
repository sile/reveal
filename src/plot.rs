use structopt::StructOpt;

pub mod curve;
pub mod importance;
pub mod utils;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum PlotOpt {
    Importance(self::importance::PlotImportanceOpt),
    Curve(self::curve::PlotCurveOpt),
}

impl PlotOpt {
    pub fn plot(&self, reader: impl std::io::BufRead) -> anyhow::Result<()> {
        match self {
            Self::Importance(opt) => opt.plot(reader),
            Self::Curve(opt) => opt.plot(reader),
        }
    }
}
