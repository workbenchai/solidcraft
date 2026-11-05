use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Warning,
    Error,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub code: String,
    /// Stable entity address, e.g. boards/controller/traces/signal.
    pub entity: String,
    pub severity: Severity,
    pub message: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub fingerprint: String,
    pub findings: Vec<Finding>,
    pub performed: Vec<String>,
    pub not_performed: Vec<String>,
    /// Always false until the separate manufacturing qualification checks are implemented.
    pub manufacturing_ready: bool,
}
impl Project {
    /// Bounded basic ERC and dimensional checks, explicitly short of full board DRC.
    pub fn check(&self) -> Result<Report> {
        let fingerprint = self.fingerprint()?;
        let mut findings = Vec::new();
        let mut add = |code: &str, entity: String, severity, message: &str| {
            findings.push(Finding { code: code.into(), entity, severity, message: message.into() })
        };
        let memberships: BTreeMap<_, _> = self.nets.iter().flat_map(|(id, n)| n.pins.iter().map(move |p| (p, id))).collect();
        for (cid, c) in &self.components {
            for (n, pin) in &c.pins {
                let r = PinRef { component: cid.clone(), pin: n.clone() };
                if !pin.no_connect && !memberships.contains_key(&r) {
                    add(
                        "pin_unassigned",
                        format!("components/{}/pins/{n}", cid.0),
                        Severity::Warning,
                        "Pin has neither a net nor an intentional no-connect",
                    );
                }
            }
        }
        for (nid, n) in &self.nets {
            let mut drivers = 0;
            for pin in &n.pins {
                if self
                    .components
                    .get(&pin.component)
                    .and_then(|c| c.pins.get(&pin.pin))
                    .is_some_and(|p| matches!(p.electrical_type, PinType::Output | PinType::PowerOutput))
                {
                    drivers += 1;
                }
            }
            if drivers > 1 {
                add(
                    "multiple_drivers",
                    format!("nets/{}", nid.0),
                    Severity::Error,
                    "Net has multiple push-pull/power drivers; verify the intended circuit",
                );
            }
            if n.pins.len() < 2 {
                add("single_pin_net", format!("nets/{}", nid.0), Severity::Warning, "Net has fewer than two logical pins");
            }
        }
        // Combine overlapping netgroup constraints conservatively using the strongest minimum.
        let mut widths: BTreeMap<&NetId, f64> = BTreeMap::new();
        let mut clearances: BTreeMap<&NetId, f64> = BTreeMap::new();
        for g in self.netgroups.values() {
            for n in &g.nets {
                if let Some(w) = g.min_trace_width_mm {
                    widths.entry(n).and_modify(|v| *v = v.max(w)).or_insert(w);
                }
                if let Some(c) = g.min_clearance_mm {
                    clearances.entry(n).and_modify(|v| *v = v.max(c)).or_insert(c);
                }
            }
        }
        for (bid, b) in &self.boards {
            let base = format!("boards/{}", bid.0);
            for (tid, t) in &b.traces {
                if t.width_mm < b.rules.min_trace_width_mm.max(widths.get(&t.net).copied().unwrap_or(0.)) {
                    add("trace_width", format!("{base}/traces/{}", tid.0), Severity::Error, "Trace is narrower than the board/netgroup minimum");
                }
                if t.points.iter().any(|&p| !b.outline.contains(p)) {
                    add(
                        "trace_outside",
                        format!("{base}/traces/{}", tid.0),
                        Severity::Error,
                        "Trace centreline vertex lies outside the board or inside a cutout",
                    );
                }
            }
            for (vid, v) in &b.vias {
                if v.drill_mm < b.rules.min_drill_mm || (v.diameter_mm - v.drill_mm) / 2. < b.rules.min_annular_ring_mm {
                    add("via_dimensions", format!("{base}/vias/{}", vid.0), Severity::Error, "Via drill or annular ring is below the board minimum");
                }
                if !b.outline.contains(v.at) {
                    add("via_outside", format!("{base}/vias/{}", vid.0), Severity::Error, "Via centre lies outside the board or inside a cutout");
                }
            }
            for (pid, pad) in &b.pads {
                if !b.outline.contains(pad.at) {
                    add("pad_outside", format!("{base}/pads/{}", pid.0), Severity::Error, "Pad centre lies outside the board or inside a cutout");
                }
                if let Some(d) = &pad.drill {
                    let (w, h) = match pad.shape {
                        PadShape::Circle { diameter_mm } => (diameter_mm, diameter_mm),
                        PadShape::Rectangle { width_mm, height_mm } | PadShape::RoundedRectangle { width_mm, height_mm, .. } => (width_mm, height_mm),
                    };
                    if d.diameter_mm < b.rules.min_drill_mm
                        || d.plated && ((w - d.slot_length_mm).min(h) - d.diameter_mm) / 2. < b.rules.min_annular_ring_mm
                    {
                        add("pad_drill", format!("{base}/pads/{}", pid.0), Severity::Error, "Pad drill or annular ring is below the board minimum");
                    }
                }
            }
            for (pid, p) in &b.pours {
                if p.clearance_mm < b.rules.min_clearance_mm.max(clearances.get(&p.net).copied().unwrap_or(0.)) {
                    add("pour_clearance", format!("{base}/pours/{}", pid.0), Severity::Error, "Pour clearance is below the board/netgroup minimum");
                }
                add(
                    "pour_unfilled",
                    format!("{base}/pours/{}", pid.0),
                    Severity::Warning,
                    "Pour boundary stores intent; actual copper fill has not been computed",
                );
            }
        }
        Ok(Report {
            fingerprint,
            findings,
            manufacturing_ready: false,
            performed: [
                "structural_references",
                "basic_pin_erc",
                "trace_width",
                "drill_and_annular_ring",
                "centre_point_bounds",
                "pour_clearance_setting",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            not_performed: [
                "copper_connectivity_and_unrouted_detection",
                "copper_clearance_and_shorts",
                "full_shape_edge_clearance",
                "pour_fill_and_thermal_connections",
                "schematic_path_endpoint_geometry",
                "impedance",
                "voltage_and_current_ratings",
                "manufacturing_process_qualification",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        })
    }
}
