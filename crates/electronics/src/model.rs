use crate::{Point, Polygon};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

// Distinct stable opaque identifiers prevent cross-domain reference mistakes. Map keys own
// identities; display names and reference designators can change without breaking references.
macro_rules! ids {
    ($($name:ident),* $(,)?) => {$ (
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
        #[serde(transparent)]
        pub struct $name(pub String);
        impl From<&str> for $name {
            fn from(value: &str) -> Self { Self(value.into()) }
        }
    )*};
}
ids!(BoardId, ComponentId, LayerId, NetId, NetGroupId, PadId, TraceId, ViaId, PourId, ViewId, WireId, SystemId);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub schema_version: u32,
    #[serde(default)]
    pub boards: BTreeMap<BoardId, Board>,
    #[serde(default)]
    pub components: BTreeMap<ComponentId, Component>,
    #[serde(default)]
    pub nets: BTreeMap<NetId, Net>,
    #[serde(default)]
    pub netgroups: BTreeMap<NetGroupId, NetGroup>,
    #[serde(default)]
    pub systems: BTreeMap<SystemId, System>,
    #[serde(default)]
    pub schematics: BTreeMap<ViewId, SchematicView>,
}
impl Default for Project {
    fn default() -> Self {
        Self {
            schema_version: 1,
            boards: BTreeMap::new(),
            components: BTreeMap::new(),
            nets: BTreeMap::new(),
            netgroups: BTreeMap::new(),
            systems: BTreeMap::new(),
            schematics: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Board {
    pub name: String,
    pub outline: Polygon,
    /// Stack in physical order, top to bottom, including dielectric layers.
    pub stackup: Vec<Layer>,
    pub rules: BoardRules,
    #[serde(default)]
    pub frame: BoardFrame,
    /// Optional SolidCraft mechanical component. Resolved by the document integration.
    #[serde(default)]
    pub mechanical_component: Option<u64>,
    #[serde(default)]
    pub placements: BTreeMap<ComponentId, Placement>,
    /// Pad coordinates are board-local, after component rotation/mirroring.
    #[serde(default)]
    pub pads: BTreeMap<PadId, Pad>,
    #[serde(default)]
    pub traces: BTreeMap<TraceId, Trace>,
    #[serde(default)]
    pub vias: BTreeMap<ViaId, Via>,
    #[serde(default)]
    pub pours: BTreeMap<PourId, Pour>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BoardFrame {
    pub origin_mm: [f64; 3],
    pub x_axis: [f64; 3],
    pub y_axis: [f64; 3],
}
impl Default for BoardFrame {
    fn default() -> Self {
        Self { origin_mm: [0.; 3], x_axis: [1., 0., 0.], y_axis: [0., 1., 0.] }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub kind: LayerKind,
    pub thickness_mm: f64,
    pub material: String,
    /// Relative permittivity for dielectrics, when specified.
    #[serde(default)]
    pub relative_permittivity: Option<f64>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LayerKind {
    Copper,
    Dielectric,
    SolderMask,
    Silkscreen,
    Paste,
    Mechanical,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BoardRules {
    pub min_trace_width_mm: f64,
    pub min_clearance_mm: f64,
    pub min_drill_mm: f64,
    pub min_annular_ring_mm: f64,
}
impl Default for BoardRules {
    fn default() -> Self {
        Self { min_trace_width_mm: 0.15, min_clearance_mm: 0.15, min_drill_mm: 0.2, min_annular_ring_mm: 0.1 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Top,
    Bottom,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    pub at: Point,
    pub rotation_rad: f64,
    pub side: Side,
    pub footprint: String,
    #[serde(default = "populated_default")]
    pub populated: bool,
    #[serde(default)]
    pub mechanical_component: Option<u64>,
}

fn populated_default() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PinRef {
    pub component: ComponentId,
    pub pin: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Component {
    pub reference: String,
    pub kind: ComponentKind,
    #[serde(default)]
    pub manufacturer: Option<String>,
    #[serde(default)]
    pub manufacturer_part_number: Option<String>,
    /// Local pin number -> pin. Multiple physical pads may map to the same pin.
    pub pins: BTreeMap<String, Pin>,
    #[serde(default)]
    pub system: Option<SystemId>,
    #[serde(default)]
    pub annotations: BTreeMap<String, String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ComponentKind {
    Resistor { resistance_ohm: f64, tolerance_fraction: f64, power_rating_w: f64 },
    Capacitor { capacitance_f: f64, tolerance_fraction: f64, voltage_rating_v: f64, polarized: bool },
    Inductor { inductance_h: f64, current_rating_a: f64, resistance_ohm: f64 },
    Ic { function: String },
    Discrete { function: String },
    Connector,
    TestPoint,
    Other { description: String },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Pin {
    pub name: String,
    pub electrical_type: PinType,
    /// Explicit intentional no-connect. A pin cannot also belong to a net.
    #[serde(default)]
    pub no_connect: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PinType {
    Passive,
    Input,
    Output,
    Bidirectional,
    TriState,
    OpenDrain,
    PowerInput,
    PowerOutput,
    Unspecified,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Net {
    pub name: String,
    pub pins: BTreeSet<PinRef>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NetGroup {
    pub name: String,
    pub nets: BTreeSet<NetId>,
    #[serde(default)]
    pub min_trace_width_mm: Option<f64>,
    #[serde(default)]
    pub min_clearance_mm: Option<f64>,
    /// Ordered positive/negative nets; both must also be group members.
    #[serde(default)]
    pub differential_pair: Option<[NetId; 2]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Connection {
    None,
    Pin { pin: PinRef },
    Net { net: NetId },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PadShape {
    Circle { diameter_mm: f64 },
    Rectangle { width_mm: f64, height_mm: f64 },
    RoundedRectangle { width_mm: f64, height_mm: f64, corner_radius_mm: f64 },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Drill {
    pub diameter_mm: f64,
    /// Distance between slot end centres; zero is a round drill.
    pub slot_length_mm: f64,
    pub plated: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Pad {
    pub at: Point,
    pub rotation_rad: f64,
    pub shape: PadShape,
    /// Explicit copper layer membership; a non-plated mechanical hole has no copper layers.
    pub layers: BTreeSet<LayerId>,
    #[serde(default)]
    pub drill: Option<Drill>,
    pub connection: Connection,
    pub solder_mask_expansion_mm: f64,
    pub paste_margin_mm: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Trace {
    pub net: NetId,
    pub layer: LayerId,
    /// Centreline polyline. Curves can be introduced as a versioned extension.
    pub points: Vec<Point>,
    pub width_mm: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Via {
    pub net: NetId,
    pub at: Point,
    /// Inclusive copper endpoints, in stack order. Supports through, blind and buried vias.
    pub start_layer: LayerId,
    pub end_layer: LayerId,
    pub drill_mm: f64,
    pub diameter_mm: f64,
    pub tented_top: bool,
    pub tented_bottom: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Pour {
    pub net: NetId,
    pub layer: LayerId,
    pub boundary: Polygon,
    pub clearance_mm: f64,
    pub min_neck_mm: f64,
    pub priority: i32,
    pub thermal: ThermalRelief,
    // Boundary is design intent only. A fill engine must generate and validate actual copper.
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ThermalRelief {
    pub spoke_count: u8,
    pub spoke_width_mm: f64,
    pub gap_mm: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct System {
    pub name: String,
    /// None for a root system; another system creates a subsystem.
    #[serde(default)]
    pub parent: Option<SystemId>,
    #[serde(default)]
    pub ports: BTreeMap<String, Port>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Port {
    pub name: String,
    pub net: NetId,
    pub direction: PortDirection,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PortDirection {
    Input,
    Output,
    Bidirectional,
    Power,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SchematicView {
    pub name: String,
    #[serde(default)]
    pub system: Option<SystemId>,
    #[serde(default)]
    pub symbols: BTreeMap<ComponentId, Symbol>,
    #[serde(default)]
    pub junctions: BTreeMap<String, Junction>,
    #[serde(default)]
    pub wires: BTreeMap<WireId, Wire>,
    #[serde(default)]
    pub labels: Vec<NetLabel>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Symbol {
    pub at: Point,
    pub rotation_rad: f64,
    /// Pin locations relative to symbol origin. Allows split symbols across views.
    pub pin_locations: BTreeMap<String, Point>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Junction {
    pub at: Point,
    pub net: NetId,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WireEnd {
    Pin { pin: PinRef },
    Junction { junction: String },
    Port { system: SystemId, port: String },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Wire {
    pub net: NetId,
    pub from: WireEnd,
    pub to: WireEnd,
    /// Drawing path only. Crossing paths never create implicit connectivity.
    pub points: Vec<Point>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NetLabel {
    pub net: NetId,
    pub text: String,
    pub at: Point,
}
