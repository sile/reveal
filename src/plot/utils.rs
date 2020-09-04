use anyhow::bail;
use std::process::Command;

pub fn execute_gnuplot(script_path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
    let output = Command::new("gnuplot")
        .args(&[script_path.as_ref()])
        .output()?;
    if !output.status.success() {
        if let Ok(err) = String::from_utf8(output.stderr) {
            bail!("Gnuplot error: {}", err);
        }
    }
    Ok(())
}
