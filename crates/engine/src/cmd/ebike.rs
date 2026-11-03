//! One native project spanning motor geometry, electronics, harness and firmware sources.
//! Ordinary feature commands build the assembly; all subsequent feature edits stay intact.
use super::CommandSpec;
use crate::params::bad;
use crate::schema::ebike::{Configure, Create, Inspect, Reply};
use crate::schema::{TypedContract, output_schema_for};
use crate::{Result, Session};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use solidcraft_doc::ebike::{Finding, Project};
use solidcraft_doc::expr::Kind;

const CREATE: TypedContract<Create> = TypedContract::new();
const CONFIGURE: TypedContract<Configure> = TypedContract::new();
const INSPECT: TypedContract<Inspect> = TypedContract::new();
pub static COMMANDS: &[CommandSpec] = &[
    CommandSpec::new("ebike.create", "Create E-bike Project", crate::checked!("ebike.create", create))
        .icon("component")
        .contract(&CREATE)
        .output(output_schema_for::<Reply>)
        .params("project?: complete project; requires an empty design; inserts editable frame, motor, housing, mount and board envelopes"),
    CommandSpec::new("ebike.configure", "Configure E-bike Project", crate::checked!("ebike.configure", configure))
        .icon("params")
        .doc_only()
        .contract(&CONFIGURE)
        .output(output_schema_for::<Reply>)
        .params("project, expected_revision: updates domain data and named geometry parameters atomically; retains hand-authored features"),
    CommandSpec::new("ebike.inspect", "E-bike Project", crate::checked!("ebike.inspect", inspect))
        .icon("settings")
        .doc_only()
        .noundo()
        .contract(&INSPECT)
        .output(output_schema_for::<Reply>)
        .params("→ project, revision, analytic estimate, consistency findings and unresolved validation gates"),
];
fn decode<T: serde::de::DeserializeOwned>(p: &Value, cmd: &str) -> Result<T> {
    serde_json::from_value(p.clone()).map_err(|e| bad(cmd, e.to_string()))
}
fn set_parameters(s: &mut Session, p: &Project) -> Result<()> {
    for (name, value) in p.geometry_parameters() {
        s.doc_mut().set_param(name, &format!("{value} mm"), Some("mm"), Some("E-bike project geometry"))?;
    }
    Ok(())
}
fn create(s: &mut Session, p: &Value) -> Result<Value> {
    let args: Create = decode(p, "ebike.create")?;
    if !s.doc.features.is_empty() || !s.doc.components.is_empty() || !s.doc.params.is_empty() || s.doc.ebike.is_some() {
        return Err(bad("ebike.create", "start an empty design first; existing work is never replaced"));
    }
    let mut project = args.project.unwrap_or_default();
    project.sync_firmware_config();
    project.validate()?;
    let mut next = s.scratch();
    next.doc_mut().name = "Pedal-assist motor and controller".into();
    next.doc_mut().units = "mm".into();
    set_parameters(&mut next, &project)?;
    next.doc_mut().ebike = Some(project);
    for (command, params) in geometry() {
        next.execute(command, &params)?;
    }
    next.execute("component.activate", &json!({"component":"root"}))?;
    let signature = feature_signature(&next)?;
    if let Some(project) = next.doc_mut().ebike.as_mut() {
        project.geometry_signature = signature;
    }
    next.undo.clear();
    s.accept_computed(next);
    changed_reply(s)
}
fn configure(s: &mut Session, p: &Value) -> Result<Value> {
    let mut args: Configure = decode(p, "ebike.configure")?;
    if s.doc.ebike.is_none() {
        return Err(bad("ebike.configure", "no e-bike project"));
    }
    if args.expected_revision != s.revision {
        return Err(bad("ebike.configure", "project changed since this editor opened; reload before applying"));
    }
    args.project.sync_firmware_config();
    args.project.geometry_signature = s.doc.ebike.as_ref().map(|p| p.geometry_signature.clone()).unwrap_or_default();
    args.project.validate()?;
    let mut next = s.scratch();
    set_parameters(&mut next, &args.project)?;
    next.doc_mut().ebike = Some(args.project);
    // Evaluate the scratch document before accepting it. A dimension that breaks an existing
    // user-authored feature refuses the entire configuration instead of losing that feature.
    next.refresh();
    if let Some(result) = next.model.results.iter().find(|r| r.error.is_some()) {
        return Err(bad("ebike.configure", format!("configuration breaks feature {}: {:?}", result.name, result.error)));
    }
    next.undo.clear();
    s.accept_computed(next);
    changed_reply(s)
}
// Session::execute refreshes once more after a successful mutation. Return that revision.
fn changed_reply(s: &mut Session) -> Result<Value> {
    let mut reply = inspect(s, &json!({}))?;
    reply["revision"] = json!(s.revision + 1);
    Ok(reply)
}

