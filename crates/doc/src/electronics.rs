//! Electronics are saved with the mechanical document and can link boards/placements to CAD components.
use crate::{DocError, Document, Result};

/// Native Rust electronics types for extension authors.
pub use solidcraft_electronics as native;

impl Document {
    pub fn validate_electronics(&self) -> Result<()> {
        if let Some(p) = &self.electronics {
            p.validate().map_err(|e| DocError::Invalid(e.to_string()))?;
            for b in p.boards.values() {
                for id in b.mechanical_component.iter().chain(b.placements.values().filter_map(|p| p.mechanical_component.as_ref())) {
                    if !self.components.iter().any(|c| c.id == *id) {
                        return Err(DocError::Invalid(format!("electronics references unknown mechanical component {id}")));
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    #[test]
    fn native_json_round_trip_and_old_documents() {
        let d = Document { electronics: Some(solidcraft_electronics::example::rc_filter()), ..Document::default() };
        let reopened = Document::from_json(&d.to_json()).unwrap();
        assert_eq!(d.electronics, reopened.electronics);
        let mut old = serde_json::to_value(Document::default()).unwrap();
        old.as_object_mut().unwrap().remove("electronics");
        assert!(Document::from_json(&old.to_string()).unwrap().electronics.is_none());
    }
    #[test]
    fn invalid_graph_and_dangling_mechanical_links_are_refused_on_open() {
        let mut p = solidcraft_electronics::example::rc_filter();
        p.boards.values_mut().next().unwrap().mechanical_component = Some(9999);
        let d = Document { electronics: Some(p), ..Document::default() };
        assert!(Document::from_json(&d.to_json()).is_err());
    }
    #[test]
    fn linked_mechanical_component_cannot_be_deleted_until_unlinked() {
        let mut d = Document::default();
        let id = d.add_component(Some("Filter board"), 0).unwrap();
        let mut p = solidcraft_electronics::example::rc_filter();
        p.boards.values_mut().next().unwrap().mechanical_component = Some(id);
        d.electronics = Some(p);
        d.validate_electronics().unwrap();
        let before = d.clone();
        assert!(d.remove_component_tree(id).is_err());
        assert_eq!(d, before);
        d.electronics.as_mut().unwrap().boards.values_mut().next().unwrap().mechanical_component = None;
        d.remove_component_tree(id).unwrap();
        d.validate_electronics().unwrap();
    }
}
