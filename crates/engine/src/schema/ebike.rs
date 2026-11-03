//! Parameter contracts for the native pedal-assist project commands.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use solidcraft_doc::ebike::{Estimate, Finding, Project};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Create {
    /// Optional project; default is the 48 V, 500 W continuous / 750 W peak study.
    #[serde(default)]
    pub project: Option<Project>,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Configure {
    /// Complete replacement project definition. Existing native features remain editable.
    pub project: Project,
    /// Document revision observed before editing. Reject stale writes.
    pub expected_revision: u64,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Inspect {}
#[derive(Debug, Serialize, JsonSchema)]
pub struct Reply {
    pub project: Project,
    pub revision: u64,
    pub estimate: Estimate,
    /// Failed consistency checks and explicit unresolved validation work.
    pub findings: Vec<Finding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub param_warnings: Vec<String>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::Session;
    use crate::schema::tests::assert_keys;
    use crate::schema::{ParamCheck, input_schema_for};
    use serde_json::json;
    #[test]
    fn schemas_list_the_supported_keys() {
        assert_keys(input_schema_for::<Create>(), &["project"]);
        assert_keys(input_schema_for::<Configure>(), &["expected_revision", "project"]);
        assert_keys(input_schema_for::<Inspect>(), &[]);
    }
    #[test]
    fn strict_unknown_keys_do_not_create_geometry() {
        let mut s = Session { param_check: ParamCheck::Strict, ..Session::default() };
        assert!(s.execute("ebike.create", &json!({"typo":1})).is_err());
        assert!(s.doc.features.is_empty() && s.doc.ebike.is_none());
    }
}
