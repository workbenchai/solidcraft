use super::*;
use std::collections::BTreeSet;

fn example() -> Project {
    example::rc_filter()
}
fn board_mut(p: &mut Project) -> &mut Board {
    p.boards.values_mut().next().unwrap()
}
fn pref(c: &str, n: &str) -> PinRef {
    PinRef { component: c.into(), pin: n.into() }
}

#[test]
fn example_round_trips_all_domains_and_resolves_pads() {
    let p = example();
    p.validate().unwrap();
    let json = serde_json::to_string(&p).unwrap();
    let reopened: Project = serde_json::from_str(&json).unwrap();
    assert_eq!(p, reopened);
    assert_eq!(p.fingerprint().unwrap(), reopened.fingerprint().unwrap());
    let pad = p.boards.values().next().unwrap().pads.get(&PadId::from("resistor-2")).unwrap();
    assert_eq!(p.pad_net(pad), Some(&NetId::from("filtered")));
    assert_eq!(p.pin_net(&pref("resistor", "2")), Some(&NetId::from("filtered")));
    assert_eq!(p.systems.get(&SystemId::from("filter")).unwrap().parent, Some("control".into()));
    assert_eq!(p.boards.values().next().unwrap().traces.len(), 2);
}
#[test]
fn no_connect_and_multinet_membership_are_exclusive() {
    let mut p = example();
    p.nets.get_mut(&NetId::from("input")).unwrap().pins.insert(pref("resistor", "2"));
    assert!(p.validate().unwrap_err().to_string().contains("more than one net"));
    let mut p = example();
    p.components.get_mut(&ComponentId::from("resistor")).unwrap().pins.get_mut("1").unwrap().no_connect = true;
    assert!(p.validate().unwrap_err().to_string().contains("no-connect"));
}
#[test]
fn references_are_checked_across_every_domain() {
    let mutations: [fn(&mut Project); 6] = [
        |p| {
            p.components.remove(&ComponentId::from("buffer"));
        },
        |p| {
            board_mut(p).traces.values_mut().next().unwrap().net = "missing".into();
        },
        |p| {
            board_mut(p).traces.values_mut().next().unwrap().layer = "core".into();
        },
        |p| {
            board_mut(p).placements.remove(&ComponentId::from("resistor"));
        },
        |p| {
            p.schematics.values_mut().next().unwrap().symbols.clear();
        },
        |p| {
            p.systems.get_mut(&SystemId::from("filter")).unwrap().parent = Some("missing".into());
        },
    ];
    for change in mutations {
        let mut p = example();
        change(&mut p);
        assert!(p.validate().is_err());
    }
}
#[test]
fn duplicate_placement_and_reference_are_refused() {
    let mut p = example();
    let b = p.boards.values().next().unwrap().clone();
    p.boards.insert("other".into(), b);
    assert!(p.validate().unwrap_err().to_string().contains("multiple boards"));
    let mut p = example();
    p.components.get_mut(&ComponentId::from("buffer")).unwrap().reference = "R1".into();
    assert!(p.validate().is_err());
}
#[test]
fn hierarchy_cycles_are_refused_without_recursion() {
    let mut p = example();
    p.systems.get_mut(&SystemId::from("control")).unwrap().parent = Some("filter".into());
    assert!(p.validate().unwrap_err().to_string().contains("cycle"));
}
#[test]
fn changing_wire_net_does_not_rewire_the_circuit() {
    let mut p = example();
    p.schematics.values_mut().next().unwrap().wires.values_mut().next().unwrap().net = "input".into();
    assert!(p.validate().unwrap_err().to_string().contains("endpoints disagree"));
}
#[test]
fn blind_and_buried_vias_use_explicit_stack_order() {
    let mut p = example();
    let b = board_mut(&mut p);
    b.stackup.insert(
        2,
        Layer {
            id: "inner".into(),
            name: "Inner".into(),
            kind: LayerKind::Copper,
            thickness_mm: 0.035,
            material: "Copper".into(),
            relative_permittivity: None,
        },
    );
    b.vias.values_mut().next().unwrap().end_layer = "inner".into();
    p.validate().unwrap();
    let b = board_mut(&mut p);
    b.vias.values_mut().next().unwrap().start_layer = "inner".into();
    assert!(p.validate().is_err());
}
#[test]
fn nonfinite_values_and_invalid_passives_are_refused() {
    let mutations: [fn(&mut Project); 4] = [
        |p| {
            board_mut(p).pads.values_mut().next().unwrap().at[0] = f64::NAN;
        },
        |p| {
            board_mut(p).frame.x_axis = [0., 0., 0.];
        },
        |p| {
            board_mut(p).traces.values_mut().next().unwrap().width_mm = f64::INFINITY;
        },
        |p| {
            p.components.get_mut(&ComponentId::from("capacitor")).unwrap().kind =
                ComponentKind::Capacitor { capacitance_f: -1., tolerance_fraction: 0.1, voltage_rating_v: 16., polarized: false };
        },
    ];
    for change in mutations {
        let mut p = example();
        change(&mut p);
        assert!(p.validate().is_err());
    }
}
#[test]
fn polygon_rejects_crossings_and_overlapping_holes() {
    let mut b = board_mut(&mut example()).clone();
    b.outline.outer = vec![[0., 0.], [4., 4.], [0., 4.], [4., 0.]];
    assert!(b.outline.validate().is_err());
    b.outline.outer = vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]];
    b.outline.holes = vec![vec![[1., 1.], [5., 1.], [5., 5.], [1., 5.]], vec![[4., 4.], [7., 4.], [7., 7.], [4., 7.]]];
    assert!(b.outline.validate().is_err());
    b.outline.holes.pop();
    b.outline.validate().unwrap();
    assert!(!b.outline.contains([2., 2.]));
    assert!(b.outline.contains([8., 8.]));
}
#[test]
fn erc_and_dimensions_do_not_claim_routing_or_manufacturing_success() {
    let mut p = example();
    board_mut(&mut p).traces.values_mut().next().unwrap().width_mm = 0.1;
    let report = p.check().unwrap();
    assert!(report.findings.iter().any(|f| f.code == "trace_width"));
    assert!(report.findings.iter().any(|f| f.code == "pour_unfilled"));
    assert!(report.not_performed.iter().any(|s| s == "copper_connectivity_and_unrouted_detection"));
    assert!(!report.manufacturing_ready);
    // A stronger netgroup constraint wins over the board's weaker minimum.
    board_mut(&mut p).traces.values_mut().next().unwrap().width_mm = 0.175;
    assert!(p.check().unwrap().findings.iter().any(|f| f.code == "trace_width"));
}
#[test]
fn multiple_push_pull_drivers_are_reported() {
    let mut p = example();
    p.components.get_mut(&ComponentId::from("connector")).unwrap().pins.get_mut("2").unwrap().electrical_type = PinType::Output;
    assert!(p.check().unwrap().findings.iter().any(|f| f.code == "multiple_drivers"));
}
#[test]
fn transactions_are_atomic_and_can_add_related_nodes_together() {
    let p = example();
    let original = p.fingerprint().unwrap();
    assert!(p.apply(&[Edit::RemoveNet { id: "ground".into() }]).is_err());
    assert_eq!(p.fingerprint().unwrap(), original);
    let mut c = p.components.get(&ComponentId::from("resistor")).unwrap().clone();
    c.reference = "R2".into();
    let edited = p
        .apply(&[
            Edit::PutNet {
                id: "new-net".into(),
                net: Net { name: "NEW".into(), pins: BTreeSet::from([pref("new-resistor", "1"), pref("new-resistor", "2")]) },
            },
            Edit::PutComponent { id: "new-resistor".into(), component: c },
        ])
        .unwrap();
    assert_eq!(edited.components.len(), p.components.len() + 1);
    assert!(p.apply(&[]).is_err());
    assert!(p.apply(&[Edit::RemoveBoard { id: "missing".into() }]).is_err());
}
#[test]
fn unknown_fields_and_future_versions_do_not_silently_disappear() {
    let mut json = serde_json::to_value(example()).unwrap();
    json["schema_version"] = 2.into();
    assert!(serde_json::from_value::<Project>(json).unwrap().validate().is_err());
    let mut json = serde_json::to_value(example()).unwrap();
    json["nets_typo"] = serde_json::json!({});
    assert!(serde_json::from_value::<Project>(json).is_err());
}
#[test]
fn fingerprints_change_for_geometry_connectivity_and_hierarchy() {
    let p = example();
    let old = p.fingerprint().unwrap();
    let mutations: [fn(&mut Project); 3] = [
        |p| {
            board_mut(p).vias.values_mut().next().unwrap().at[0] += 1.;
        },
        |p| {
            p.nets.get_mut(&NetId::from("input")).unwrap().name = "RENAMED".into();
        },
        |p| {
            p.systems.get_mut(&SystemId::from("filter")).unwrap().parent = None;
        },
    ];
    for change in mutations {
        let mut p = p.clone();
        change(&mut p);
        assert_ne!(old, p.fingerprint().unwrap());
    }
}

