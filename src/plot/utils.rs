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

pub fn normalize_filename(s: &str) -> String {
    let mut t = String::new();
    let mut replaced = false;
    for c in s.to_ascii_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            replaced = false;
            t.push(c);
        } else if !replaced {
            replaced = true;
            t.push('-');
        }
    }
    t.trim_matches('-').to_owned()
}
