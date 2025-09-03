use rexpect::spawn;
use rexpect::error::Error;

fn main() -> Result<(), Error> {
    let mut p = spawn("echo hello", Some(2000))?;
    p.exp_eof()?;
    let output = p.get_output();
    println!("Output: {}", output);
    Ok(())
}