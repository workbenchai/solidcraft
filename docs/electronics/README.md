# Native electronics in SolidCraft

SolidCraft stores an electrical design alongside the mechanical document. The independent
`solidcraft-electronics` Rust crate defines the model, graph edits and scoped validation.
It has no EDA application dependency, process invocation, copied EDA implementation or
intermediate vendor file. Dependencies are existing workspace libraries: serde, serde_json,
schemars, sha2 and thiserror, all permissively licensed.

This is a programmatic design foundation. It does not yet add a graphical schematic editor,
interactive board router, copper fill engine, field solver or fabrication exporter.

## Saved entities

| Entity | Definition |
|---|---|
| Board | Outline with cutouts; ordered stackup; design rules; placements; pads, traces, vias and pours |
| Layer | Stable identity, function, material, thickness and optional relative permittivity |
| Component | Stable identity and reference designator; typed device data; logical pins; optional manufacturer/part number; system membership |
| Device kind | Resistor, capacitor, inductor, IC, discrete device, connector, test point or explicitly described other device |
| Pin | Local pin number, name, electrical direction/type and intentional no-connect state |
| Pad | Physical circle, rectangle or rounded rectangle; position/rotation; copper layer membership; optional round/slotted drill; mask and paste margins |
| Trace | Net, copper layer, width and centreline polyline |
| Via | Net, position, inclusive copper layer span, drill/pad diameters and tenting |
| Pour | Net, copper layer, polygon boundary, clearance, minimum neck, priority and thermal-relief intent |
| Net | Named logical connection with a set of component-pin references |
| Netgroup | Set of nets with width/clearance constraints and an optional positive/negative differential pair |
| Schematic view | Symbol positions, visible pin locations, explicit junctions, wires and net labels |
| System | Functional block with optional parent and named ports bound to nets; a parent makes it a subsystem |

Dimensions are millimetres, angles radians and electrical quantities SI. Tolerances are
fractions, not percentages. Board coordinates are X/Y; the board frame defines a 3D origin
and orthonormal X/Y axes. Its normal is X cross Y. This frame is a saved interface definition;
it does not automatically reposition linked CAD geometry. Positive depth into the stack is
along the negative normal. Component placement rotations are counterclockwise as seen from
the top; bottom-side footprint generation must mirror its local geometry before producing
board-local pad positions. Pads are already transformed board-local geometry. The `put_placement` edit moves/rotates/mirrors pin-bound pads with their component.
Traces retain their coordinates and must be reassessed. Whole-board replacement carries
explicit final pad coordinates. Footprint changes require removing/replacing the old pads.

`LayerId` belongs to a board. Pad/trace/via/pour ids belong to that board. Component, net,
netgroup, system and view ids belong to the project. Pin numbers belong to a component.
Reference designators are unique within a board; different boards may each contain R1.
Opaque string ids must be stable; changing a display name never changes identity. A logical
component can be placed on only one board; use separate instances for repeated hardware.
Stackup order is physical top to bottom. Through, blind and buried via endpoints must follow
that order. Differential-pair constraints record intent; length/skew and impedance are not
solved by this change. Solder mask/paste expansion is a linear margin in millimetres, not a ratio.

## Connectivity is shared across views

`Net.pins` is the authoritative pin-to-net assignment. A pin belongs to at most one net and
cannot be both connected and intentionally no-connect. A physical pad binds to a `PinRef`,
a net directly (e.g. a standalone copper land), or no electrical connection (e.g. a mounting
hole). `Project::pad_net` resolves the intended net without duplicating the assignment.
Several pads can bind to one logical pin. A pin-bound pad requires its component to be placed
on that board. Unassigned pins are permitted in unfinished designs and reported by checks.

Wires reference logical pins, explicit schematic junctions or system ports. Both endpoints
must agree with the wire's logical net. Pins must be drawn in that view. Crossing wires and
coincident coordinates never implicitly create a connection. Net-label text is presentation;
its explicit net id determines meaning. Multiple schematic views may show different pins of
the same component. Wire paths are drawing intent; endpoint-to-path geometry is not checked.

A net can span several boards. That records logical intent; it does not manufacture a harness,
prove that connectors mate or imply that PCB copper connects across boards.

## Native commands

All commands are available through `Session::execute`, scripts, the CLI and the existing
MCP execute/batch interface. They publish typed JSON input and output schemas.

- `electronics.inspect {}` returns `present`, `project`, `revision` and `report`. An ordinary
  mechanical design returns an empty electrical project with `present: false`.
