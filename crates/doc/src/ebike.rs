//! Editable cross-domain pedal-assist project. SI electrical quantities; geometry in mm.
//! Analytical estimates are explicitly distinct from field simulation and physical verification.
use crate::{DocError, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Bike {
    pub donor_measured: bool,
    pub wheel_radius_mm: f64,
    pub bottom_bracket_width_mm: f64,
    pub bottom_bracket_bore_mm: f64,
    pub chainline_mm: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Motor {
    pub slots: u32,
    pub poles: u32,
    pub stator_radius_mm: f64,
    pub air_gap_mm: f64,
    pub magnet_thickness_mm: f64,
    pub back_iron_mm: f64,
    pub stack_mm: f64,
    pub shaft_radius_mm: f64,
    pub turns_per_tooth: u32,
    pub wire_diameter_mm: f64,
    /// Phase-to-neutral resistance at 20 C, including parallel paths. Assumed until measured.
    pub phase_resistance_20_ohm: f64,
    pub phase_inductance_h: f64,
    /// Peak per-phase permanent-magnet flux linkage, assumed until solved or measured.
    pub flux_linkage_wb: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Drive {
    pub stage_one_ratio: f64,
    pub stage_two_ratio: f64,
    pub reduction_efficiency: f64,
    pub cadence_rpm: f64,
    /// Mechanical assist power at the crank, not battery power.
    pub continuous_crank_w: f64,
    pub peak_crank_w: f64,
    pub assist_cutoff_kph: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Controller {
    pub bus_nominal_v: f64,
    pub bus_min_v: f64,
    pub bus_max_v: f64,
    pub battery_limit_a: f64,
    pub iq_peak_limit_a: f64,
    pub mosfet_rating_v: f64,
    pub pwm_hz: f64,
    pub pcb_length_mm: f64,
    pub pcb_width_mm: f64,
    pub pcb_thickness_mm: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Thermal {
    pub ambient_c: f64,
    pub winding_limit_c: f64,
    pub winding_to_case_k_per_w: f64,
    pub case_to_air_k_per_w: f64,
    pub core_loss_w: f64,
    pub controller_loss_w: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub schema_version: u32,
    /// Fingerprint of the generated feature definitions; edits invalidate geometry-derived assumptions.
    #[serde(default)]
    pub geometry_signature: String,
    pub bike: Bike,
    pub motor: Motor,
    pub drive: Drive,
    pub controller: Controller,
    pub thermal: Thermal,
    /// Embedded, editable design sources. Never executed by loading or checking a project.
    pub files: BTreeMap<String, String>,
}
impl Default for Project {
    fn default() -> Self {
        Self {
            schema_version: 1,
            geometry_signature: String::new(),
            bike: Bike {
                donor_measured: false,
                wheel_radius_mm: 335.0,
                bottom_bracket_width_mm: 68.0,
                bottom_bracket_bore_mm: 34.8,
                chainline_mm: 50.0,
            },
            motor: Motor {
                slots: 12,
                poles: 14,
                stator_radius_mm: 40.0,
                air_gap_mm: 0.7,
                magnet_thickness_mm: 3.0,
                back_iron_mm: 3.0,
                stack_mm: 30.0,
                shaft_radius_mm: 6.0,
                turns_per_tooth: 12,
                wire_diameter_mm: 1.0,
                phase_resistance_20_ohm: 0.06,
                phase_inductance_h: 0.0001,
                flux_linkage_wb: 0.008,
            },
            drive: Drive {
                stage_one_ratio: 5.0,
                stage_two_ratio: 6.0,
                reduction_efficiency: 0.9,
                cadence_rpm: 90.0,
                continuous_crank_w: 500.0,
                peak_crank_w: 750.0,
                assist_cutoff_kph: 25.0,
            },
            controller: Controller {
                bus_nominal_v: 48.0,
                bus_min_v: 39.0,
                bus_max_v: 54.6,
                battery_limit_a: 25.0,
                iq_peak_limit_a: 40.0,
                mosfet_rating_v: 100.0,
                pwm_hz: 20000.0,
                pcb_length_mm: 90.0,
                pcb_width_mm: 60.0,
                pcb_thickness_mm: 1.6,
            },
            thermal: Thermal {
                ambient_c: 25.0,
                winding_limit_c: 100.0,
                winding_to_case_k_per_w: 0.4,
                case_to_air_k_per_w: 0.35,
                core_loss_w: 12.0,
                controller_loss_w: 8.0,
            },
            files: [
                ("controller/architecture.json".into(), include_str!("../../../examples/ebike/controller.json").into()),
                ("wiring/harness.json".into(), include_str!("../../../examples/ebike/wiring.json").into()),
                ("firmware/control.rs".into(), include_str!("../../../examples/ebike/firmware/control.rs").into()),
            ]
            .into_iter()
            .collect(),
        }
    }
}
fn invalid(message: impl Into<String>) -> DocError {
    DocError::Invalid(message.into())
}
fn range(name: &str, n: f64, lo: f64, hi: f64) -> Result<()> {
    if !n.is_finite() || n < lo || n > hi {
        return Err(invalid(format!("{name} must be finite and in {lo}..={hi}")));
    }
    Ok(())
}
impl Project {
    pub fn validate(&self) -> Result<()> {
        if self.geometry_signature.len() > 64 || !self.geometry_signature.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(invalid("invalid geometry fingerprint"));
        }
        if self.schema_version != 1 {
            return Err(invalid("unsupported e-bike schema_version"));
        }
        let (m, d, c, t, b) = (&self.motor, &self.drive, &self.controller, &self.thermal, &self.bike);
        if m.slots != 12 || m.poles != 14 {
            return Err(invalid("first motor template supports 12 slots / 14 poles only"));
        }
        for (name, n, lo, hi) in [
            ("wheel radius", b.wheel_radius_mm, 250.0, 400.0),
            ("BB width", b.bottom_bracket_width_mm, 60.0, 100.0),
            ("BB bore", b.bottom_bracket_bore_mm, 25.0, 55.0),
            ("chainline", b.chainline_mm, 35.0, 80.0),
            ("stator radius", m.stator_radius_mm, 30.0, 65.0),
            ("air gap", m.air_gap_mm, 0.3, 2.0),
            ("magnet thickness", m.magnet_thickness_mm, 1.0, 8.0),
            ("back iron", m.back_iron_mm, 1.0, 8.0),
            ("stack", m.stack_mm, 15.0, 60.0),
            ("shaft radius", m.shaft_radius_mm, 4.0, 12.0),
            ("wire diameter", m.wire_diameter_mm, 0.2, 3.0),
            ("phase resistance", m.phase_resistance_20_ohm, 0.001, 2.0),
            ("inductance", m.phase_inductance_h, 0.000001, 0.01),
            ("flux linkage", m.flux_linkage_wb, 0.001, 0.05),
            ("first ratio", d.stage_one_ratio, 1.0, 10.0),
            ("second ratio", d.stage_two_ratio, 1.0, 10.0),
            ("reduction efficiency", d.reduction_efficiency, 0.5, 1.0),
            ("cadence", d.cadence_rpm, 30.0, 150.0),
            ("continuous crank power", d.continuous_crank_w, 50.0, 1500.0),
            ("peak crank power", d.peak_crank_w, 50.0, 2000.0),
            ("assist cutoff", d.assist_cutoff_kph, 1.0, 45.0),
            ("bus minimum", c.bus_min_v, 20.0, 60.0),
            ("bus maximum", c.bus_max_v, 20.0, 75.0),
            ("bus nominal", c.bus_nominal_v, 20.0, 60.0),
            ("battery limit", c.battery_limit_a, 1.0, 60.0),
            ("iq peak limit", c.iq_peak_limit_a, 1.0, 80.0),
            ("MOSFET rating", c.mosfet_rating_v, 60.0, 200.0),
            ("PWM", c.pwm_hz, 10000.0, 40000.0),
            ("PCB length", c.pcb_length_mm, 40.0, 150.0),
            ("PCB width", c.pcb_width_mm, 30.0, 100.0),
            ("PCB thickness", c.pcb_thickness_mm, 0.8, 3.2),
            ("ambient", t.ambient_c, -10.0, 60.0),
            ("winding limit", t.winding_limit_c, 60.0, 150.0),
            ("winding thermal resistance", t.winding_to_case_k_per_w, 0.01, 10.0),
            ("case thermal resistance", t.case_to_air_k_per_w, 0.01, 10.0),
            ("core loss", t.core_loss_w, 0.0, 200.0),
            ("controller loss", t.controller_loss_w, 0.0, 200.0),
        ] {
            range(name, n, lo, hi)?;
        }
        if !(1..=100).contains(&m.turns_per_tooth) {
            return Err(invalid("turns per tooth must be 1..=100"));
        }
        if !(c.bus_min_v < c.bus_nominal_v && c.bus_nominal_v < c.bus_max_v) {
            return Err(invalid("bus voltages must increase: min < nominal < max"));
        }
        if d.peak_crank_w < d.continuous_crank_w {
            return Err(invalid("peak power cannot be below continuous power"));
        }
        if self.files.len() > 32 {
            return Err(invalid("at most 32 embedded sources"));
        }
        for (path, text) in &self.files {
            if path.is_empty()
                || path.len() > 160
                || path.starts_with('/')
                || path.contains('\\')
                || path.split('/').any(|s| s.is_empty() || s == ".." || s == ".")
                || !path.bytes().all(|b| b.is_ascii_alphanumeric() || b"/_-.".contains(&b))
                || text.len() > 65536
            {
                return Err(invalid("embedded source path or size is invalid (relative path, 64 KiB maximum)"));
            }
        }
        Ok(())
    }
    /// Native parameters drive the displayed envelopes. Changing them through the ordinary
    /// parameter editor is detected by project.check rather than silently using stale physics.
    pub fn geometry_parameters(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("eb_stage_one_r", 12.0 * self.drive.stage_one_ratio),
            ("eb_stage_two_r", 12.0 * self.drive.stage_two_ratio),
            ("eb_stator_r", self.motor.stator_radius_mm),
            ("eb_gap", self.motor.air_gap_mm),
            ("eb_magnet_t", self.motor.magnet_thickness_mm),
            ("eb_iron_t", self.motor.back_iron_mm),
            ("eb_stack", self.motor.stack_mm),
            ("eb_shaft_r", self.motor.shaft_radius_mm),
            ("eb_bb_width", self.bike.bottom_bracket_width_mm),
            ("eb_bb_bore", self.bike.bottom_bracket_bore_mm),
            ("eb_chainline", self.bike.chainline_mm),
            ("eb_wheel_r", self.bike.wheel_radius_mm),
            ("eb_pcb_l", self.controller.pcb_length_mm),
            ("eb_pcb_w", self.controller.pcb_width_mm),
            ("eb_pcb_t", self.controller.pcb_thickness_mm),
        ]
    }
    /// Configuration source is derived from the same definition as the feasibility checks.
    pub fn sync_firmware_config(&mut self) {
        self.files.insert("firmware/config.rs".into(), format!(
            "// Generated from the native project. Change the project definition, then apply.\npub const MIN_BUS_V: f32 = {:.6};\npub const MAX_BUS_V: f32 = {:.6};\npub const WINDING_LIMIT_C: f32 = {:.6};\npub const IQ_PEAK_LIMIT_A: f32 = {:.6};\npub const ASSIST_CUTOFF_KPH: f32 = {:.6};\n",
            self.controller.bus_min_v,self.controller.bus_max_v,self.thermal.winding_limit_c,
            self.controller.iq_peak_limit_a,self.drive.assist_cutoff_kph));
    }
    pub fn estimate(&self) -> Result<Estimate> {
        self.validate()?;
        let (m, d, c, t) = (&self.motor, &self.drive, &self.controller, &self.thermal);
        let rpm = d.cadence_rpm * d.stage_one_ratio * d.stage_two_ratio;
        let omega = rpm * std::f64::consts::TAU / 60.0;
        let kt = 1.5 * f64::from(m.poles / 2) * m.flux_linkage_wb;
        let iq = (d.continuous_crank_w / d.reduction_efficiency + t.core_loss_w) / (omega * kt);
        let peak_iq = (d.peak_crank_w / d.reduction_efficiency + t.core_loss_w) / (omega * kt);
        let cold_loss = 1.5 * iq * iq * m.phase_resistance_20_ohm;
        let total_rth = t.winding_to_case_k_per_w + t.case_to_air_k_per_w;
        let feedback = cold_loss * 0.00393 * total_rth;
        let winding = if feedback < 1.0 {
            Some(
                (t.ambient_c + (t.core_loss_w + t.controller_loss_w) * t.case_to_air_k_per_w + cold_loss * (1.0 - 0.00393 * 20.0) * total_rth)
                    / (1.0 - feedback),
            )
        } else {
            None
        };
        let copper = winding.map(|temp| cold_loss * (1.0 + 0.00393 * (temp - 20.0)));
        let resistance = winding.map(|temp| m.phase_resistance_20_ohm * (1.0 + 0.00393 * (temp - 20.0)));
        // Surface PMSM dq model, id=0, peak phase values. No field weakening.
        let phase_v = resistance.map(|r| {
            let omega_e = omega * f64::from(m.poles / 2);
            (omega_e * m.phase_inductance_h * iq).hypot(r * iq + omega_e * m.flux_linkage_wb)
        });
        let input = copper.map(|loss| d.continuous_crank_w / d.reduction_efficiency + loss + t.core_loss_w + t.controller_loss_w);
        let checks = vec![
            Finding::new("continuous_current", iq <= c.iq_peak_limit_a, "Continuous dq current fits the configured peak phase-current limit"),
            Finding::new("peak_current", peak_iq <= c.iq_peak_limit_a, "Peak torque current fits; peak duration and heating remain unverified"),
            Finding::new(
                "winding_temperature",
                winding.is_some_and(|x| x <= t.winding_limit_c),
                "Lumped steady-state winding temperature fits the configured limit",
            ),
            Finding::new(
                "voltage_headroom",
                phase_v.is_some_and(|x| x <= c.bus_min_v / 3.0_f64.sqrt() * 0.95),
                "Continuous dq voltage fits 95% of SVPWM phase capability at minimum battery voltage",
            ),
            Finding::new(
                "battery_current",
                input.is_some_and(|p| p / c.bus_min_v <= c.battery_limit_a),
                "Estimated continuous battery current fits at minimum voltage",
            ),
            Finding::new(
                "voltage_rating",
                c.mosfet_rating_v >= c.bus_max_v * 1.5,
                "MOSFET nameplate margin exceeds 1.5x full-charge voltage; transients still require measurement",
            ),
        ];
        Ok(Estimate {
            motor_rpm: rpm,
            crank_torque_nm: d.continuous_crank_w / (d.cadence_rpm * std::f64::consts::TAU / 60.0),
            torque_constant_nm_per_peak_a: kt,
            iq_peak_a: iq,
            peak_iq_a: peak_iq,
            winding_c: winding,
            copper_loss_w: copper,
            phase_voltage_peak_v: phase_v,
            battery_input_w: input,
            checks,
            release_ready: false,
        })
    }
}
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct Finding {
    pub id: String,
    pub passed: bool,
    pub explanation: String,
}
impl Finding {
    pub fn new(id: &str, passed: bool, explanation: &str) -> Self {
        Self { id: id.into(), passed, explanation: explanation.into() }
    }
}
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct Estimate {
    pub motor_rpm: f64,
    pub crank_torque_nm: f64,
    pub torque_constant_nm_per_peak_a: f64,
    pub iq_peak_a: f64,
    pub peak_iq_a: f64,
    pub winding_c: Option<f64>,
    pub copper_loss_w: Option<f64>,
    pub phase_voltage_peak_v: Option<f64>,
    pub battery_input_w: Option<f64>,
    pub checks: Vec<Finding>,
    pub release_ready: bool,
}
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    #[test]
    fn nominal_and_hot_cases() {
        let mut p = Project::default();
        let e = p.estimate().unwrap();
        assert!((e.motor_rpm - 2700.0).abs() < 1e-9);
        assert!((e.crank_torque_nm - 53.0516476973).abs() < 1e-8);
        assert!((e.torque_constant_nm_per_peak_a - 0.084).abs() < 1e-12);
        assert!(e.checks.iter().all(|x| x.passed), "{e:?}");
        assert!(!e.release_ready);
        p.thermal.case_to_air_k_per_w = 10.0;
        let e = p.estimate().unwrap();
        assert!(e.winding_c.is_none());
        assert!(e.checks.iter().any(|x| x.id == "winding_temperature" && !x.passed));
    }
    #[test]
    fn hostile_and_future_data_refused() {
        let mut p = Project::default();
        p.motor.air_gap_mm = f64::NAN;
        assert!(p.validate().is_err());
        let p = Project { schema_version: 2, ..Project::default() };
        assert!(p.validate().is_err());
        let mut p = Project::default();
        p.files.insert("../escape".into(), "x".into());
        assert!(p.validate().is_err());
        let mut p = Project::default();
        p.controller.bus_max_v = 30.0;
        assert!(p.validate().is_err());
    }
    #[test]
    fn electrical_boundary_cases_fail() {
        let mut p = Project::default();
        p.motor.flux_linkage_wb = 0.02;
        assert!(p.estimate().unwrap().checks.iter().any(|x| x.id == "voltage_headroom" && !x.passed));
        let mut p = Project::default();
        p.controller.iq_peak_limit_a = 10.0;
        assert!(p.estimate().unwrap().checks.iter().any(|x| x.id == "continuous_current" && !x.passed));
    }
}
