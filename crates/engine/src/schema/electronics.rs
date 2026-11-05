//! Native electronics command contracts, shared by CLI, MCP and desktop command execution.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use solidcraft_doc::electronics::native::{Edit, Project, Report};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Inspect {}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Replace {
    /// Complete electronics graph. Does not replace mechanical features or the e-bike project.
    pub project: Project,
    /// Revision returned by a fresh electronics.inspect. Rejects stale edits.
    pub expected_revision: u64,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Apply {
    /// Ordered edits; validate references after the whole transaction. 1..1024 edits.
    pub edits: Vec<Edit>,
    /// Revision returned by a fresh electronics.inspect. Rejects stale edits.
    pub expected_revision: u64,
}
#[derive(Debug, Serialize, JsonSchema)]
pub struct Reply {
    pub present: bool,
    pub project: Project,
    pub revision: u64,
    pub report: Report,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub param_warnings: Vec<String>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::schema::tests::{assert_keys, conforms};
    use crate::schema::{input_schema_for, output_schema_for};
    #[test]
    fn contracts_and_reply_match_commands() {
        assert_keys(input_schema_for::<Inspect>(), &[]);
        assert_keys(input_schema_for::<Replace>(), &["expected_revision", "project"]);
        assert_keys(input_schema_for::<Apply>(), &["edits", "expected_revision"]);
        let mut s = crate::Session::default();
        let reply = s.execute("electronics.inspect", &serde_json::json!({})).unwrap();
        let schema = serde_json::to_value(output_schema_for::<Reply>()).unwrap();
        assert!(conforms(&schema, &schema, &reply));
    }
}