#[test]
fn netgroup_pairs_and_plated_pad_geometry_are_checked() {
    let mut p = example();
    p.netgroups.values_mut().next().unwrap().differential_pair = Some(["input".into(), "input".into()]);
    assert!(p.validate().is_err());
    let mut p = example();
    let pad = board_mut(&mut p).pads.values_mut().next().unwrap();
    pad.drill = Some(Drill { diameter_mm: 0.3, slot_length_mm: 0., plated: true });
    assert!(p.validate().is_err()); // Through-hole needs two copper layers.
    let pad = board_mut(&mut p).pads.values_mut().next().unwrap();
    pad.layers.insert("bottom".into());
    p.validate().unwrap();
    board_mut(&mut p).pads.values_mut().next().unwrap().drill.as_mut().unwrap().diameter_mm = 2.;
    assert!(p.validate().is_err());
}
#[test]
fn deep_hierarchy_limit_is_independent_of_identifier_order() {
    let mut p = Project::default();
    for i in 0..257 {
        p.systems.insert(
            SystemId(format!("s{i:04}")),
            System { name: format!("System {i}"), parent: (i > 0).then(|| SystemId(format!("s{:04}", i - 1))), ports: Default::default() },
        );
    }
    assert!(p.validate().unwrap_err().to_string().contains("256"));
}
#[test]
fn geometry_budget_precedes_expensive_validation() {
    let mut p = example();
    board_mut(&mut p).traces.values_mut().next().unwrap().points = vec![[0., 0.]; 100_001];
    assert!(p.validate().unwrap_err().to_string().contains("budget"));
}

