use crate::geometry::{number, path, point};
use crate::*;
use std::collections::{BTreeMap, BTreeSet};

const MAX_ITEMS: usize = 100_000;
fn require(ok: bool, why: &str) -> Result<()> {
    if ok { Ok(()) } else { Err(Error(why.into())) }
}
fn text(s: &str) -> Result<()> {
    require(
        !s.trim().is_empty() && s.len() <= 256 && !s.chars().any(char::is_control),
        "text must be nonempty, <=256 bytes and contain no control characters",
    )
}
fn id(s: &str) -> Result<()> {
    require(!s.is_empty() && s.len() <= 128 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"_-.:".contains(&b)), "invalid stable identifier")
}
fn unique_names<'a>(names: impl Iterator<Item = &'a str>) -> Result<()> {
    let mut seen = BTreeSet::new();
    for name in names {
        text(name)?;
        require(seen.insert(name), "duplicate name/reference")?;
    }
    Ok(())
}
fn positive(v: f64, what: &str) -> Result<()> {
    number(v, 1e-18, 1e12, what)
}
fn length(v: f64) -> Result<()> {
    number(v, 1e-6, 1e6, "length (mm)")
}
fn angle(v: f64) -> Result<()> {
    number(v, -1e6, 1e6, "rotation (radians)")
}
fn component_kind(k: &ComponentKind) -> Result<()> {
    match k {
        ComponentKind::Resistor { resistance_ohm, tolerance_fraction, power_rating_w } => {
            number(*resistance_ohm, 0., 1e12, "resistance")?;
            number(*tolerance_fraction, 0., 1., "tolerance")?;
            positive(*power_rating_w, "power rating")?;
        }
        ComponentKind::Capacitor { capacitance_f, tolerance_fraction, voltage_rating_v, .. } => {
            positive(*capacitance_f, "capacitance")?;
            number(*tolerance_fraction, 0., 1., "tolerance")?;
            positive(*voltage_rating_v, "voltage rating")?;
        }
        ComponentKind::Inductor { inductance_h, current_rating_a, resistance_ohm } => {
            positive(*inductance_h, "inductance")?;
            positive(*current_rating_a, "current rating")?;
            number(*resistance_ohm, 0., 1e12, "resistance")?;
        }
        ComponentKind::Ic { function } | ComponentKind::Discrete { function } => text(function)?,
        ComponentKind::Other { description } => text(description)?,
        ComponentKind::Connector | ComponentKind::TestPoint => {}
    }
    Ok(())
}
fn pin<'a>(p: &'a Project, r: &PinRef) -> Result<&'a Pin> {
    p.components.get(&r.component).and_then(|c| c.pins.get(&r.pin)).ok_or_else(|| Error(format!("unknown pin {}:{}", r.component.0, r.pin)))
}
fn net(p: &Project, n: &NetId) -> Result<()> {
    require(p.nets.contains_key(n), "unknown net")
}
fn copper<'a>(b: &'a Board, l: &LayerId) -> Result<(usize, &'a Layer)> {
    b.stackup
        .iter()
        .enumerate()
        .find(|(_, v)| &v.id == l && v.kind == LayerKind::Copper)
        .ok_or_else(|| Error("expected a copper layer on this board".into()))
}
fn frame(f: &BoardFrame) -> Result<()> {
    for v in f.origin_mm {
        number(v, -1e6, 1e6, "board origin")?;
    }
    for axis in [f.x_axis, f.y_axis] {
        for v in axis {
            number(v, -1., 1., "board axis")?;
        }
        require((axis.iter().map(|v| v * v).sum::<f64>() - 1.).abs() < 1e-9, "board axes must be unit vectors")?;
    }
    require(f.x_axis.iter().zip(f.y_axis).map(|(a, b)| a * b).sum::<f64>().abs() < 1e-9, "board axes must be perpendicular")
}
fn shape(s: &PadShape) -> Result<(f64, f64)> {
    let (w, h) = match s {
        PadShape::Circle { diameter_mm } => (*diameter_mm, *diameter_mm),
        PadShape::Rectangle { width_mm, height_mm } => (*width_mm, *height_mm),
        PadShape::RoundedRectangle { width_mm, height_mm, corner_radius_mm } => {
            number(*corner_radius_mm, 0., width_mm.min(*height_mm) / 2., "pad corner radius")?;
            (*width_mm, *height_mm)
        }
    };
    length(w)?;
    length(h)?;
    Ok((w, h))
}

