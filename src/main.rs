use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
enum Opt {
    Importance(reveal::importance::ImportanceOpt),
    Plot(reveal::plot::PlotOpt),
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();
    match opt {
        Opt::Importance(opt) => {
            let records = reveal::io::read_records(std::io::stdin().lock())
                .collect::<anyhow::Result<Vec<_>>>()?;
            let importances = opt.calculate_importances(&records)?;
            serde_json::to_writer(std::io::stdout().lock(), &importances)?;
            println!();
        }
        Opt::Plot(opt) => {
            opt.plot(std::io::stdin().lock())?;
        }
    }
    Ok(())
}
