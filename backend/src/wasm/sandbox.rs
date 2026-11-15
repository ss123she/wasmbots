use anyhow::Result;
use wasmtime::{
    Config, Engine, Instance, Linker, Module, Store, StoreLimits, StoreLimitsBuilder, TypedFunc,
};

use crate::arena::modules::{
    ModuleSnapshot, ModuleSpec, STARTING_CREDITS, core_spec, default_blueprint,
};

pub const BASE_FUEL_PER_TICK: u64 = 15_000;

pub struct BotInput {
    pub hp: f32,
    pub x: f32,
    pub y: f32,
    pub angle: f32,
    pub cooldown: f32,
    pub nearest_orb_angle: f32,
    pub enemies: Vec<(f32, f32)>,
    pub modules: Vec<ModuleSnapshot>,
}

#[derive(Default)]
pub struct BotOutput {
    pub speed: f32,
    pub turn: f32,
    pub fired_modules: Vec<usize>,
    pub scans: Vec<(f32, f32)>,
}

pub struct BotState {
    pub limits: StoreLimits,
    pub building: bool,
    pub credits: i32,
    pub blueprint: Vec<ModuleSpec>,
    pub input: BotInput,
    pub output: BotOutput,
}

pub struct BotRuntime {
    store: Store<BotState>,
    tick_fn: TypedFunc<(), ()>,
    pub blueprint: Vec<ModuleSpec>,
    pub overheated_ticks: u32,
    pub last_fuel_used: u64,
    pub last_fuel_budget: u64,
}

pub fn create_engine() -> Result<Engine> {
    Ok(Engine::new(
        &Config::new().consume_fuel(true).max_wasm_stack(256 * 1024),
    )?)
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

        let core = core_spec();
        let state = BotState {
            limits,
            building: true,
            credits: STARTING_CREDITS,
            blueprint: vec![core],
            input: BotInput {
                hp: core.max_hp,
                x: 500.0,
                y: 500.0,
                angle: 0.0,
                cooldown: 0.0,
                nearest_orb_angle: 999.0,
                enemies: Vec::new(),
                modules: Vec::new(),
            },
            output: BotOutput::default(),
        };

        let mut store = Store::new(engine, state);
        store.limiter(|s| &mut s.limits);
        store.set_fuel(100_000)?;

        let instance: Instance = linker.instantiate(&mut store, &module)?;
        let tick_fn = instance.get_typed_func::<(), ()>(&mut store, "tick")?;

        if let Ok(build_fn) = instance.get_typed_func::<(), ()>(&mut store, "build") {
            let _ = build_fn.call(&mut store, ());
        } else {
            store.data_mut().blueprint = default_blueprint();
        }
        store.data_mut().building = false;

        let blueprint = store.data().blueprint.clone();

        Ok(Self {
            store,
            tick_fn,
            blueprint,
            overheated_ticks: 0,
            last_fuel_used: 0,
            last_fuel_budget: BASE_FUEL_PER_TICK,
        })
    }

    pub fn step(&mut self, input: BotInput, fuel_budget: u64) -> BotOutput {
        self.last_fuel_budget = fuel_budget;
        if self.overheated_ticks > 0 {
            self.overheated_ticks -= 1;
            self.last_fuel_used = fuel_budget;
            return BotOutput::default();
        }

        let state = self.store.data_mut();
        state.input = input;
        state.output = BotOutput::default();

        let _ = self.store.set_fuel(fuel_budget);
        match self.tick_fn.call(&mut self.store, ()) {
            Ok(()) => {
                let left = self.store.get_fuel().unwrap_or(0);
                self.last_fuel_used = fuel_budget.saturating_sub(left);
            }
            Err(_) => {
                self.last_fuel_used = fuel_budget;
                self.overheated_ticks = 15;
            }
        }

        std::mem::take(&mut self.store.data_mut().output)
    }
}