// Count nested entities and geometry before traversals, including programmatically constructed models.
pub(crate) fn budget(p: &Project) -> Result<()> {
    let mut count = 0usize;
    let mut add = |n: usize| -> Result<()> {
        count = count.saturating_add(n);
        require(count <= MAX_ITEMS, "electronics model exceeds 100000 entity/geometry budget")
    };
    for n in [p.boards.len(), p.components.len(), p.nets.len(), p.netgroups.len(), p.systems.len(), p.schematics.len()] {
        add(n)?;
    }
    for c in p.components.values() {
        add(c.pins.len())?;
        add(c.annotations.len())?;
    }
    for n in p.nets.values() {
        add(n.pins.len())?;
    }
    for g in p.netgroups.values() {
        add(g.nets.len())?;
    }
    for s in p.systems.values() {
        add(s.ports.len())?;
    }
    for b in p.boards.values() {
        for n in [b.stackup.len(), b.placements.len(), b.pads.len(), b.traces.len(), b.vias.len(), b.pours.len(), b.outline.outer.len()] {
            add(n)?;
        }
        for h in &b.outline.holes {
            add(h.len())?;
        }
        for pad in b.pads.values() {
            add(pad.layers.len())?;
        }
        for t in b.traces.values() {
            add(t.points.len())?;
        }
        for z in b.pours.values() {
            add(z.boundary.outer.len())?;
            for h in &z.boundary.holes {
                add(h.len())?;
            }
        }
    }
    for s in p.schematics.values() {
        for n in [s.symbols.len(), s.junctions.len(), s.wires.len(), s.labels.len()] {
            add(n)?;
        }
        for symbol in s.symbols.values() {
            add(symbol.pin_locations.len())?;
        }
        for w in s.wires.values() {
            add(w.points.len())?;
        }
    }
    Ok(())
}

pub(crate) fn project(p: &Project) -> Result<()> {
    require(p.schema_version == 1, "unsupported electronics schema version")?;
    budget(p)?;
    unique_names(p.nets.values().map(|n| n.name.as_str()))?;
    for (k, c) in &p.components {
        id(&k.0)?;
        text(&c.reference)?;
        component_kind(&c.kind)?;
        if let Some(s) = &c.system {
            require(p.systems.contains_key(s), "unknown component system")?;
        }
        for t in [&c.manufacturer, &c.manufacturer_part_number].into_iter().flatten() {
            text(t)?;
        }
        require(c.pins.len() <= 4096, "too many pins on component")?;
        for (n, pin) in &c.pins {
            id(n)?;
            text(&pin.name)?;
        }
        for (k, v) in &c.annotations {
            text(k)?;
            text(v)?;
        }
    }
    let mut memberships = BTreeMap::new();
    for (k, n) in &p.nets {
        id(&k.0)?;
        for r in &n.pins {
            require(!pin(p, r)?.no_connect, "intentional no-connect pin also assigned to net")?;
            require(memberships.insert(r, k).is_none(), "pin belongs to more than one net")?;
        }
    }
    for (k, g) in &p.netgroups {
        id(&k.0)?;
        text(&g.name)?;
        for n in &g.nets {
            net(p, n)?;
        }
        if let Some(v) = g.min_trace_width_mm {
            length(v)?;
        }
        if let Some(v) = g.min_clearance_mm {
            length(v)?;
        }
        if let Some([a, b]) = &g.differential_pair {
            require(a != b && g.nets.contains(a) && g.nets.contains(b), "differential pair needs two distinct group nets")?;
        }
    }
    systems(p)?;
    let mut placed = BTreeSet::new();
    for (k, b) in &p.boards {
        id(&k.0)?;
        board(p, b)?;
        for c in b.placements.keys() {
            require(placed.insert(c), "component placed on multiple boards; create another instance")?;
        }
    }
    for (k, v) in &p.schematics {
        id(&k.0)?;
        schematic(p, v, &memberships)?;
    }
    Ok(())
}
fn systems(p: &Project) -> Result<()> {
    // Memoized depths keep traversal iterative and enforce the limit even when ancestors
    // were already visited in a different map order.
    let mut depths: BTreeMap<&SystemId, usize> = BTreeMap::new();
    for (k, s) in &p.systems {
        id(&k.0)?;
        text(&s.name)?;
        for (n, port) in &s.ports {
            id(n)?;
            text(&port.name)?;
            net(p, &port.net)?;
        }
        let mut seen = BTreeSet::new();
        let mut chain = Vec::new();
        let mut current = Some(k);
        let mut base_depth = 0;
        while let Some(at) = current {
            if let Some(depth) = depths.get(at) {
                base_depth = *depth;
                break;
            }
            require(seen.insert(at), "system hierarchy contains a cycle")?;
            chain.push(at);
            require(chain.len() <= 256, "system hierarchy exceeds 256 levels")?;
            current = p.systems.get(at).ok_or_else(|| Error("unknown parent system".into()))?.parent.as_ref();
        }
        for at in chain.into_iter().rev() {
            base_depth += 1;
            require(base_depth <= 256, "system hierarchy exceeds 256 levels")?;
            depths.insert(at, base_depth);
        }
    }
    Ok(())
}

