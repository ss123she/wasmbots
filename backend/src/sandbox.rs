use anyhow::Result;
use wasmtime::{
    Caller, Config, Engine, Instance, Linker, Module, Store, StoreLimits, StoreLimitsBuilder,
    TypedFunc,
};

use crate::geom;

pub struct BotInput {
    pub hp: f32,
    pub x: f32,
    pub y: f32,
    pub angle: f32,
    pub cooldown: f32,
    pub nearest_orb_angle: f32,
    pub enemies: Vec<(f32, f32)>,
}

#[derive(Default)]
pub struct BotOutput {
    pub speed: f32,
    pub turn: f32,
    pub fired: bool,
    pub scans: Vec<(f32, f32)>, // (angle_offset, fov)
}

pub struct BotState {
    pub limits: StoreLimits,
    pub input: BotInput,
    pub output: BotOutput,
}

pub struct BotRuntime {
    store: Store<BotState>,
    tick_fn: TypedFunc<(), ()>,
    pub overheated_ticks: u32,
    pub last_fuel_used: u64,
}

pub const FUEL_PER_TICK: u64 = 15_000;

pub fn create_engine() -> Result<Engine> {
    Ok(Engine::new(
        &Config::new().consume_fuel(true).max_wasm_stack(256 * 1024),
    )?)
}

pub fn create_linker(engine: &Engine) -> Result<Linker<BotState>> {
    let mut linker = Linker::new(engine);

    linker.func_wrap(
        "env",
        "get_self",
        |caller: Caller<'_, BotState>, stat: i32| -> f32 {
            let inp = &caller.data().input;
            match stat {
                0 => inp.hp,
                1 => inp.x,
                2 => inp.y,
                3 => inp.angle,
                4 => inp.cooldown,
                _ => 0.0,
            }
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

    linker.func_wrap("env", "fire", |mut caller: Caller<'_, BotState>| -> i32 {
        if caller.data().input.cooldown <= 0.0 {
            caller.data_mut().output.fired = true;
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
        "scan_enemy",
        |mut caller: Caller<'_, BotState>, offset_deg: f32, fov_deg: f32| -> f32 {
            let inp = &caller.data().input;
            let nearest = inp
                .enemies
                .iter()
                .filter_map(|&(tx, ty)| {
                    geom::in_cone(inp.x, inp.y, inp.angle, offset_deg, fov_deg, tx, ty)
                })
                .reduce(f32::min)
                .unwrap_or(-1.0);

            caller.data_mut().output.scans.push((offset_deg, fov_deg));
            nearest
        },
    )?;

    Ok(linker)
}

impl BotRuntime {
    pub fn from_bytes(
        engine: &Engine,
        linker: &Linker<BotState>,
        wasm_bytes: &[u8],
    ) -> Result<Self> {
        let module = Module::new(engine, wasm_bytes)?;
        let limits = StoreLimitsBuilder::new()
            .memory_size(2 * 1024 * 1024)
            .instances(1)
            .build();

        let state = BotState {
            limits,
            input: BotInput {
                hp: 100.0,
                x: 500.0,
                y: 500.0,
                angle: 0.0,
                cooldown: 0.0,
                nearest_orb_angle: 999.0,
                enemies: Vec::new(),
            },
            output: BotOutput::default(),
        };

        let mut store = Store::new(engine, state);
        store.limiter(|s| &mut s.limits);
        store.set_fuel(FUEL_PER_TICK)?;

        let instance: Instance = linker.instantiate(&mut store, &module)?;
        let tick_fn = instance.get_typed_func::<(), ()>(&mut store, "tick")?;

        Ok(Self {
            store,
            tick_fn,
            overheated_ticks: 0,
            last_fuel_used: 0,
        })
    }

    pub fn step(&mut self, input: BotInput) -> BotOutput {
        if self.overheated_ticks > 0 {
            self.overheated_ticks -= 1;
            self.last_fuel_used = FUEL_PER_TICK;
            return BotOutput::default();
        }

        let state = self.store.data_mut();
        state.input = input;
        state.output = BotOutput::default();

        let _ = self.store.set_fuel(FUEL_PER_TICK);
        match self.tick_fn.call(&mut self.store, ()) {
            Ok(()) => {
                let left = self.store.get_fuel().unwrap_or(0);
                self.last_fuel_used = FUEL_PER_TICK.saturating_sub(left);
            }
            Err(_) => {
                // OutOfFuel or Trap.
                self.last_fuel_used = FUEL_PER_TICK;
                self.overheated_ticks = 15;
            }
        }

        std::mem::replace(&mut self.store.data_mut().output, BotOutput::default())
    }
}
