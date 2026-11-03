# SolidCraft fork and related-project review

Reviewed 2026-11-03, before implementation. This is a bounded public-source survey, not a claim that all fork branches were inspected.

## Search method

- Queried GitHub repository search for `solidcraft fork:only`, plus README searches for PCB, motor, thermal and electromagnetic work.
- Retrieved the first 100 upstream forks sorted by stars from GitHub's public forks API. Screened names and descriptions; most advertise unchanged CAD functionality.
- Read SolidCraft's current architecture, manifests, contribution rules and reference-test documentation.
- Read the READMEs of the relevant sibling projects and the rover example. Did not read or port GPL implementation code.

## Findings

| Project | Observed work | Relevance and decision |
| --- | --- | --- |
| [Hawaii-005/solidcraft](https://github.com/Hawaii-005/solidcraft) | Fork description identifies Android, touch and S Pen support | Platform work; no motor/ECAD capability established by this survey |
| [Exa201/SolidCraft-Mobile](https://github.com/Exa201/SolidCraft-Mobile) | Fork description identifies native Android support | Platform work; no motor/ECAD capability established |
| [denara/solidcraft-unraid](https://github.com/denara/solidcraft-unraid) | Fork description explicitly identifies Unraid packaging | Deployment work |
| [0xlillith/solidcraft-rover](https://github.com/0xlillith/solidcraft-rover) | README describes an editable, MCP-built rover with motor and wiring illustrations | Useful assembly precedent; not evidence of electromagnetic design or an integrated electronics editor |
| [tangency-labs/fieldcraft](https://github.com/tangency-labs/fieldcraft) | README reports structural, steady/transient thermal and coupled thermal-stress solvers; MIT OR Apache-2.0 | Strong future external analysis adapter. No magnetic solver verified. Not linked or vendored in this change |
| [tangency-labs/pcbcraft](https://github.com/tangency-labs/pcbcraft) | README reports schematic, SMD/through-hole layout, basic routing, checks and KiCad board export; explicitly GPL-3.0-or-later | Overlapping EDA work, but incompatible with this fork's permissive-code policy. No implementation copied, read or linked |
| [jijinbei/solidcraft](https://github.com/jijinbei/solidcraft), branch `exchange/snapshot` | [Upstream PR 8845](https://github.com/tangency-labs/solidcraft/pull/8845) proposes `solidcraft-exchange` and `export-snapshot` for typed geometry/analysis exchange; open and unmerged when checked | Closest related SolidCraft fork found. Its PR reports FieldCraft format/pin incompatibility. Not integrated or benchmarked against this fork |
| [jijinbei/EMCraft](https://github.com/jijinbei/EMCraft) | Public README declares MIT OR Apache-2.0 and documents RF/microwave FEM/time-domain solvers, magnetic losses and parameter studies | Related electromagnetic work, but README does not establish nonlinear motor magnetostatics, permanent-magnet rotor torque or rotating-machine support. No implementation linked or copied |
| [upstream issue 5091](https://github.com/tangency-labs/solidcraft/issues/5091) | Proposal to import KiCad board/copper/component geometry | Relevant interchange proposal; issue text alone is not evidence of implemented ECAD |

No integrated BLDC electromagnetic + thermal + motor-controller design fork was established by these searches. [Issue 8844](https://github.com/tangency-labs/solidcraft/issues/8844) still describes EMCraft as unpublished; the public repository was subsequently found and its current README read. [FieldCraft issue 660](https://github.com/tangency-labs/fieldcraft/issues/660) requests electromagnetic capability, and [SolidCraft issue 1624](https://github.com/tangency-labs/solidcraft/issues/1624) is an electrical-plugin research request rather than implemented functionality.

Private work, unindexed branches and forks outside the sampled pages remain unknown. FieldCraft and PCBCraft are sibling projects, not evidence that a SolidCraft fork already supplies the requested combined workflow.

## Resulting implementation choice

Add a small native multidisciplinary project in the Workbench fork. Reuse SolidCraft's ordinary feature and command engine. Preserve the MIT/Apache boundary. Embed editable design sources without executing them on load. Keep analytical estimates and unresolved validation work visible. Defer a FieldCraft adapter until a pinned compatible geometry contract and thermal benchmark are demonstrated.
