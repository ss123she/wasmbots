use wasmtime::{Engine, Linker};

use crate::{
    arena::{
        components::{Bot, Hull, Pos},
        snapshot::ArenaSnapshot,
        systems,
    },
    wasm::{
        host::create_linker,
        sandbox::{BotRuntime, BotState, create_engine},
    },
};

pub struct World {
    engine: Engine,
    linker: Linker<BotState>,
    ecs: hecs::World,
    tick: u64,
    next_color: usize,
}

impl World {
    pub fn new() -> anyhow::Result<Self> {
        let engine = create_engine()?;
        let linker = create_linker(&engine)?;
        Ok(Self {
            engine,
            linker,
            ecs: hecs::World::new(),
            tick: 0,
            next_color: 0,
        })
    }

    pub fn spawn_bot(&mut self, name: &str, wasm_bytes: &[u8]) -> anyhow::Result<()> {
        let runtime = BotRuntime::from_bytes(&self.engine, &self.linker, wasm_bytes)?;
        let modules = runtime.blueprint.clone();
        let colors = [
            "#38bdf8", "#a855f7", "#f97316", "#ec4899", "#22d3ee", "#eab308",
        ];
        let color = colors[self.next_color % colors.len()].to_string();
        self.next_color += 1;

        self.ecs.spawn((
            Pos(systems::random_pos(100.0, 900.0)),
            Bot {
                name: name.to_string(),
                color,
                angle: rand::random_range(-180.0..180.0),
                kills: 0,
                overclock_ticks: 0,
                runtime,
                last_scans: Vec::new(),
            },
            Hull(modules),
        ));

        Ok(())
    }

    pub fn step(&mut self) -> ArenaSnapshot {
        self.tick += 1;
        let mut events = Vec::new();

        systems::spawn_orbs(&mut self.ecs, self.tick);
        systems::step_bots(&mut self.ecs);
        systems::separate_bots(&mut self.ecs);
        systems::step_bullets(&mut self.ecs, &mut events);
        systems::step_orbs(&mut self.ecs, &mut events);
        systems::respawn_dead(&mut self.ecs);

        systems::build_snapshot(&self.ecs, self.tick, events)
    }
}
