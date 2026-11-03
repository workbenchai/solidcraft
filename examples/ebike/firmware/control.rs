//! Portable pedal-assist supervisory logic, not a hardware driver or a complete FOC loop.
//! Gate output stays disabled until sensor calibration and hardware commissioning are supplied.
#![no_std]
mod config;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State { Disabled, Ready, Assisting, Fault }
#[derive(Clone, Copy)]
pub struct Input {
    pub bus_v: f32,
    pub winding_c: f32,
    pub controller_c: f32,
    pub cadence_rpm: f32,
    pub wheel_kph: f32,
    pub demand: f32,
    pub brake: bool,
    pub fault: bool,
    pub sensors_valid: bool,
    pub commissioned: bool,
}
pub struct Controller { pub state: State }
pub struct Output { pub iq_peak_a: f32, pub gate_enable: bool }
impl Default for Controller { fn default() -> Self { Self { state: State::Disabled } } }
impl Controller {
    /// Caller must supply fresh samples each tick; hardware watchdog and brake interlock are separate.
    pub fn tick(&mut self, i: Input, iq_limit_a: f32, cutoff_kph: f32) -> Output {
        let mut out = Output { iq_peak_a: 0.0, gate_enable: false };
        let finite = [i.bus_v, i.winding_c, i.controller_c, i.cadence_rpm,
            i.wheel_kph, i.demand, iq_limit_a, cutoff_kph].iter().all(|x| x.is_finite());
        if !finite || iq_limit_a <= 0.0 || iq_limit_a > config::IQ_PEAK_LIMIT_A || cutoff_kph <= 0.0 || cutoff_kph > config::ASSIST_CUTOFF_KPH
            || i.fault || !(config::MIN_BUS_V..=config::MAX_BUS_V).contains(&i.bus_v)
            || i.winding_c >= config::WINDING_LIMIT_C || i.controller_c >= 90.0 || i.wheel_kph < 0.0 {
            self.state = State::Fault;
        }
        if self.state == State::Fault { return out; } // power-cycle/manual service reset only
        if !i.commissioned || !i.sensors_valid {
            self.state = State::Disabled;
            return out;
        }
        self.state = State::Ready;
        if i.brake || i.cadence_rpm < 10.0 || i.wheel_kph >= cutoff_kph || i.demand <= 0.0 { return out; }
        self.state = State::Assisting;
        // Linear thermal derating over the last 20 C, no regeneration.
        let thermal = ((100.0-i.winding_c)/20.0).min((90.0-i.controller_c)/20.0).clamp(0.0,1.0);
        out.iq_peak_a = i.demand.clamp(0.0,1.0) * iq_limit_a * thermal;
        out.gate_enable = out.iq_peak_a > 0.0;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Input { Input { bus_v:48.0,winding_c:30.0,controller_c:30.0,cadence_rpm:60.0,
        wheel_kph:10.0,demand:0.5,brake:false,fault:false,sensors_valid:true,commissioned:false } }
    #[test] fn uncommissioned_never_enables() { assert!(!Controller::default().tick(sample(),30.0,25.0).gate_enable); }
    #[test] fn brake_and_no_cadence_cut_assist() {
        for (brake,cadence) in [(true,60.0),(false,0.0)] {
            let mut i=sample(); i.commissioned=true; i.brake=brake; i.cadence_rpm=cadence;
            assert!(!Controller::default().tick(i,30.0,25.0).gate_enable);
        }
    }
    #[test] fn faults_latch_and_nan_fails_closed() {
        let mut c=Controller::default(); let mut i=sample(); i.commissioned=true;
        i.bus_v=f32::NAN; assert!(!c.tick(i,30.0,25.0).gate_enable);
        i.bus_v=48.0; assert!(!c.tick(i,30.0,25.0).gate_enable); assert_eq!(c.state,State::Fault);
    }
    #[test] fn valid_assist_derates_and_speed_cuts() {
        let mut c=Controller::default(); let mut i=sample(); i.commissioned=true; i.winding_c=90.0;
        assert!((c.tick(i,30.0,25.0).iq_peak_a-7.5).abs()<0.001);
        i.wheel_kph=25.0; assert!(!c.tick(i,30.0,25.0).gate_enable);
    }
}
