# Pedal-assist motor and controller project

This fork adds one native project that keeps a motor assembly, controller architecture, logical harness and firmware source in a saved `.sc` design. It is an engineering study and development foundation. It is not a fabricated, tested drive or a completed ECAD/firmware suite.

## Open and edit

1. Build and launch SolidCraft normally.
2. Start an empty design. Open the command palette and choose **Create E-bike Project**.
3. The **Design** page edits the typed motor, battery, drive and thermal definition. Choose **Create assembly**.
4. `firmware/config.rs` is regenerated from the project definition when applying changes. **Sources** edits the embedded controller architecture, wiring JSON and Rust supervisory control source.
5. **Checks** reports estimates, consistency errors and explicitly unresolved validation gates.
6. Save using the ordinary Save command. Reopen the `.sc` file to recover both the assembly and embedded sources.
7. Use **E-bike Project** in the command palette to reopen the editor. **Apply and check** updates the named geometry parameters without replacing manually edited features. Stale editors refuse to overwrite a changed document; **Reload current project** discards the draft.

The generated solids are ordinary timeline features and components. Change dimensions, add holes, replace an envelope with a detailed component, and inspect or hide components with existing SolidCraft tools. Rotor, winding and reduction solids are simplified fit-study geometry, not manufacturing drawings. The source editor is plain text; it does not compile or flash firmware and is not a graphical PCB router.

## CLI and MCP

```sh
cargo run --profile ci -p solidcraft-cli -- run examples/ebike/create.json --out /tmp/ebike.step
cargo run --profile ci -p solidcraft-cli -- snapshot examples/ebike/create.json --out /tmp/ebike.png
```

The script saves `/tmp/ebike.sc` using the existing `file.save_as` command. Change this path before running on another platform. MCP uses existing `execute` / `batch` with `ebike.create`, `ebike.inspect` and `ebike.configure`; `describe_command` publishes typed input and output schemas. Configure requires the revision returned by a fresh `ebike.inspect` and a complete `project` object, including embedded source files. Always inspect again after a mutation before making another change.

## Baseline assumptions

| Item | Initial assumption |
| --- | --- |
| Donor | Original generic steel diamond-frame template, not a measured or licensed stock bicycle model |
| Fit | 68 mm bottom bracket shell width, 34.8 mm bore, 50 mm chainline, 335 mm wheel radius; all must be measured |
| Location | Motor near bottom bracket, axis parallel to crank; a 2-stage reduction and crank freewheel must be detailed |
| Supply | User-supplied 13-series lithium-ion pack, 48 V nominal convention, 54.6 V full-charge, 39 V minimum design point; BMS, fused interface and charger assumed external |
| Motor | Radial-flux outrunner, 12 slots / 14 poles, 80 mm nominal stator diameter, 30 mm active stack |
| Geometry | 0.7 mm nominal radial gap before rectangular-tooth corner effects; flat magnet, bearing, coil, housing and mount envelopes |
| Reduction | 5:1 then 6:1, 90% assumed total mechanical efficiency; pulleys are envelopes, not tooth profiles |
| Assist targets | 500 W continuous and 750 W peak mechanical output at crank, at 90 rpm cadence |
| Controller | 25 A battery limit, 40 A peak dq/phase current limit, 20 kHz PWM, 100 V MOSFET design target |
| Board | 90 × 60 × 1.6 mm envelope; pin-level schematic and routing remain incomplete |
| Thermal | 25 C ambient, 100 C winding limit; assumed winding/case and case/air thermal resistances |

"Standard e-bike battery" is not one universal voltage or connector. This is an explicit editable 48 V baseline. The speed cutoff is an engineering configuration, not a claim about any jurisdiction's legal classification.

The controller architecture lists STM32G431 and DRV8353 as candidates. See [TI DRV8353](https://www.ti.com/product/DRV8353), its [datasheet](https://www.ti.com/lit/ds/symlink/drv8353.pdf), and [ST's motor-control introduction](https://wiki.st.com/stm32mcu/wiki/STM32MotorControl:Introduction_to_Motor_Control_with_STM32). DRV8353's VM operating limit is 75 V; its VDrain sense limit is 100 V. These are separate from external MOSFET ratings. No vendor implementation or SDK source is included.

## What the checks mean

The feasibility model uses SI units internally for electrical and thermal values and millimetres for geometry:

- Motor speed = cadence × both reduction ratios.
- Crank torque = crank mechanical power / crank angular speed.
- Current sizing includes mechanical shaft power and the assumed core-loss torque at the operating speed.
- Surface PMSM, `id = 0`: torque = `1.5 × pole_pairs × phase_peak_flux_linkage × iq_peak`.
- Copper loss = `1.5 × iq_peak² × phase_resistance(T)`; phase RMS current is `iq_peak / sqrt(2)`.
- `R(T) = R20 × (1 + 0.00393 × (T - 20 C))`.
- Steady winding temperature solves the two-resistance thermal network algebraically, including resistance/temperature feedback. Non-positive stability denominator returns no temperature and a failed check.
- dq voltage includes resistive drop, back EMF and synchronous inductive drop; comparison uses 95% of `Vdc_min / sqrt(3)`.
- Battery input includes reduction loss, winding loss, assumed core loss and assumed controller loss.

Flux linkage, resistance, inductance, losses and thermal resistances are **assumed input values**. Turns and wire diameter are recorded design intent; they do not automatically establish those physical parameters. Passing these estimates never sets `release_ready` to true. Peak current is checked, but peak voltage, transient heating and allowable peak duration require further work.

## Development slices and acceptance evidence

1. **Native project (this change):** save/reopen, parameter changes, embedded source edits, guarded writes, ordinary editable assembly, analytic checks and fail-closed supervisory logic.
2. **Motor fidelity:** validate a winding schedule, slot fill, stack and magnet materials; compare no-load EMF, torque/current, saturation and losses against an independent solver and a bench motor. Bind solver artifacts to design revisions.
3. **Mechanical transmission:** design and check both reduction stages, shaft/rotor retention, bearings, clutch/freewheel, clamps, fatigue, pedal clearance, chainline, guards and sealing. Validate against measurements of a real donor frame before fabrication.
4. **Controller:** complete a rated pin-level schematic and BOM, native net/pad/track model, routed power stage, ERC/DRC and fabrication exports. Test gate timing, current sensing, load transients and heat using a limited bench supply.
5. **Firmware:** commission hardware interfaces, current sensing, rotor-angle sensing, FOC, current/speed loops, hardware shutdown and watchdog. Validate brake/cadence loss and sensor faults on a restrained test stand.
6. **Coupled qualification:** measured efficiency and thermal rise, transient peak duty, dynamometer torque/speed curves, enclosure and harness tests, and verified installed clearances.

A design change must invalidate relevant evidence; a file or screenshot alone is not a passing verification. No magnetic solver, FieldCraft adapter, fabrication-ready PCB, motor winding recipe or MCU flash target is claimed by this change.
