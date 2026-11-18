//! Just Dire Things for Horizon — slice 1: the Simple Coal Generator.
//!
//! One `impl Hooks` at the crate root is the whole mod. [`Hooks::register`]
//! declares the generator's block entity type held by its block, its one fuel
//! slot and its insert-only FE buffer; a [`BlockEntityPart`] answers that
//! entity's server tick (burn fuel, store FE); `register_commands` /
//! `run_mod_command` add the `/justdirethings info|set` harness command.
//!
//! The arithmetic — the buffer, the burn, the fuel boundary — is [`runtime`],
//! the pure module this port has carried since its `.hnmod` bundle, with its
//! tests (`cargo test` runs them without the game).
//!
//! Energy: the buffer is the mod block's `Capabilities.Energy.BLOCK` domain
//! (`DefaultCapabilitiesRegistrar::block_energy`), whose amount is the int
//! `energy` in the block entity's saved compound, as upstream's
//! `EnergyStorageNoReceive` keeps it. Upstream's generator cannot be filled
//! from outside (`canReceive()` is false), so nothing may insert; every face
//! may drain it.
//!
//! Fuel: the burn time is the engine's own `cooking_fuel` resolution
//! (`ItemsHost::item_burn_time`, the counterpart of `stack.getBurnTime(SMELTING)`),
//! so vanilla and modded fuels work without a table of our own. The fuel's own
//! speed multiplier is read from the item's declared `minecraft:cooking_fuel`
//! component (see the gaps in `notes.md`: the SDK has no resolved accessor for
//! it).
mod runtime;

use mod_api::{
    self, BlockEntityPart, BlockEntityTickEvent, BlockEntityTickReply, BlockEntityTicksEvent,
    BlockPart, Command, CommandArgumentValue, CommandInvocation, Compound, DimensionKey, Flow,
    Hooks, LevelRef, ModContainer, ModEnergyBuffer, RegisterCommandsEvent, Registrar,
    RegistrationContext, RunModCommandEvent, ServerHost, Tag, Text,
};
use runtime::{CAPACITY, Fuel, Generator, clamp_energy};

/// The block this slice implements.
const GENERATOR: &str = "justdirethings:generatort1";
/// The block entity type this mod declares for it, in its own namespace.
const GENERATOR_TYPE: &str = "generatort1";
/// The fuel slot (upstream's `MACHINE_SLOTS = 1`).
const FUEL_SLOT: usize = 0;
/// The component a fuel carries its own burn time and speed in.
const COOKING_FUEL: &str = "minecraft:cooking_fuel";
/// What the status line answers where the block is not the generator.
const NO_BLOCK: &str = "No Just Dire Things block there.";

/// The generator's block entity behaviour: it ticks, and its tick burns.
struct GeneratorPart;

// A block entity Part is also the block's Part (the same type answers both), so
// the trait bound needs an implementation; every `BlockPart` method has its
// default here — this slice adds no block behaviour of its own.
#[mod_api::horizon_part]
impl BlockPart for GeneratorPart {}