fn feature_signature(s: &Session) -> Result<String> {
    let bytes = serde_json::to_vec(&(&s.doc.features, &s.doc.occurrences, &s.doc.assembly)).map_err(|e| bad("ebike.inspect", e.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn inspect(s: &mut Session, _: &Value) -> Result<Value> {
    let project = s.doc.ebike.clone().ok_or_else(|| bad("ebike.inspect", "no e-bike project"))?;
    let estimate = project.estimate()?;
    let mut findings = Vec::new();
    if project.geometry_signature != feature_signature(s)? {
        findings.push(Finding::new(
            "geometry_changed",
            false,
            "Native feature definitions changed; revalidate all geometry-derived motor and fit assumptions",
        ));
    }
    for (name, wanted) in project.geometry_parameters() {
        let same = s.doc.eval(name, Kind::Length).is_ok_and(|n| (n - wanted).abs() < 1e-6);
        if !same {
            findings.push(Finding::new(
                name,
                false,
                "Native geometry parameter differs from the physics definition; reconcile before using estimates",
            ));
        }
    }
    for (id, explanation) in [
        ("donor_fit", "Generic steel diamond frame: measure the actual donor, inspect its condition and verify clamps, crank sweep and chainline"),
        (
            "magnetic_validation",
            "Flux linkage, resistance and inductance are assumptions; slot fill, saturation, torque ripple, losses and demagnetisation need field analysis and tests",
        ),
        (
            "mechanical_validation",
            "Reduction, freewheel, bearings, shaft, rotor retention, clamp loads, fatigue, guards and sealing need detailed design and validation",
        ),
        ("pcb_validation", "Controller is a block architecture and board envelope, not a pin-level schematic, routed PCB or fabrication output"),
        (
            "firmware_validation",
            "Embedded source is supervisory logic only; FOC, gate timing, current-loop tuning, sensor calibration and hardware fault paths are not commissioned",
        ),
        ("thermal_validation", "Thermal resistances and fixed losses are assumed; transient peak duty and cooling require simulation or measurement"),
    ] {
        findings.push(Finding::new(id, false, explanation));
    }
    serde_json::to_value(Reply { project, revision: s.revision, estimate, findings, param_warnings: Vec::new() })
        .map_err(|e| bad("ebike.inspect", e.to_string()))
}

fn geometry() -> Vec<(&'static str, Value)> {
    let mut commands = Vec::new();
    let mut add = |id, p| commands.push((id, p));
    add("component.create", json!({"name":"Donor steel frame","parent":"root"}));
    // Generic, original diamond-frame reference. These are fit-study centre lines, not a vendor frame.
    for (name, a, b, r) in [
        ("Seat tube", [0., 0., 280.], [-170., 0., 800.], 14.),
        ("Down tube", [0., 0., 280.], [500., 0., 650.], 17.),
        ("Top tube", [-170., 0., 800.], [470., 0., 800.], 14.),
        ("Head tube", [500., 0., 650.], [470., 0., 800.], 18.),
        ("Left chainstay", [0., -30., 280.], [-440., -65., 335.], 9.),
        ("Right chainstay", [0., 30., 280.], [-440., 65., 335.], 9.),
        ("Left seatstay", [-170., 0., 800.], [-440., -65., 335.], 8.),
        ("Right seatstay", [-170., 0., 800.], [-440., 65., 335.], 8.),
    ] {
        let delta: [f64; 3] = std::array::from_fn(|i| b[i] - a[i]);
        let length = delta.iter().map(|v| v * v).sum::<f64>().sqrt();
        add("solid.cylinder", json!({"base":a,"axis":delta,"radius":r,"height":length,"body_name":name}));
        add("solid.cylinder", json!({"base":a,"axis":delta,"radius":r-1.2,"height":length,"operation":"cut","targets":[name]}));
    }
    add(
        "solid.cylinder",
        json!({"base":[0,"-eb_bb_width/2",280],"axis":"Y","radius":"eb_bb_bore/2+3","height":"eb_bb_width","body_name":"Bottom bracket shell"}),
    );
    add(
        "solid.cylinder",
        json!({"base":[0,"-eb_bb_width/2",280],"axis":"Y","radius":"eb_bb_bore/2","height":"eb_bb_width","operation":"cut","targets":["Bottom bracket shell"]}),
    );
    add("component.create", json!({"name":"Wheel envelopes","parent":"root"}));
    for (name, x) in [("Rear wheel", -440), ("Front wheel", 650)] {
        add("solid.cylinder", json!({"base":[x,-15,335],"axis":"Y","radius":"eb_wheel_r","height":30,"body_name":name}));
        add("solid.cylinder", json!({"base":[x,-15,335],"axis":"Y","radius":"eb_wheel_r-20 mm","height":30,"operation":"cut","targets":[name]}));
    }
    // Motor axis is +Y; all named bodies are ordinary editable timeline features.
    add("component.create", json!({"name":"Stator 12 slot","parent":"root"}));
    add(
        "solid.cylinder",
        json!({"base":[120,"eb_chainline",350],"axis":"Y","radius":"eb_stator_r*0.7","height":"eb_stack","body_name":"Stator yoke"}),
    );
    add(
        "solid.cylinder",
        json!({"base":[120,"eb_chainline",350],"axis":"Y","radius":18,"height":"eb_stack","operation":"cut","targets":["Stator yoke"]}),
    );
    for i in 0..12 {
        let name = format!("Stator tooth {}", i + 1);
        add(
            "solid.box",
            json!({"corner":["eb_stator_r*0.68","eb_chainline",-3.5],"length":"eb_stator_r*0.32","width":"eb_stack","height":7,"body_name":name}),
        );
        add("solid.move", json!({"bodies":[name],"axis":[0,1,0],"angle":i*30,"translate":[120,0,350]}));
    }
    add("component.create", json!({"name":"Winding envelopes","parent":"root"}));
    for i in 0..12 {
        for (side, z) in [("a", -6.0), ("b", 3.5)] {
            let name = format!("Coil {} {side}", i + 1);
            add(
                "solid.box",
                json!({"corner":["eb_stator_r*0.72","eb_chainline-2 mm",z],"length":"eb_stator_r*0.2","width":"eb_stack+4 mm","height":2.5,"body_name":name}),
            );
            add("solid.move", json!({"bodies":[name],"axis":[0,1,0],"angle":i*30,"translate":[120,0,350]}));
        }
    }
    add("component.create", json!({"name":"Rotor 14 pole","parent":"root"}));
    add(
        "solid.cylinder",
        json!({"base":[120,"eb_chainline",350],"axis":"Y","radius":"eb_stator_r+eb_gap+eb_magnet_t+eb_iron_t+0.5 mm","height":"eb_stack","body_name":"Rotor back iron"}),
    );
    add(
        "solid.cylinder",
        json!({"base":[120,"eb_chainline",350],"axis":"Y","radius":"eb_stator_r+eb_gap+eb_magnet_t+0.5 mm","height":"eb_stack","operation":"cut","targets":["Rotor back iron"]}),
    );
    // Flat magnets are tangent to the specified cylindrical minimum air gap. Sector magnets are a later refinement.
    for i in 0..14 {
        let name = format!("Magnet {}", i + 1);
        add(
            "solid.box",
            json!({"corner":["eb_stator_r+eb_gap","eb_chainline",-4.5],"length":"eb_magnet_t","width":"eb_stack","height":9,"body_name":name}),
        );
        add("solid.move", json!({"bodies":[name],"axis":[0,1,0],"angle":f64::from(i)*360.0/14.0,"translate":[120,0,350]}));
    }
    add("component.create", json!({"name":"Shaft and bearing envelopes","parent":"root"}));
    add(
        "solid.cylinder",
        json!({"base":[120,"eb_chainline-10 mm",350],"axis":"Y","radius":"eb_shaft_r","height":"eb_stack+30 mm","body_name":"Motor shaft"}),
    );
    for (name, y) in [("Front bearing", "eb_chainline-6 mm"), ("Rear bearing", "eb_chainline+eb_stack")] {
        add("solid.cylinder", json!({"base":[120,y,350],"axis":"Y","radius":16,"height":6,"body_name":name}));
        add("solid.cylinder", json!({"base":[120,y,350],"axis":"Y","radius":"eb_shaft_r","height":6,"operation":"cut","targets":[name]}));
    }
    add("component.create", json!({"name":"Motor housing","parent":"root"}));
    add(
        "solid.cylinder",
        json!({"base":[120,"eb_chainline-8 mm",350],"axis":"Y","radius":"eb_stator_r+eb_gap+eb_magnet_t+eb_iron_t+5 mm","height":"eb_stack+16 mm","body_name":"Motor case"}),
    );
    add(
        "solid.cylinder",
        json!({"base":[120,"eb_chainline-8 mm",350],"axis":"Y","radius":"eb_stator_r+eb_gap+eb_magnet_t+eb_iron_t+2 mm","height":"eb_stack+16 mm","operation":"cut","targets":["Motor case"]}),
    );
    add("component.create", json!({"name":"Mount and reduction envelopes","parent":"root"}));
    add("solid.box", json!({"corner":[-20,"eb_chainline-14 mm",250],"length":190,"width":6,"height":35,"body_name":"Mount plate envelope"}));
    for (name, x, z, r, y) in [
        ("Stage one pulley", 120, 350, "12 mm", "eb_chainline+eb_stack+12 mm"),
        ("Intermediate pulley", 30, 410, "eb_stage_one_r", "eb_chainline+eb_stack+12 mm"),
        ("Stage two pinion", 30, 410, "12 mm", "eb_chainline+eb_stack+26 mm"),
        ("Crank reduction", 0, 280, "eb_stage_two_r", "eb_chainline+eb_stack+26 mm"),
    ] {
        add("solid.cylinder", json!({"base":[x,y,z],"axis":"Y","radius":r,"height":8,"body_name":name}));
    }
    add("component.create", json!({"name":"Controller PCB envelope","parent":"root"}));
    add("solid.box", json!({"corner":[75,-40,350],"length":"eb_pcb_l","width":"eb_pcb_t","height":"eb_pcb_w","body_name":"Controller board"}));
    add("component.create", json!({"name":"Controller housing envelope","parent":"root"}));
    add("solid.box", json!({"corner":[70,-55,345],"length":"eb_pcb_l+10 mm","width":25,"height":"eb_pcb_w+10 mm","body_name":"Controller housing"}));
    add(
        "solid.box",
        json!({"corner":[73,-52,348],"length":"eb_pcb_l+4 mm","width":19,"height":"eb_pcb_w+4 mm","operation":"cut","targets":["Controller housing"]}),
    );
    commands
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    #[test]
    fn create_edit_save_and_undo() {
        let mut s = Session::default();
        let created = s.execute("ebike.create", &json!({})).unwrap();
        assert_eq!(created["revision"].as_u64(), Some(s.revision));
        let yoke_volume = |s: &Session| {
            let world = s.world_state().unwrap();
            solidcraft_doc::kernel::measure(&world.body("Stator yoke").unwrap().body).unwrap().volume
        };
        let expected = std::f64::consts::PI * (28.0_f64.powi(2) - 18.0_f64.powi(2));
        assert!((yoke_volume(&s) - expected * 30.0).abs() < 1e-3);
        assert!(s.model.results.iter().all(|x| x.error.is_none()));
        assert!(s.doc.components.len() >= 9);
        let saved = s.doc.to_json();
        let reopened = solidcraft_doc::Document::from_json(&saved).unwrap();
        assert_eq!(s.doc.ebike, reopened.ebike);
        let mut p = s.doc.ebike.clone().unwrap();
        p.motor.stack_mm = 35.0;
        let rev = s.revision;
        s.execute("ebike.configure", &json!({"project":p,"expected_revision":rev})).unwrap();
        assert!((s.doc.eval("eb_stack", Kind::Length).unwrap() - 35.0).abs() < 1e-9);
        assert!((yoke_volume(&s) - expected * 35.0).abs() < 1e-3);
        assert!(s.execute("ebike.configure", &json!({"project":p,"expected_revision":rev})).is_err());
        s.execute("edit.undo", &json!({})).unwrap();
        assert_eq!(s.doc.ebike.as_ref().unwrap().motor.stack_mm, 30.0);
        assert!(s.execute("ebike.create", &json!({})).is_err());
    }
    #[test]
    fn geometry_parameter_edits_are_reported() {
        let mut s = Session::default();
        s.doc_mut().ebike = Some(Project::default());
        set_parameters(&mut s, &Project::default()).unwrap();
        s.doc_mut().set_param("eb_gap", "1 mm", Some("mm"), None).unwrap();
        let r = s.execute("ebike.inspect", &json!({})).unwrap();
        assert!(r["findings"].as_array().unwrap().iter().any(|x| x["id"] == "eb_gap"));
        assert_eq!(r["estimate"]["release_ready"], false);
    }
}
