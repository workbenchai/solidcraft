//! Atomic graph edits. References are checked after the entire transaction, so related entities
//! can be created or deleted together. The original project remains intact on any error.
use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Edit {
    PutBoard { id: BoardId, board: Board },
    PutComponent { id: ComponentId, component: Component },
    PutNet { id: NetId, net: Net },
    PutNetgroup { id: NetGroupId, netgroup: NetGroup },
    PutSystem { id: SystemId, system: System },
    PutSchematic { id: ViewId, schematic: SchematicView },
    PutPad { board: BoardId, id: PadId, pad: Pad },
    PutTrace { board: BoardId, id: TraceId, trace: Trace },
    PutVia { board: BoardId, id: ViaId, via: Via },
    PutPour { board: BoardId, id: PourId, pour: Pour },
    PutPlacement { board: BoardId, component: ComponentId, placement: Placement },
    RemoveBoard { id: BoardId },
    RemoveComponent { id: ComponentId },
    RemoveNet { id: NetId },
    RemoveNetgroup { id: NetGroupId },
    RemoveSystem { id: SystemId },
    RemoveSchematic { id: ViewId },
    RemovePad { board: BoardId, id: PadId },
    RemoveTrace { board: BoardId, id: TraceId },
    RemoveVia { board: BoardId, id: ViaId },
    RemovePour { board: BoardId, id: PourId },
    RemovePlacement { board: BoardId, component: ComponentId },
}
impl Project {
    pub fn apply(&self, edits: &[Edit]) -> Result<Self> {
        if edits.is_empty() || edits.len() > 1024 {
            return Err(Error("transaction needs 1..1024 edits".into()));
        }
        self.validate()?;
        let mut next = self.clone();
        for edit in edits {
            match edit {
                Edit::PutBoard { id, board } => {
                    next.boards.insert(id.clone(), board.clone());
                }
                Edit::PutComponent { id, component } => {
                    next.components.insert(id.clone(), component.clone());
                }
                Edit::PutNet { id, net } => {
                    next.nets.insert(id.clone(), net.clone());
                }
                Edit::PutNetgroup { id, netgroup } => {
                    next.netgroups.insert(id.clone(), netgroup.clone());
                }
                Edit::PutSystem { id, system } => {
                    next.systems.insert(id.clone(), system.clone());
                }
                Edit::PutSchematic { id, schematic } => {
                    next.schematics.insert(id.clone(), schematic.clone());
                }
                Edit::RemoveBoard { id } => {
                    removed(next.boards.remove(id))?;
                }
                Edit::RemoveComponent { id } => {
                    removed(next.components.remove(id))?;
                }
                Edit::RemoveNet { id } => {
                    removed(next.nets.remove(id))?;
                }
                Edit::RemoveNetgroup { id } => {
                    removed(next.netgroups.remove(id))?;
                }
                Edit::RemoveSystem { id } => {
                    removed(next.systems.remove(id))?;
                }
                Edit::RemoveSchematic { id } => {
                    removed(next.schematics.remove(id))?;
                }
                other => {
                    let bid = match other {
                        Edit::PutPad { board, .. }
                        | Edit::PutTrace { board, .. }
                        | Edit::PutVia { board, .. }
                        | Edit::PutPour { board, .. }
                        | Edit::PutPlacement { board, .. }
                        | Edit::RemovePad { board, .. }
                        | Edit::RemoveTrace { board, .. }
                        | Edit::RemoveVia { board, .. }
                        | Edit::RemovePour { board, .. }
                        | Edit::RemovePlacement { board, .. } => board,
                        _ => return Err(Error("invalid board edit".into())),
                    };
                    let b = next.boards.get_mut(bid).ok_or_else(|| Error("unknown edit board".into()))?;
                    match other {
                        Edit::PutPad { id, pad, .. } => {
                            b.pads.insert(id.clone(), pad.clone());
                        }
                        Edit::PutTrace { id, trace, .. } => {
                            b.traces.insert(id.clone(), trace.clone());
                        }
                        Edit::PutVia { id, via, .. } => {
                            b.vias.insert(id.clone(), via.clone());
                        }
                        Edit::PutPour { id, pour, .. } => {
                            b.pours.insert(id.clone(), pour.clone());
                        }
                        Edit::PutPlacement { component, placement, .. } => {
                            move_placement(b, component, placement)?;
                        }
                        Edit::RemovePad { id, .. } => {
                            removed(b.pads.remove(id))?;
                        }
                        Edit::RemoveTrace { id, .. } => {
                            removed(b.traces.remove(id))?;
                        }
                        Edit::RemoveVia { id, .. } => {
                            removed(b.vias.remove(id))?;
                        }
                        Edit::RemovePour { id, .. } => {
                            removed(b.pours.remove(id))?;
                        }
                        Edit::RemovePlacement { component, .. } => {
                            removed(b.placements.remove(component))?;
                        }
                        _ => return Err(Error("invalid board edit".into())),
                    }
                }
            }
            crate::validate::budget(&next)?;
        }
        next.validate()?;
        Ok(next)
    }
}
fn removed<T>(v: Option<T>) -> Result<()> {
    v.map(|_| ()).ok_or_else(|| Error("cannot remove missing entity".into()))
}

// Physical pads follow a component move in the same transaction. Traces stay where they are;
// routing must be reassessed after a placement edit.
fn move_placement(board: &mut Board, component: &ComponentId, next: &Placement) -> Result<()> {
    if let Some(old) = board.placements.get(component) {
        let has_pads = board.pads.values().any(|p| matches!(&p.connection, Connection::Pin { pin } if &pin.component == component));
        if has_pads && old.footprint != next.footprint {
            return Err(Error("remove or replace footprint pads before changing the footprint".into()));
        }
        let copper: Vec<_> = board.stackup.iter().filter(|l| l.kind == LayerKind::Copper).map(|l| l.id.clone()).collect();
        let mirrored_layers: std::collections::BTreeMap<_, _> = copper.iter().zip(copper.iter().rev()).collect();
        let (os, oc) = old.rotation_rad.sin_cos();
        let (ns, nc) = next.rotation_rad.sin_cos();
        for pad in board.pads.values_mut().filter(|p| matches!(&p.connection, Connection::Pin { pin } if &pin.component == component)) {
            if old.at == next.at && old.rotation_rad == next.rotation_rad && old.side == next.side {
                continue;
            }
            if old.side != next.side {
                pad.layers = pad.layers.iter().map(|l| mirrored_layers.get(l).map_or_else(|| l.clone(), |v| (*v).clone())).collect();
            }
            let dx = pad.at[0] - old.at[0];
            let dy = pad.at[1] - old.at[1];
            let mut x = oc * dx + os * dy;
            let y = -os * dx + oc * dy;
            if old.side != next.side {
                x = -x;
            }
            pad.at = [next.at[0] + nc * x - ns * y, next.at[1] + ns * x + nc * y];
            let relative = pad.rotation_rad - old.rotation_rad;
            pad.rotation_rad = next.rotation_rad + if old.side != next.side { std::f64::consts::PI - relative } else { relative };
        }
    }
    board.placements.insert(component.clone(), next.clone());
    Ok(())
}
