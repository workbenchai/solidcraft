//! Build the native e-bike study without the desktop/GPU stack.
//! Run: cargo run -p solidcraft-engine --example ebike-native -- /tmp/ebike-study
use serde_json::json;
use solidcraft_engine::Session;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let prefix = std::env::args().nth(1).unwrap_or_else(|| "ebike-study".into());
    let mut session = Session::default();
    session.execute("ebike.create", &json!({}))?;
    session.execute("file.save_as", &json!({"path": format!("{prefix}.sc")}))?;
    session.execute("file.export", &json!({"path": format!("{prefix}.obj")}))?;
    let report = session.execute("ebike.inspect", &json!({}))?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
