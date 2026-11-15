use anyhow::Result;
use wasmtime::{Caller, Engine, Linker};

use crate::{
    geom,
    modules::{CELL_SIZE, ModuleKind, try_place},
    sandbox::BotState,
};

pub fn create_linker(engine: &Engine) -> Result<Linker<BotState>> {
    let mut linker = Linker::new(engine);

    linker.func_wrap("env", "credits", |caller: Caller<'_, BotState>| -> i32 {
        caller.data().credits
    })?;

    linker.func_wrap(
        "env",
        "place_module",
        |mut caller: Caller<'_, BotState>, gx: i32, gy: i32, kind: i32, dir_deg: f32| -> i32 {
            let state = caller.data_mut();
            if !state.building {
                return -1;
            }
            try_place(
                &mut state.blueprint,
                &mut state.credits,
                gx,
                gy,
                kind,
                dir_deg,
            )
        },
    )?;

    linker.func_wrap(
        "env",
        "get_self",
        |caller: Caller<'_, BotState>, stat: i32| -> f32 {
            let s = caller.data();
            let inp = &s.input;
            match stat {
                0 => inp.hp,
                1 => inp.x,
                2 => inp.y,
                3 => inp.angle,
                4 => inp.cooldown,
                5 => s.credits as f32,
                6 => inp.modules.len() as f32,
                _ => 0.0,
            }
        },
    )?;

    linker.func_wrap(
        "env",
        "module_hp",
        |caller: Caller<'_, BotState>, id: i32| -> f32 {
            if id < 0 {
                return -1.0;
            }
            caller
                .data()
                .input
                .modules
                .get(id as usize)
                .map(|m| m.hp.max(0.0))
                .unwrap_or(-1.0)
        },
    )?;

    linker.func_wrap(
        "env",
        "drive",
        |mut caller: Caller<'_, BotState>, speed: f32, turn: f32| {
            let out = &mut caller.data_mut().output;
            out.speed = speed.clamp(0.0, 1.0);
            out.turn = turn.clamp(-1.0, 1.0);
        },
    )?;

    linker.func_wrap(
        "env",
        "fire_module",
        |mut caller: Caller<'_, BotState>, id: i32| -> i32 {
            if id < 0 {
                return 0;
            }
            let idx = id as usize;
            let state = caller.data_mut();
            let ready = state.input.modules.get(idx).is_some_and(|m| {
                m.kind == ModuleKind::Cannon
                    && m.hp > 0.0
                    && m.cooldown <= 0.0
                    && !state.output.fired_modules.contains(&idx)
            });
            if ready {
                state.output.fired_modules.push(idx);
                1
            } else {
                0
            }
        },
    )?;

    linker.func_wrap("env", "fire", |mut caller: Caller<'_, BotState>| -> i32 {
        let state = caller.data_mut();
        let ready = state.input.modules.iter().enumerate().position(|(idx, m)| {
            m.kind == ModuleKind::Cannon
                && m.hp > 0.0
                && m.cooldown <= 0.0
                && !state.output.fired_modules.contains(&idx)
        });
        if let Some(idx) = ready {
            state.output.fired_modules.push(idx);
            1
        } else {
            0
        }
    })?;

    linker.func_wrap("env", "scan_orb", |caller: Caller<'_, BotState>| -> f32 {
        caller.data().input.nearest_orb_angle
    })?;

    linker.func_wrap(
        "env",
        "scan_from",
        |mut caller: Caller<'_, BotState>, id: i32, offset_deg: f32, fov_deg: f32| -> f32 {
            if id < 0 {
                return -1.0;
            }
            let state = caller.data_mut();
            let inp = &state.input;
            let Some(m) = inp.modules.get(id as usize) else {
                return -1.0;
            };
            if m.hp <= 0.0 || !matches!(m.kind, ModuleKind::Core | ModuleKind::Radar) {
                return -1.0;
            }

            let max_range = if m.kind == ModuleKind::Radar {
                1500.0
            } else {
                250.0
            };
            let (mx, my) = geom::local_to_world(
                inp.x,
                inp.y,
                inp.angle,
                m.gx as f32 * CELL_SIZE,
                m.gy as f32 * CELL_SIZE,
            );
            let scan_offset = m.dir + offset_deg;

            let nearest = inp
                .enemies
                .iter()
                .filter_map(|&(tx, ty)| {
                    geom::in_cone(mx, my, inp.angle, scan_offset, fov_deg, tx, ty)
                })
                .filter(|&d| d <= max_range)
                .reduce(f32::min)
                .unwrap_or(-1.0);

            state.output.scans.push((scan_offset, fov_deg));
            nearest
        },
    )?;

    linker.func_wrap(
        "env",
        "scan_enemy",
        |mut caller: Caller<'_, BotState>, offset_deg: f32, fov_deg: f32| -> f32 {
            let state = caller.data_mut();
            let inp = &state.input;
            let has_radar = inp
                .modules
                .iter()
                .any(|m| m.kind == ModuleKind::Radar && m.hp > 0.0);
            let max_range = if has_radar { 1500.0 } else { 250.0 };

            let nearest = inp
                .enemies
                .iter()
                .filter_map(|&(tx, ty)| {
                    geom::in_cone(inp.x, inp.y, inp.angle, offset_deg, fov_deg, tx, ty)
                })
                .filter(|&d| d <= max_range)
                .reduce(f32::min)
                .unwrap_or(-1.0);

            state.output.scans.push((offset_deg, fov_deg));
            nearest
        },
    )?;

    Ok(linker)
}