fn board(p: &Project, b: &Board) -> Result<()> {
    unique_names(b.placements.keys().filter_map(|id| p.components.get(id).map(|c| c.reference.as_str())))?;
    text(&b.name)?;
    b.outline.validate()?;
    frame(&b.frame)?;
    require((2..=128).contains(&b.stackup.len()), "board needs 2..128 stackup layers")?;
    let mut layers = BTreeSet::new();
    for l in &b.stackup {
        id(&l.id.0)?;
        require(layers.insert(&l.id), "duplicate layer id")?;
        text(&l.name)?;
        text(&l.material)?;
        number(l.thickness_mm, 0., 1000., "layer thickness")?;
        if matches!(l.kind, LayerKind::Copper | LayerKind::Dielectric) {
            length(l.thickness_mm)?;
        }
        if let Some(v) = l.relative_permittivity {
            number(v, 1., 1e6, "relative permittivity")?;
        }
    }
    require(b.stackup.iter().any(|l| l.kind == LayerKind::Copper), "board has no copper layer")?;
    for v in [b.rules.min_trace_width_mm, b.rules.min_clearance_mm, b.rules.min_drill_mm, b.rules.min_annular_ring_mm] {
        length(v)?;
    }
    for (k, v) in &b.placements {
        require(p.components.contains_key(k), "unknown placed component")?;
        point(v.at)?;
        angle(v.rotation_rad)?;
        text(&v.footprint)?;
    }
    for (k, v) in &b.pads {
        id(&k.0)?;
        point(v.at)?;
        angle(v.rotation_rad)?;
        let (w, h) = shape(&v.shape)?;
        for l in &v.layers {
            copper(b, l)?;
        }
        match &v.connection {
            Connection::Pin { pin: r } => {
                pin(p, r)?;
                require(b.placements.contains_key(&r.component), "pad pin component is not placed on this board")?;
            }
            Connection::Net { net: n } => net(p, n)?,
            Connection::None => {}
        }
        if let Some(d) = &v.drill {
            length(d.diameter_mm)?;
            number(d.slot_length_mm, 0., 1e6, "slot length")?;
            require(!d.plated || v.layers.len() >= 2, "plated through-hole pad needs at least two copper layers")?;
            require(d.plated || v.layers.is_empty(), "non-plated hole cannot carry copper pads")?;
            if d.plated {
                require(d.diameter_mm < h.min(w - d.slot_length_mm), "drill does not fit inside pad")?;
            }
        } else {
            require(v.layers.len() == 1, "surface pad needs exactly one copper layer")?;
        }
        require(!v.layers.is_empty() || matches!(v.connection, Connection::None), "non-copper hole cannot carry an electrical connection")?;
        number(v.solder_mask_expansion_mm, -1e3, 1e3, "mask expansion")?;
        number(v.paste_margin_mm, -1e3, 1e3, "paste margin")?;
        require(w + 2. * v.solder_mask_expansion_mm > 0. && h + 2. * v.solder_mask_expansion_mm > 0., "mask opening collapses")?;
        require(w + 2. * v.paste_margin_mm > 0. && h + 2. * v.paste_margin_mm > 0., "paste opening collapses")?;
    }
    for (k, t) in &b.traces {
        id(&k.0)?;
        net(p, &t.net)?;
        copper(b, &t.layer)?;
        length(t.width_mm)?;
        path(&t.points)?;
    }
    for (k, v) in &b.vias {
        id(&k.0)?;
        net(p, &v.net)?;
        point(v.at)?;
        let (a, _) = copper(b, &v.start_layer)?;
        let (z, _) = copper(b, &v.end_layer)?;
        require(a < z, "via span must have distinct copper endpoints in stack order")?;
        length(v.drill_mm)?;
        length(v.diameter_mm)?;
        require(v.diameter_mm > v.drill_mm, "via drill exceeds pad diameter")?;
    }
    for (k, v) in &b.pours {
        id(&k.0)?;
        net(p, &v.net)?;
        copper(b, &v.layer)?;
        v.boundary.validate()?;
        length(v.clearance_mm)?;
        length(v.min_neck_mm)?;
        require(v.thermal.spoke_count >= 2 && v.thermal.spoke_count <= 16, "thermal relief needs 2..16 spokes")?;
        length(v.thermal.spoke_width_mm)?;
        length(v.thermal.gap_mm)?;
    }
    Ok(())
}
fn end_net<'a>(p: &'a Project, v: &'a SchematicView, e: &'a WireEnd, m: &BTreeMap<&PinRef, &'a NetId>) -> Result<&'a NetId> {
    match e {
        WireEnd::Pin { pin: r } => {
            pin(p, r)?;
            require(v.symbols.get(&r.component).is_some_and(|s| s.pin_locations.contains_key(&r.pin)), "wire pin is not drawn in this view")?;
            m.get(r).copied().ok_or_else(|| Error("wire endpoint pin has no net".into()))
        }
        WireEnd::Junction { junction } => v.junctions.get(junction).map(|j| &j.net).ok_or_else(|| Error("unknown schematic junction".into())),
        WireEnd::Port { system, port } => {
            p.systems.get(system).and_then(|s| s.ports.get(port)).map(|p| &p.net).ok_or_else(|| Error("unknown system port".into()))
        }
    }
}
fn schematic(p: &Project, v: &SchematicView, m: &BTreeMap<&PinRef, &NetId>) -> Result<()> {
    text(&v.name)?;
    if let Some(s) = &v.system {
        require(p.systems.contains_key(s), "unknown schematic system")?;
    }
    for (k, s) in &v.symbols {
        let c = p.components.get(k).ok_or_else(|| Error("unknown symbol component".into()))?;
        point(s.at)?;
        angle(s.rotation_rad)?;
        for (n, &pos) in &s.pin_locations {
            require(c.pins.contains_key(n), "unknown symbol pin")?;
            point(pos)?;
        }
    }
    for (k, j) in &v.junctions {
        id(k)?;
        point(j.at)?;
        net(p, &j.net)?;
    }
    for (k, w) in &v.wires {
        id(&k.0)?;
        net(p, &w.net)?;
        path(&w.points)?;
        require(end_net(p, v, &w.from, m)? == &w.net && end_net(p, v, &w.to, m)? == &w.net, "wire endpoints disagree with logical net")?;
    }
    for l in &v.labels {
        text(&l.text)?;
        net(p, &l.net)?;
        point(l.at)?;
    }
    Ok(())
}
