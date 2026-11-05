//! Original RC input-filter example with an IC and connector. A model fixture, not a qualified circuit.
use crate::*;
use std::collections::{BTreeMap, BTreeSet};

fn pins(names: &[(&str, PinType)]) -> BTreeMap<String, Pin> {
    names
        .iter()
        .enumerate()
        .map(|(i, (name, kind))| ((i + 1).to_string(), Pin { name: (*name).into(), electrical_type: *kind, no_connect: false }))
        .collect()
}
fn pref(c: &str, p: &str) -> PinRef {
    PinRef { component: c.into(), pin: p.into() }
}
fn polygon(w: f64, h: f64) -> Polygon {
    Polygon { outer: vec![[0., 0.], [w, 0.], [w, h], [0., h]], holes: vec![] }
}
pub fn rc_filter() -> Project {
    let mut p = Project::default();
    p.systems.insert("control".into(), System { name: "Control electronics".into(), parent: None, ports: BTreeMap::new() });
    p.systems.insert(
        "filter".into(),
        System {
            name: "Input filter".into(),
            parent: Some("control".into()),
            ports: BTreeMap::from([
                ("in".into(), Port { name: "Signal input".into(), net: "input".into(), direction: PortDirection::Input }),
                ("out".into(), Port { name: "Filtered output".into(), net: "filtered".into(), direction: PortDirection::Output }),
            ]),
        },
    );
    for (id, reference, kind, pins, system) in [
        (
            "resistor",
            "R1",
            ComponentKind::Resistor { resistance_ohm: 1000., tolerance_fraction: 0.01, power_rating_w: 0.1 },
            pins(&[("A", PinType::Passive), ("B", PinType::Passive)]),
            "filter",
        ),
        (
            "capacitor",
            "C1",
            ComponentKind::Capacitor { capacitance_f: 1e-7, tolerance_fraction: 0.1, voltage_rating_v: 16., polarized: false },
            pins(&[("A", PinType::Passive), ("B", PinType::Passive)]),
            "filter",
        ),
        (
            "buffer",
            "U1",
            ComponentKind::Ic { function: "Non-inverting buffer".into() },
            pins(&[("IN", PinType::Input), ("OUT", PinType::Output), ("VCC", PinType::PowerInput), ("GND", PinType::PowerInput)]),
            "control",
        ),
        (
            "connector",
            "J1",
            ComponentKind::Connector,
            pins(&[("IN", PinType::Passive), ("OUT", PinType::Passive), ("VCC", PinType::Passive), ("GND", PinType::Passive)]),
            "control",
        ),
    ] {
        p.components.insert(
            id.into(),
            Component {
                reference: reference.into(),
                kind,
                pins,
                system: Some(system.into()),
                manufacturer: None,
                manufacturer_part_number: None,
                annotations: BTreeMap::new(),
            },
        );
    }
    for (id, name, ends) in [
        ("input", "INPUT", vec![pref("connector", "1"), pref("resistor", "1")]),
        ("filtered", "FILTERED", vec![pref("resistor", "2"), pref("capacitor", "1"), pref("buffer", "1")]),
        ("output", "OUTPUT", vec![pref("buffer", "2"), pref("connector", "2")]),
        ("supply", "VCC", vec![pref("buffer", "3"), pref("connector", "3")]),
        ("ground", "GND", vec![pref("capacitor", "2"), pref("buffer", "4"), pref("connector", "4")]),
    ] {
        p.nets.insert(id.into(), Net { name: name.into(), pins: ends.into_iter().collect() });
    }
    p.netgroups.insert(
        "signals".into(),
        NetGroup {
            name: "Signals".into(),
            nets: ["input".into(), "filtered".into(), "output".into()].into(),
            min_trace_width_mm: Some(0.2),
            min_clearance_mm: Some(0.2),
            differential_pair: None,
        },
    );
    let mut b = Board {
        name: "RC input filter".into(),
        outline: polygon(40., 30.),
        stackup: vec![
            Layer {
                id: "top".into(),
                name: "Top copper".into(),
                kind: LayerKind::Copper,
                thickness_mm: 0.035,
                material: "Copper".into(),
                relative_permittivity: None,
            },
            Layer {
                id: "core".into(),
                name: "Core".into(),
                kind: LayerKind::Dielectric,
                thickness_mm: 1.53,
                material: "FR-4 (unspecified grade)".into(),
                relative_permittivity: Some(4.3),
            },
            Layer {
                id: "bottom".into(),
                name: "Bottom copper".into(),
                kind: LayerKind::Copper,
                thickness_mm: 0.035,
                material: "Copper".into(),
                relative_permittivity: None,
            },
        ],
        rules: BoardRules::default(),
        frame: BoardFrame::default(),
        mechanical_component: None,
        placements: BTreeMap::new(),
        pads: BTreeMap::new(),
        traces: BTreeMap::new(),
        vias: BTreeMap::new(),
        pours: BTreeMap::new(),
    };
    for (c, x, y) in [("resistor", 10., 10.), ("capacitor", 14., 12.), ("buffer", 22., 10.), ("connector", 4., 10.)] {
        b.placements.insert(
            c.into(),
            Placement {
                at: [x, y],
                rotation_rad: 0.,
                side: Side::Top,
                footprint: format!("example-{c}"),
                populated: true,
                mechanical_component: None,
            },
        );
        if let Some(component) = p.components.get(&ComponentId::from(c)) {
            for (i, n) in component.pins.keys().enumerate() {
                b.pads.insert(
                    PadId(format!("{c}-{n}")),
                    Pad {
                        at: [x + i as f64 * 2., y],
                        rotation_rad: 0.,
                        shape: PadShape::Rectangle { width_mm: 1., height_mm: 1.5 },
                        layers: BTreeSet::from(["top".into()]),
                        drill: None,
                        connection: Connection::Pin { pin: pref(c, n) },
                        solder_mask_expansion_mm: 0.05,
                        paste_margin_mm: -0.05,
                    },
                );
            }
        }
    }
    b.traces.insert("input".into(), Trace { net: "input".into(), layer: "top".into(), points: vec![[4., 10.], [10., 10.]], width_mm: 0.25 });
    b.traces.insert("filter".into(), Trace { net: "filtered".into(), layer: "top".into(), points: vec![[12., 10.], [14., 12.]], width_mm: 0.25 });
    b.vias.insert(
        "ground".into(),
        Via {
            net: "ground".into(),
            at: [16., 14.],
            start_layer: "top".into(),
            end_layer: "bottom".into(),
            drill_mm: 0.3,
            diameter_mm: 0.65,
            tented_top: true,
            tented_bottom: true,
        },
    );
    b.pours.insert(
        "ground".into(),
        Pour {
            net: "ground".into(),
            layer: "bottom".into(),
            boundary: Polygon { outer: vec![[1., 1.], [39., 1.], [39., 29.], [1., 29.]], holes: vec![] },
            clearance_mm: 0.2,
            min_neck_mm: 0.25,
            priority: 0,
            thermal: ThermalRelief { spoke_count: 4, spoke_width_mm: 0.25, gap_mm: 0.2 },
        },
    );
    p.boards.insert("filter-board".into(), b);
    let symbols = [("resistor", [10., 10.]), ("capacitor", [20., 10.])]
        .into_iter()
        .map(|(id, at)| {
            (id.into(), Symbol { at, rotation_rad: 0., pin_locations: BTreeMap::from([("1".into(), [-2., 0.]), ("2".into(), [2., 0.])]) })
        })
        .collect();
    p.schematics.insert(
        "filter".into(),
        SchematicView {
            name: "RC filter".into(),
            system: Some("filter".into()),
            symbols,
            junctions: BTreeMap::new(),
            wires: BTreeMap::from([(
                "filtered".into(),
                Wire {
                    net: "filtered".into(),
                    from: WireEnd::Pin { pin: pref("resistor", "2") },
                    to: WireEnd::Pin { pin: pref("capacitor", "1") },
                    points: vec![[12., 10.], [18., 10.]],
                },
            )]),
            labels: vec![NetLabel { net: "filtered".into(), text: "FILTERED".into(), at: [15., 10.] }],
        },
    );
    p
}
