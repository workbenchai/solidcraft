//! Create an editable board envelope and native electrical graph in one .sc document.
use serde_json::json;
use solidcraft_doc::electronics::native::example;
use solidcraft_engine::Session;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).unwrap_or_else(|| "electronics-study.sc".into());
    let mut session = Session::default();
    let component = session.execute("component.create", &json!({"name":"RC filter board","parent":"root"}))?;
    let id = component.get("component").and_then(serde_json::Value::as_u64).ok_or("missing created component")?;
    session.execute("solid.box", &json!({"corner":[0,0,-1.6],"length":40,"width":30,"height":1.6,"body_name":"Board substrate"}))?;
    let mut project = example::rc_filter();
    if let Some(board) = project.boards.values_mut().next() {
        board.mechanical_component = Some(id);
    }
    session.execute("electronics.replace", &json!({"project":project,"expected_revision":session.revision}))?;
    session.execute("file.save_as", &json!({"path":path}))?;
    println!("{}", serde_json::to_string_pretty(&session.execute("electronics.inspect", &json!({}))?)?);
    Ok(())
}