#[mod_api::horizon_part]
impl BlockEntityPart for GeneratorPart {
    fn block_entity_ticks(&self, _: &dyn mod_api::Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let mut state = state_of(&event.entity.nbt);
        let mut slot = event.entity.items.get_mut(FUEL_SLOT).and_then(Option::take);
        let mut pulls = 0_u32;
        // The runtime pulls one fuel item at a time and consumes it as
        // upstream's `doBurn` does: one item, or the crafting remainder when
        // the stack runs out. It is called at most twice in a tick.
        let produced = state.tick(true, || {
            let stack = slot.as_ref()?;
            let fuel = fuel_of(host, stack)?;
            let count = stack.count();
            let remainder = host.item_crafting_remainder(stack);
            pulls += 1;
            slot = if count > 1 {
                let mut kept = stack.clone();
                kept.set_count(count - 1).ok()?;
                Some(kept)
            } else {
                remainder
            };
            Some(fuel)
        });
        if let Some(entry) = event.entity.items.get_mut(FUEL_SLOT) {
            *entry = slot;
        }
        store(&mut event.entity.nbt, &state);
        // Nothing burnt and nothing pulled: no payload to keep, so the chunk is
        // not dirtied (upstream's `setChanged` only runs when it changed).
        if produced == 0 && pulls == 0 {
            *event.reply = BlockEntityTickReply::Unchanged;
        }
    }
}

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        let generator = r.block_key(GENERATOR);
        let kind = r
            .block_entity_type(GENERATOR_TYPE)
            .blocks(&[GENERATOR])
            .build();
        let _ = r.block_entity_part(&kind, GeneratorPart);
        r.block_container(&generator, fuel_slot());
        r.block_energy(&generator, buffer());
    }

    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        event
            .commands
            .push(Command::new("justdirethings info <pos:block_pos>"));
        event.commands.push(Command::new(
            "justdirethings set <amount:integer> <pos:block_pos>",
        ));
    }

    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        let invocation = event.command;
        let Some(CommandArgumentValue::BlockPos(pos)) = invocation.argument("pos") else {
            return Flow::Continue;
        };
        let pos = *pos;
        match invocation.literals.get(1).map(String::as_str) {
            Some("info") => {
                let line = match level_of(host, invocation) {
                    Some(level) => status_text(host, level, pos),
                    None => NO_BLOCK.to_owned(),
                };
                feedback(host, line);
                Flow::Cancel
            }
            Some("set") => {
                let Some(CommandArgumentValue::Integer(amount)) = invocation.argument("amount")
                else {
                    feedback(host, "The amount must be a whole number of FE.".to_owned());
                    return Flow::Cancel;
                };
                let after = clamp_energy(*amount, CAPACITY);
                match set_energy(host, invocation, pos, after) {
                    Some(()) => feedback(host, format!("Energy set to {after} FE.")),
                    None => feedback(host, NO_BLOCK.to_owned()),
                }
                Flow::Cancel
            }
            _ => Flow::Continue,
        }
    }
}

/// The generator's fuel slot: one slot, reachable from every face, as upstream's
/// `GeneratorItemHandler`.
fn fuel_slot() -> ModContainer {
    ModContainer {
        slots: FUEL_SLOT + 1,
        faces: std::array::from_fn(|_| vec![FUEL_SLOT]),
        inputs: vec![FUEL_SLOT],
        outputs: vec![FUEL_SLOT],
        menu: None,
    }
}

/// The insert-only buffer. Upstream's storage is an `EnergyStorageNoReceive` of
/// `GENERATOR_T1_MAX_FE`, so nothing may fill it.
fn buffer() -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: CAPACITY,
        max_insert: 0,
        max_extract: CAPACITY,
        insert_faces: [false; 6],
        extract_faces: [true; 6],
    }
}

/// A compound is keyed by Java's lossless UTF-16 strings (`JavaString`), which
/// this API does not export by name, so a lookup spells the key the way Java
/// does and compares the stored units.
fn read_int(nbt: &Compound, key: &str) -> i32 {
    let units: Vec<u16> = key.encode_utf16().collect();
    match nbt.iter().find(|(name, _)| name.0 == units) {
        Some((_, Tag::Int(value))) => *value,
        _ => 0,
    }
}

fn write_int(nbt: &mut Compound, key: &str, value: i32) {
    nbt.insert(key.into(), Tag::Int(value));
}

/// The state as upstream saves it: `energy`, `feRemaining`, `burnRemaining`,
/// `maxBurn`, `fuelBurnMultiplier`.
fn state_of(nbt: &Compound) -> Generator {
    let mut state = Generator {
        energy: clamp_energy(read_int(nbt, "energy"), CAPACITY),
        fe_remaining: read_int(nbt, "feRemaining"),
        burn_remaining: read_int(nbt, "burnRemaining"),
        max_burn: read_int(nbt, "maxBurn"),
        fuel_multiplier: read_int(nbt, "fuelBurnMultiplier"),
    };
    // An old save without `fuelBurnMultiplier` reads 0; 1 is the default.
    if state.fuel_multiplier == 0 {
        state.fuel_multiplier = 1;
    }
    state.normalise();
    state
}

fn store(nbt: &mut Compound, state: &Generator) {
    write_int(nbt, "energy", state.energy);
    write_int(nbt, "feRemaining", state.fe_remaining);
    write_int(nbt, "burnRemaining", state.burn_remaining);
    write_int(nbt, "maxBurn", state.max_burn);
    write_int(nbt, "fuelBurnMultiplier", state.fuel_multiplier);
}