- `electronics.replace {project, expected_revision}` validates and replaces only the
  electrical graph. Mechanical features and the separate e-bike study are retained.
- `electronics.apply {edits, expected_revision}` applies 1..1024 ordered edits, validates the
  final graph, and accepts it in one undo step. A broken reference rejects the whole transaction.
  Obtain `expected_revision` from a fresh inspect call; stale document revisions are refused.

Edits are internally tagged by `op`: `put_board`, `put_component`, `put_net`, `put_netgroup`,
`put_system`, `put_schematic`, `put_pad`, `put_trace`, `put_via`, `put_pour`, `put_placement`,
and their `remove_*` counterparts. Board-scoped edits carry `board`; component placements
carry `component`. Replace a board to change its layers, or replace a component to change
its pins. Update logical nets and affected schematic references together in one transaction.
Deletes never silently cascade. Remove every dependent reference in the same transaction.

For example, after inspecting the design, add a trace with:

```json
{
  "edits": [{
    "op": "put_trace",
    "board": "filter-board",
    "id": "buffer-input",
    "trace": {
      "net": "filtered",
      "layer": "top",
      "points": [[14, 12], [22, 10]],
      "width_mm": 0.25
    }
  }],
  "expected_revision": 12
}
```

The revision above is illustrative; substitute the current inspect reply. Existing ids are
updated, new ids are inserted, and removing an unknown id is an error. Replaying an edit
against an old revision never overwrites newer work.

## Mechanical integration and persistence

`Document.electronics` is optional and saved in native `.sc` files. Old designs default to
no electronics. A pending native-format change protects against older releases silently
losing the new data. Opening validates the graph and mechanical links.

A board and each component placement may reference a SolidCraft mechanical component id.
Referenced CAD components must exist. Deleting a linked component tree is refused until the
link is explicitly removed. It does not automatically create detailed component bodies or
move them when electrical positions change. The example creates a substrate envelope only.

## Validation and evidence boundaries

Structural validation refuses unknown references, invalid/nonfinite values, invalid polygon
rings/cutouts, duplicate references and placements, inconsistent no-connect assignments,
invalid via spans, contradictory wires and cyclic/deep system hierarchies. Global entity and
geometry budgets bound traversal. It permits unfinished circuits.

`Project::check` returns a SHA-256 fingerprint and findings with stable entity addresses.
It runs basic pin ERC, minimum trace-width checks, drill/annular-ring checks, centre-point
bounds checks and checks of pour-clearance settings. Overlapping netgroups use the strongest
minimum. A pure native project fingerprint covers electrical inputs and layout intent; it is
not a fingerprint of linked mechanical feature definitions or a hardware measurement.

The report lists checks performed and not performed. In particular, it does not establish:

- Complete copper connectivity, no shorts, copper-to-copper or edge clearances.
- Actual pour fill, islands, thermal connections or routed completeness.
- Signal integrity, impedance, current capacity, voltage ratings or circuit functionality.
- Manufacturing process qualification or hardware performance.

`manufacturing_ready` remains false. A pour boundary is not filled copper and must not be
exported as if its clearances and connections had been resolved. IPC-2581, Gerber and
simulation adapters should consume this model through explicit, validated export snapshots.
They are follow-on integrations; no IPC-2581 exporter is claimed here.

## Run the original example

```sh
cargo test --profile ci -p solidcraft-electronics
cargo run --profile ci -p solidcraft-electronics --example rc-filter
cargo run --profile ci -p solidcraft-engine --example electronics-native -- /tmp/electronics-study.sc
```

The first example prints a typed RC-filter/IC/connector project and its report. It contains
four components, five nets, a two-copper-layer board, pads, traces, a via, a pour boundary,
a schematic view and a system/subsystem pair. It intentionally is not fully routed. The
second example adds an editable substrate envelope, links it to the board and saves `.sc`.

## Verification status

The 19 electrical-core tests, strict core Clippy checks, WebAssembly check and RC-filter
executable pass independently of the geometry kernel. Repository formatting, assets,
licences, layering, list, skill, UI-container and pending-format checks also passed.

The engine, its test code and examples type-checked with an isolated verification copy of
Solidcore. The published dependency pin `8ddce90` has a pre-existing type-inference error in
`blend/blend_corner/mixed.rs:1356`: the `of2m` call infers an unsized slice element. Verification
used an explicit `<[Option<BSplineCurve>; 4]>` type argument on that call in a temporary copy.
This extension does not change the dependency pin or include that workaround. An unmodified
engine build remains blocked by that dependency error. Engine runtime tests, the saved `.sc`
example execution and full workspace CI are not claimed as passing.
