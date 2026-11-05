//! Native electronics intermediate representation. No external EDA runtime or file format is required.
//!
//! Millimetres and radians for geometry; SI for electrical quantities. Logical connectivity is
//! authoritative in `Net::pins`; schematics and copper refer back to it. A valid project may be
//! unfinished. Structural validation, design findings and manufacturing qualification are distinct.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod checks;
mod edit;
pub mod example;
mod geometry;
mod model;
mod validate;

pub use checks::{Finding, Report, Severity};
pub use edit::Edit;
pub use geometry::{Point, Polygon};
pub use model::*;

use sha2::{Digest, Sha256};

#[derive(Debug, thiserror::Error)]
#[error("electronics: {0}")]
pub struct Error(pub String);
pub type Result<T> = std::result::Result<T, Error>;

impl Project {
    /// Structural validation only. Does not establish ERC, copper connectivity or manufacturability.
    pub fn validate(&self) -> Result<()> {
        validate::project(self)
    }

    /// Hash of the complete canonical model, including geometry, hierarchy and schematic views.
    pub fn fingerprint(&self) -> Result<String> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|e| Error(e.to_string()))?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }

    /// Resolve the single source of pin connectivity. Call `validate` before traversing a model.
    pub fn pin_net(&self, pin: &PinRef) -> Option<&NetId> {
        self.nets.iter().find_map(|(id, net)| net.pins.contains(pin).then_some(id))
    }

    /// Resolve a pad to its intended net, without inferring any copper connection.
    pub fn pad_net(&self, pad: &Pad) -> Option<&NetId> {
        match &pad.connection {
            Connection::Pin { pin } => self.pin_net(pin),
            Connection::Net { net } => self.nets.get_key_value(net).map(|(id, _)| id),
            Connection::None => None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests;
