//! Print a reproducible native electronics project and its explicitly scoped validation report.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project = solidcraft_electronics::example::rc_filter();
    let report = project.check()?;
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({"project":project,"report":report}))?);
    Ok(())
}