/// What the engine says one stack is worth as furnace fuel, `None` if it does
/// not burn (`ItemStack.getBurnTime(SMELTING) <= 0`).
fn fuel_of(host: &dyn ServerHost, stack: &mod_api::ItemStack) -> Option<Fuel> {
    let item = stack.item();
    let burn_time = host.item_burn_time(item)?;
    if burn_time <= 0 {
        return None;
    }
    Some(Fuel {
        burn_time: u32::try_from(burn_time).unwrap_or(0),
        speed_multiplier: speed_of(host, item),
    })
}

/// A fuel's own speed multiplier, as the item declares it in its
/// `minecraft:cooking_fuel` component (`speed_multiplier`, a Java float); vanilla
/// fuels declare none and are 1.0, which is what the runtime expects.
fn speed_of(host: &dyn ServerHost, item: mod_api::ItemKey) -> f32 {
    let Some(Tag::Compound(component)) = host.item_default_component(item, COOKING_FUEL) else {
        return 1.0;
    };
    let units: Vec<u16> = "speed_multiplier".encode_utf16().collect();
    match component.iter().find(|(name, _)| name.0 == units) {
        Some((_, Tag::Double(bits))) => f64::from_bits(*bits) as f32,
        Some((_, Tag::Float(bits))) => f32::from_bits(*bits),
        Some((_, Tag::Int(value))) => *value as f32,
        _ => 1.0,
    }
}

/// The level `invocation` executes in, from the dimension its source named.
fn level_of(host: &dyn ServerHost, invocation: &CommandInvocation) -> Option<LevelRef> {
    level_from(host, invocation.dimension)
}

fn level_from(host: &dyn ServerHost, dimension: DimensionKey) -> Option<LevelRef> {
    host.level(&dimension.id(host)?)
}

/// The generator at `pos`, if that is what stands there.
fn generator_at(
    host: &dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
) -> Option<mod_api::BlockEntityRef> {
    if level.block(host, pos)?.id(host) != GENERATOR {
        return None;
    }
    host.block_entity(level, pos)
}

fn stack_line(host: &dyn ServerHost, stack: Option<mod_api::ItemStack>) -> String {
    match stack {
        Some(stack) => format!("{} x{}", stack.item().id(host), stack.count()),
        None => "empty".to_owned(),
    }
}

/// A machine's status line. `t=` is the world game time, so a harness can tell
/// a fresh reply from a stale one.
fn status_text(host: &dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> String {
    let Some(entity) = generator_at(host, level, pos) else {
        return NO_BLOCK.to_owned();
    };
    let Some(nbt) = entity.data(host) else {
        return NO_BLOCK.to_owned();
    };
    let state = state_of(&nbt);
    let fuel = entity.item_slot(host, FUEL_SLOT as u32).ok().flatten();
    format!(
        "Simple Coal Generator at {},{},{}: {}/{} FE (burning {}, fe {} of {}, \
         burn {}/{}, multiplier {}, fuel {}, t={})",
        pos[0],
        pos[1],
        pos[2],
        state.energy,
        CAPACITY,
        state.burning(),
        state.fe_remaining,
        state.max_burn.saturating_mul(state.fe_per_tick()),
        state.burn_remaining,
        state.max_burn,
        state.fuel_multiplier,
        stack_line(host, fuel),
        level.game_time(host).unwrap_or(0)
    )
}

/// `/justdirethings set` (a harness control, not upstream gameplay): write the
/// stored FE, clamped to the capacity. `None` where the block is not ours.
fn set_energy(
    host: &mut dyn ServerHost,
    invocation: &CommandInvocation,
    pos: [i32; 3],
    energy: i32,
) -> Option<()> {
    let level = level_of(host, invocation)?;
    let entity = generator_at(host, level, pos)?;
    let mut data = Compound::new();
    write_int(&mut data, "energy", energy);
    entity.merge_data(host, data).ok()?;
    Some(())
}

fn feedback(host: &mut dyn ServerHost, line: String) {
    let _ = host.commands().feedback(Text::literal(line));
}