#[test]
fn placement_edits_transform_pads_and_preserve_trace_geometry() {
    let p = example();
    let old = p.boards.values().next().unwrap();
    let mut placement = old.placements.get(&ComponentId::from("resistor")).unwrap().clone();
    placement.at = [20., 20.];
    placement.rotation_rad = std::f64::consts::FRAC_PI_2;
    let moved = p.apply(&[Edit::PutPlacement { board: "filter-board".into(), component: "resistor".into(), placement }]).unwrap();
    let b = moved.boards.values().next().unwrap();
    let at = b.pads.get(&PadId::from("resistor-2")).unwrap().at;
    assert!((at[0] - 20.).abs() < 1e-9 && (at[1] - 22.).abs() < 1e-9);
    assert_eq!(old.traces, b.traces);
    let mut placement = old.placements.get(&ComponentId::from("resistor")).unwrap().clone();
    placement.side = Side::Bottom;
    let mirrored = p.apply(&[Edit::PutPlacement { board: "filter-board".into(), component: "resistor".into(), placement }]).unwrap();
    let at = mirrored.boards.values().next().unwrap().pads.get(&PadId::from("resistor-2")).unwrap().at;
    assert!((at[0] - 8.).abs() < 1e-9 && (at[1] - 10.).abs() < 1e-9);
    assert_eq!(mirrored.boards.values().next().unwrap().pads.get(&PadId::from("resistor-2")).unwrap().layers, BTreeSet::from(["bottom".into()]));
}

#[test]
fn separate_boards_can_reuse_reference_designators() {
    let mut p = example();
    let mut second = p.boards.values().next().unwrap().clone();
    let ids: Vec<_> = second.placements.keys().cloned().collect();
    for old in ids {
        let new = ComponentId(format!("second-{}", old.0));
        p.components.insert(new.clone(), p.components.get(&old).unwrap().clone());
        let placement = second.placements.remove(&old).unwrap();
        second.placements.insert(new.clone(), placement);
        for pad in second.pads.values_mut() {
            if let Connection::Pin { pin } = &mut pad.connection
                && pin.component == old
            {
                pin.component = new.clone();
            }
        }
        for net in p.nets.values_mut() {
            let pins: Vec<_> =
                net.pins.iter().filter(|pin| pin.component == old).map(|pin| PinRef { component: new.clone(), pin: pin.pin.clone() }).collect();
            net.pins.extend(pins);
        }
    }
    p.boards.insert("second".into(), second);
    p.validate().unwrap();
}
