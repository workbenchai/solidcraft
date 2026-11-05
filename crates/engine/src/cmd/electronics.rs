//! Native electrical graph commands. Every transaction is one undoable design edit.
use super::CommandSpec;
use crate::params::bad;
use crate::schema::electronics::{Apply, Inspect, Replace, Reply};
use crate::schema::{TypedContract, output_schema_for};
use crate::{Result, Session};
use serde_json::Value;
use solidcraft_doc::electronics::native::Project;

const APPLY: TypedContract<Apply> = TypedContract::new();
const INSPECT: TypedContract<Inspect> = TypedContract::new();
const REPLACE: TypedContract<Replace> = TypedContract::new();
pub static COMMANDS: &[CommandSpec] = &[
    CommandSpec::new("electronics.apply", "Edit Electronics", crate::checked!("electronics.apply", apply))
        .doc_only()
        .icon("component")
        .contract(&APPLY)
        .output(output_schema_for::<Reply>)
        .params("edits, expected_revision: atomically add, update or remove native electrical entities"),
    CommandSpec::new("electronics.inspect", "Inspect Electronics", crate::checked!("electronics.inspect", inspect))
        .doc_only()
        .noundo()
        .icon("inspect")
        .contract(&INSPECT)
        .output(output_schema_for::<Reply>)
        .params("→ native electronics project, revision and scoped validation report; absent project is empty"),
    CommandSpec::new("electronics.replace", "Set Electronics", crate::checked!("electronics.replace", replace))
        .doc_only()
        .icon("component")
        .contract(&REPLACE)
        .output(output_schema_for::<Reply>)
        .params("project, expected_revision: validate and save the complete electrical graph in the native design"),
];
fn decode<T: serde::de::DeserializeOwned>(p: &Value, cmd: &str) -> Result<T> {
    serde_json::from_value(p.clone()).map_err(|e| bad(cmd, e.to_string()))
}
fn reply(project: Project, present: bool, revision: u64) -> Result<Value> {
    let report = project.check().map_err(|e| bad("electronics.inspect", e.to_string()))?;
    serde_json::to_value(Reply { present, project, revision, report, param_warnings: vec![] }).map_err(|e| bad("electronics.inspect", e.to_string()))
}
fn inspect(s: &mut Session, _: &Value) -> Result<Value> {
    s.doc.validate_electronics()?;
    reply(s.doc.electronics.clone().unwrap_or_default(), s.doc.electronics.is_some(), s.revision)
}
fn accept(s: &mut Session, project: Project, expected: u64, cmd: &str) -> Result<Value> {
    if s.revision != expected {
        return Err(bad(cmd, "design changed; inspect again before editing"));
    }
    // Validate and compute the response before touching the live document. This also verifies
    // cross-domain mechanical links. Session::execute provides undo and rollback on failure.
    let mut next = s.doc.as_ref().clone();
    next.electronics = Some(project.clone());
    next.validate_electronics()?;
    let revision = s.revision.checked_add(1).ok_or_else(|| bad(cmd, "revision exhausted"))?;
    let response = reply(project.clone(), true, revision)?;
    s.set_doc(next);
    Ok(response)
}
fn replace(s: &mut Session, p: &Value) -> Result<Value> {
    let a: Replace = decode(p, "electronics.replace")?;
    accept(s, a.project, a.expected_revision, "electronics.replace")
}
fn apply(s: &mut Session, p: &Value) -> Result<Value> {
    let a: Apply = decode(p, "electronics.apply")?;
    if s.revision != a.expected_revision {
        return Err(bad("electronics.apply", "design changed; inspect again before editing"));
    }
    let project = s.doc.electronics.clone().unwrap_or_default().apply(&a.edits).map_err(|e| bad("electronics.apply", e.to_string()))?;
    accept(s, project, a.expected_revision, "electronics.apply")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serde_json::json;
    use solidcraft_doc::electronics::native::{Edit, example};
    #[test]
    fn save_reopen_undo_and_stale_write() {
        let mut s = Session::default();
        let p = example::rc_filter();
        let r = s.execute("electronics.replace", &json!({"project":p,"expected_revision":s.revision})).unwrap();
        assert_eq!(r["revision"], s.revision);
        assert_eq!(s.doc.electronics.as_ref(), Some(&p));
        let d = solidcraft_doc::Document::from_json(&s.doc.to_json()).unwrap();
        assert_eq!(d.electronics, Some(p.clone()));
        assert!(s.execute("electronics.replace", &json!({"project":p,"expected_revision":s.revision-1})).is_err());
        s.execute("edit.undo", &json!({})).unwrap();
        assert!(s.doc.electronics.is_none());
        s.execute("edit.redo", &json!({})).unwrap();
        assert_eq!(s.doc.electronics, Some(p));
    }
    #[test]
    fn invalid_transaction_preserves_document_and_mechanical_design() {
        let mut s = Session::default();
        s.doc_mut().name = "Mechanical assembly".into();
        let p = example::rc_filter();
        s.execute("electronics.replace", &json!({"project":p,"expected_revision":s.revision})).unwrap();
        let before = s.doc.clone();
        let edit = Edit::RemoveNet { id: "ground".into() };
        assert!(s.execute("electronics.apply", &json!({"edits":[edit],"expected_revision":s.revision})).is_err());
        assert_eq!(s.doc, before);
        assert_eq!(s.doc.name, "Mechanical assembly");
        assert!(s.execute("electronics.apply", &json!({"edits":[],"expected_revision":s.revision,"typo":1})).is_err());
    }
}
