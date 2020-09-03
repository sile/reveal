use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
enum Opt {
    Importance(reveal::importance::ImportanceOpt),
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();
    match opt {
        Opt::Importance(opt) => {
            let records = reveal::io::read_records(std::io::stdin().lock())
                .collect::<anyhow::Result<Vec<_>>>()?;
            println!("# LEN: {}", records.len());
            //
            todo!()
        }
    }
    Ok(())
}
