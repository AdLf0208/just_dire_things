// Pure machine logic for Just Dire Things slice 1: the Simple Coal Generator.
//
// This file is both the standalone Cargo library and the injected
// `crate::game::justdirethings` module. It has no game types: the hook glue in
// `support.rs` turns a block entity, its fuel slot and the engine's
// `minecraft:cooking_fuel` values into the small values used here.
//
// Numbers are upstream 1.5.7's defaults (`Config.generatorT1Config`); see
// `FINDINGS.md` for the derivation.

/// The generator's buffer (`GENERATOR_T1_MAX_FE`, default 1,000,000 FE).
pub const CAPACITY: i32 = 1_000_000;
/// FE made per burn tick of fuel (`GENERATOR_T1_FE_PER_FUEL_TICK`, default 15).
pub const FE_PER_FUEL_TICK: i32 = 15;
/// The burn-speed multiplier from the config (`GENERATOR_T1_BURN_SPEED_MULTIPLIER`,
/// default 4): the burn time is divided by it and the output multiplied by it.
pub const BURN_SPEED_MULTIPLIER: i32 = 4;

/// One fuel item as the engine describes it: its burn time in ticks and the
/// `speed_multiplier` of its `minecraft:cooking_fuel` component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fuel {
    pub burn_time: u32,
    pub speed_multiplier: f32,
}

/// Energy clamped into a buffer.
#[must_use]
pub fn clamp_energy(energy: i32, capacity: i32) -> i32 {
    energy.clamp(0, capacity.max(0))
}

/// Upstream's `getBurnSpeedMultiplier`: the config's multiplier times the
/// fuel's own integer multiplier (vanilla fuel is 1, Primal Coal is 2).
#[must_use]
pub fn burn_speed(fuel_multiplier: i32) -> i32 {
    BURN_SPEED_MULTIPLIER.saturating_mul(fuel_multiplier.max(1))
}

/// The generator's saved state, keyed as upstream saves it (`energy`,
/// `feRemaining`, `burnRemaining`, `maxBurn`, `fuelBurnMultiplier`, plus the
/// redstone control's `redstoneMode`, `pulsed` and `receivingRedstone`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Generator {
    pub energy: i32,
    pub fe_remaining: i32,
    pub burn_remaining: i32,
    pub max_burn: i32,
    pub fuel_multiplier: i32,
    /// Upstream `RedstoneControlData`: the mode and the signal state it reads.
    pub redstone: RedstoneControl,
}

impl Default for Generator {
    fn default() -> Self {
        Self {
            energy: 0,
            fe_remaining: 0,
            burn_remaining: 0,
            max_burn: 0,
            fuel_multiplier: 1,
            redstone: RedstoneControl::default(),
        }
    }
}

impl Generator {
    /// The empty generator.
    #[must_use]
    #[allow(
        dead_code,
        reason = "pure API: the glue builds the state from NBT, and the game build does not compile the tests"
    )]
    pub fn new() -> Self {
        Self::default()
    }

    /// Upstream's gate: fuel burns only with some room in the buffer.
    #[must_use]
    pub fn room(&self) -> i32 {
        (CAPACITY - self.energy).max(0)
    }

    /// True while a fuel item is being burned (upstream's `feRemaining > 0`).
    #[must_use]
    pub fn burning(&self) -> bool {
        self.fe_remaining > 0
    }

    /// FE this tick would produce at the current fuel's speed.
    #[must_use]
    pub fn fe_per_tick(&self) -> i32 {
        FE_PER_FUEL_TICK.saturating_mul(burn_speed(self.fuel_multiplier))
    }

    /// Load one fuel item when no FE is left to burn. `next_fuel` takes the
    /// next item out of the slot and answers what it is worth, `None` when the
    /// slot is empty or holds something that does not burn.
    pub fn load_fuel(&mut self, next_fuel: &mut impl FnMut() -> Option<Fuel>) -> bool {
        if self.fe_remaining > 0 {
            return false;
        }
        self.max_burn = 0;
        self.burn_remaining = 0;
        if self.room() <= 0 {
            return false;
        }
        let Some(fuel) = next_fuel() else {
            return false;
        };
        if fuel.burn_time == 0 {
            return false;
        }
        // Upstream keeps an integer multiplier; the engine's value is a float
        // (vanilla 1.0, Primal Coal 2.0), so round it.
        self.fuel_multiplier = fuel.speed_multiplier.max(1.0).round() as i32;
        let speed = burn_speed(self.fuel_multiplier);
        self.fe_remaining =
            i32::try_from(i64::from(fuel.burn_time).saturating_mul(i64::from(FE_PER_FUEL_TICK)))
                .unwrap_or(i32::MAX);
        self.max_burn = (fuel.burn_time / u32::try_from(speed).unwrap_or(1)) as i32;
        self.burn_remaining = self.max_burn.saturating_add(1);
        true
    }

    /// One server tick: burn, then store what the buffer takes. Returns the FE
    /// stored this tick.
    ///
    /// `active` is upstream's `isActiveRedstone()`, asked where upstream asks it:
    /// once to start a burn and once to top the tick up when a burn ends inside
    /// it. It is a closure, not a `bool`, because `PULSE` spends its latch on
    /// each call — a `bool` would let one pulse pay twice.
    pub fn tick(
        &mut self,
        active: &mut impl FnMut() -> bool,
        mut next_fuel: impl FnMut() -> Option<Fuel>,
    ) -> i32 {
        if active() && !self.burning() {
            self.load_fuel(&mut next_fuel);
        }
        if !self.burning() {
            return 0;
        }
        let per_tick = self.fe_per_tick();
        let stored = per_tick.min(self.fe_remaining).min(self.room()).max(0);
        if stored > 0 {
            self.energy += stored;
            self.fe_remaining -= stored;
        }
        self.burn_remaining = self.burn_remaining.saturating_sub(1);
        let mut produced = stored;
        if !self.burning() && active() {
            // The burn ended inside this tick: upstream loads the next item and
            // tops the tick up, so output does not stutter between fuels — but
            // only while the gate allows it, which is why `active` is asked here
            // too (in `PULSE` the latch is already spent, so no top-up).
            self.load_fuel(&mut next_fuel);
            if stored < per_tick {
                let extra = (per_tick - stored)
                    .min(self.fe_remaining)
                    .min(self.room())
                    .max(0);
                if extra > 0 {
                    self.energy += extra;
                    self.fe_remaining -= extra;
                }
                produced += extra;
                self.burn_remaining = self.burn_remaining.saturating_sub(1);
            }
        }
        produced
    }

    /// Clamp what was read back from a save into a valid state.
    pub fn normalise(&mut self) {
        self.energy = clamp_energy(self.energy, CAPACITY);
        self.fe_remaining = self.fe_remaining.max(0);
        self.burn_remaining = self.burn_remaining.max(0);
        self.max_burn = self.max_burn.max(0);
        self.fuel_multiplier = self.fuel_multiplier.max(1);
    }
}

/// Upstream `MiscHelpers.RedstoneMode`: what a machine reads before it burns.
///
/// The order is upstream's, because the ordinal is the saved value: `IGNORED`,
/// `LOW`, `HIGH`, `PULSE` (`MiscHelpers.java`), confirmed by the jar's four GUI
/// button textures `redstone{ignore,low,high,pulse}.png`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RedstoneMode {
    /// Run whatever the signal says (upstream's default).
    #[default]
    Ignored,
    /// Run while no neighbour powers the machine.
    Low,
    /// Run while a neighbour powers the machine.
    High,
    /// Run one burn per rising edge of the signal.
    Pulse,
}

impl RedstoneMode {
    /// Every mode, in the order upstream saves them.
    pub const ALL: [Self; 4] = [Self::Ignored, Self::Low, Self::High, Self::Pulse];

    /// The ordinal upstream saves as `redstoneMode`.
    #[must_use]
    pub fn ordinal(self) -> i32 {
        match self {
            Self::Ignored => 0,
            Self::Low => 1,
            Self::High => 2,
            Self::Pulse => 3,
        }
    }

    /// The mode a saved ordinal names. Upstream indexes an array and would throw
    /// on a value out of range; a save that holds one falls back to the default
    /// rather than refusing to load the block.
    #[must_use]
    pub fn from_ordinal(ordinal: i32) -> Self {
        match ordinal {
            1 => Self::Low,
            2 => Self::High,
            3 => Self::Pulse,
            _ => Self::Ignored,
        }
    }

    /// The mode's name, for the status line and the harness command.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Ignored => "ignored",
            Self::Low => "low",
            Self::High => "high",
            Self::Pulse => "pulse",
        }
    }

    /// The mode a command's `<mode:word>` names, case-insensitively.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.name().eq_ignore_ascii_case(text))
    }
}

/// Upstream `RedstoneControlData`: the chosen mode, whether a neighbour powers the
/// machine, and the latched pulse `PULSE` is waiting to spend.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RedstoneControl {
    pub mode: RedstoneMode,
    pub receiving: bool,
    pub pulsed: bool,
}

impl RedstoneControl {
    /// Upstream `evaluateRedstone`: note whether a neighbour powers the machine,
    /// and latch one pulse on a rising edge in `PULSE` mode.
    pub fn sample(&mut self, powered: bool) {
        if self.mode == RedstoneMode::Pulse && !self.receiving && powered {
            self.pulsed = true;
        }
        self.receiving = powered;
    }

    /// Upstream `isActiveRedstone`: whether the machine may run this time it is
    /// asked. `PULSE` *spends* its latch here, so one rising edge buys one call.
    pub fn active(&mut self) -> bool {
        match self.mode {
            RedstoneMode::Ignored => true,
            RedstoneMode::Low => !self.receiving,
            RedstoneMode::High => self.receiving,
            RedstoneMode::Pulse => std::mem::take(&mut self.pulsed),
        }
    }
}

/// Whether a tick's payload must be kept, or may be discarded: upstream's
/// `setChanged` guard, as the block entity reply expresses it.
///
/// A tick that produced nothing and pulled no fuel has nothing to say about the
/// machine — unless it moved the redstone control, which is part of the same
/// saved state. A discarded falling edge means the next rising edge is never
/// seen, and `PULSE` then burns nothing again.
#[must_use]
pub fn persists(
    produced: i32,
    pulled: u32,
    before: RedstoneControl,
    after: RedstoneControl,
) -> bool {
    produced != 0 || pulled != 0 || before != after
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fuel(burn_time: u32, speed_multiplier: f32) -> Fuel {
        Fuel {
            burn_time,
            speed_multiplier,
        }
    }

    fn one(fuel_item: Fuel) -> impl FnMut() -> Option<Fuel> {
        let mut taken = false;
        move || {
            if taken {
                None
            } else {
                taken = true;
                Some(fuel_item)
            }
        }
    }

    #[test]
    fn the_defaults_are_upstreams() {
        assert_eq!(CAPACITY, 1_000_000);
        assert_eq!(FE_PER_FUEL_TICK, 15);
        assert_eq!(BURN_SPEED_MULTIPLIER, 4);
        assert_eq!(Generator::new().fuel_multiplier, 1);
        assert_eq!(Generator::new().fe_per_tick(), 60);
    }

    #[test]
    fn a_generator_with_no_fuel_makes_nothing() {
        let mut generator = Generator::new();
        let mut empty = || None;
        assert_eq!(generator.tick(&mut || true, &mut empty), 0);
        assert_eq!(generator.energy, 0);
        assert!(!generator.burning());
    }

    #[test]
    fn one_coal_yields_twenty_four_thousand_fe_over_four_hundred_ticks() {
        // Vanilla coal: 1,600 burn ticks, speed 1.0 → burn speed 4, 60 FE/t.
        let mut generator = Generator::new();
        let mut fuel = one(fuel(1_600, 1.0));
        let mut ticks = 0;
        let mut total = 0;
        loop {
            let made = generator.tick(&mut || true, &mut fuel);
            if made == 0 {
                break;
            }
            total += made;
            ticks += 1;
        }
        assert_eq!(ticks, 400, "1,600 / 4");
        assert_eq!(total, 24_000, "1,600 fuel ticks x 15 FE");
        assert_eq!(generator.energy, 24_000);
        assert_eq!(generator.fe_remaining, 0);
    }

    #[test]
    fn primal_coal_burns_twice_as_fast_for_three_times_the_energy() {
        // coal_t1: 4,800 burn ticks, speed 2.0 → burn speed 8, 120 FE/t.
        let mut generator = Generator::new();
        let mut fuel = one(fuel(4_800, 2.0));
        let mut ticks = 0;
        let mut total = 0;
        loop {
            let made = generator.tick(&mut || true, &mut fuel);
            if made == 0 {
                break;
            }
            total += made;
            ticks += 1;
        }
        assert_eq!(ticks, 600, "4,800 / 8");
        assert_eq!(total, 72_000, "4,800 fuel ticks x 15 FE");
        assert_eq!(generator.fuel_multiplier, 2);
    }

    #[test]
    fn a_full_buffer_burns_no_fuel() {
        let mut generator = Generator {
            energy: CAPACITY,
            ..Generator::new()
        };
        let mut calls = 0;
        let mut counter = || {
            calls += 1;
            Some(fuel(1_600, 1.0))
        };
        assert_eq!(generator.tick(&mut || true, &mut counter), 0);
        assert_eq!(calls, 0, "the item must not even be looked at");
        assert_eq!(generator.energy, CAPACITY);
    }

    #[test]
    fn a_nearly_full_buffer_keeps_the_rest_of_the_burn() {
        let mut generator = Generator {
            energy: CAPACITY - 10,
            ..Generator::new()
        };
        let mut fuel = one(fuel(1_600, 1.0));
        assert_eq!(
            generator.tick(&mut || true, &mut fuel),
            10,
            "only the room left"
        );
        assert_eq!(generator.energy, CAPACITY);
        assert!(
            generator.burning(),
            "the rest of the fuel stays for when there is room"
        );
        let left = generator.fe_remaining;
        assert_eq!(left, 24_000 - 10);
        // Nothing more fits, so nothing more burns.
        assert_eq!(generator.tick(&mut || true, &mut fuel), 0);
        assert_eq!(generator.fe_remaining, left);
    }

    #[test]
    fn one_coal_is_conserved_when_the_buffer_fills_up_and_drains() {
        let mut generator = Generator::new();
        let mut fuel = one(fuel(1_600, 1.0));
        let mut total = 0;
        for tick in 0..1_000 {
            // Drain the buffer every 100 ticks so it never fills for good.
            if tick % 100 == 99 {
                total += generator.energy;
                generator.energy = 0;
            }
            generator.tick(&mut || true, &mut fuel);
        }
        total += generator.energy;
        assert_eq!(total, 24_000, "no energy is destroyed at the buffer's edge");
    }

    #[test]
    fn output_continues_across_a_fuel_boundary_in_one_tick() {
        // Two 100-tick fuels are 1,500 FE each; at 60 FE/t that is 50 full
        // ticks, with the boundary tick paying a whole 60 rather than half.
        let mut generator = Generator::new();
        let mut remaining = 2;
        let mut fuel = move || {
            if remaining == 0 {
                return None;
            }
            remaining -= 1;
            Some(fuel(100, 1.0))
        };
        let mut made = Vec::new();
        for _ in 0..50 {
            made.push(generator.tick(&mut || true, &mut fuel));
        }
        assert_eq!(made.iter().sum::<i32>(), 3_000, "two 100-tick fuels");
        assert!(
            made.iter().all(|made| *made == 60),
            "the fuel boundary costs no tick: {made:?}"
        );
    }

    #[test]
    fn a_disabled_generator_burns_nothing() {
        let mut generator = Generator::new();
        let mut fuel = one(fuel(1_600, 1.0));
        assert_eq!(generator.tick(&mut || false, &mut fuel), 0);
        assert_eq!(generator.energy, 0);
        assert!(!generator.burning());
    }

    #[test]
    fn something_that_does_not_burn_is_not_consumed() {
        let mut generator = Generator::new();
        let mut empty = || None;
        assert!(!generator.load_fuel(&mut empty));
        assert_eq!(generator.fe_remaining, 0);
    }

    #[test]
    fn normalise_clamps_what_a_save_holds() {
        let mut generator = Generator {
            energy: -5,
            fe_remaining: -2,
            burn_remaining: -9,
            max_burn: -1,
            fuel_multiplier: 0,
            ..Generator::new()
        };
        generator.normalise();
        assert_eq!(generator.energy, 0);
        assert_eq!(generator.fe_remaining, 0);
        assert_eq!(generator.burn_remaining, 0);
        assert_eq!(generator.max_burn, 0);
        assert_eq!(generator.fuel_multiplier, 1);

        let mut over = Generator {
            energy: CAPACITY + 100,
            ..Generator::new()
        };
        over.normalise();
        assert_eq!(over.energy, CAPACITY);
    }

    // --- the redstone control (slice 2) ---

    #[test]
    fn the_four_modes_are_upstreams_and_round_trip() {
        // Upstream's declaration order is the saved ordinal: IGNORED, LOW, HIGH, PULSE.
        let names = RedstoneMode::ALL.map(RedstoneMode::name);
        assert_eq!(names, ["ignored", "low", "high", "pulse"]);
        for (ordinal, mode) in RedstoneMode::ALL.into_iter().enumerate() {
            assert_eq!(mode.ordinal(), ordinal as i32, "{mode:?} keeps its ordinal");
            assert_eq!(
                RedstoneMode::from_ordinal(ordinal as i32),
                mode,
                "{mode:?} round-trips through the save"
            );
            assert_eq!(RedstoneMode::parse(mode.name()), Some(mode));
        }
        assert_eq!(RedstoneMode::parse("HIGH"), Some(RedstoneMode::High));
        assert_eq!(RedstoneMode::parse("silly"), None);
        // The order is load-bearing on its own: the ordinal *is* the saved value.
        assert_eq!(RedstoneMode::ALL.len(), 4);
    }

    #[test]
    fn an_unknown_saved_mode_reads_as_ignored() {
        // Upstream indexes an array and would throw; a save from a newer build
        // must not refuse to load the block.
        assert_eq!(RedstoneMode::from_ordinal(-1), RedstoneMode::Ignored);
        assert_eq!(RedstoneMode::from_ordinal(9), RedstoneMode::Ignored);
        assert_eq!(RedstoneMode::default(), RedstoneMode::Ignored);
    }

    #[test]
    fn ignored_runs_whatever_the_signal() {
        let mut control = RedstoneControl::default();
        assert!(control.active(), "IGNORED runs with no signal");
        control.sample(true);
        assert!(control.active(), "IGNORED runs with a signal");
    }

    #[test]
    fn low_runs_unpowered_high_runs_powered() {
        let mut low = RedstoneControl {
            mode: RedstoneMode::Low,
            ..RedstoneControl::default()
        };
        assert!(low.active(), "LOW runs with no signal");
        low.sample(true);
        assert!(
            !low.active(),
            "LOW must not run while a neighbour powers it"
        );
        low.sample(false);
        assert!(low.active(), "LOW runs again when the signal goes");

        let mut high = RedstoneControl {
            mode: RedstoneMode::High,
            ..RedstoneControl::default()
        };
        assert!(
            !high.active(),
            "HIGH with no signal must not run: nothing is powering it"
        );
        high.sample(true);
        assert!(high.active(), "HIGH runs while powered");
        high.sample(false);
        assert!(!high.active(), "HIGH stops when the signal goes");
    }

    #[test]
    fn pulse_runs_once_per_rising_edge_and_ignores_a_held_signal() {
        let mut control = RedstoneControl {
            mode: RedstoneMode::Pulse,
            ..RedstoneControl::default()
        };
        assert!(!control.active(), "PULSE with no edge yet does not run");
        control.sample(true);
        assert!(control.active(), "the rising edge runs the machine once");
        assert!(!control.active(), "the pulse is spent after one run");
        control.sample(true);
        assert!(
            !control.active(),
            "a signal that stays on rises once, so it runs no more"
        );
        control.sample(false);
        assert!(!control.active(), "a falling edge runs nothing");
        control.sample(true);
        assert!(control.active(), "the next rising edge runs it again");
    }

    /// A generator wired to its own redstone control, as the glue wires it: the
    /// signal is sampled once a tick and the gate is the control's answer.
    struct Rig {
        generator: Generator,
    }

    impl Rig {
        fn with(mode: RedstoneMode) -> Self {
            Self {
                generator: Generator {
                    redstone: RedstoneControl {
                        mode,
                        ..RedstoneControl::default()
                    },
                    ..Generator::new()
                },
            }
        }

        fn tick(&mut self, powered: bool, next_fuel: impl FnMut() -> Option<Fuel>) -> i32 {
            self.generator.redstone.sample(powered);
            let mut control = self.generator.redstone;
            let made = self.generator.tick(&mut || control.active(), next_fuel);
            self.generator.redstone = control;
            made
        }
    }

    /// Two fuels that record how many were taken.
    fn counted(
        fuel_item: Fuel,
    ) -> (
        impl FnMut() -> Option<Fuel>,
        std::rc::Rc<std::cell::Cell<u32>>,
    ) {
        let taken = std::rc::Rc::new(std::cell::Cell::new(0));
        let seen = std::rc::Rc::clone(&taken);
        let mut left = 2;
        let next = move || {
            if left == 0 {
                return None;
            }
            left -= 1;
            seen.set(seen.get() + 1);
            Some(fuel_item)
        };
        (next, taken)
    }

    #[test]
    fn a_pulse_starts_one_burn_that_then_runs_its_course() {
        // A stick's worth: 100 burn ticks at speed 4 → 25 ticks of 60 FE.
        let mut rig = Rig::with(RedstoneMode::Pulse);
        let (mut fuels, taken) = counted(fuel(100, 1.0));
        assert_eq!(rig.tick(true, &mut fuels), 60, "the rising edge starts it");
        assert_eq!(taken.get(), 1, "one fuel is taken");
        // The signal stays on with no further edge: the burn it started keeps
        // paying, because upstream only consults the gate to *start* a burn.
        let mut total = 60;
        for _ in 0..24 {
            total += rig.tick(true, &mut fuels);
        }
        assert_eq!(total, 1_500, "100 burn ticks x 15 FE");
        assert_eq!(rig.tick(true, &mut fuels), 0, "the burn is over");
        assert_eq!(taken.get(), 1, "no second fuel without a new edge");
    }

    #[test]
    fn no_new_burn_starts_without_the_next_edge() {
        let mut rig = Rig::with(RedstoneMode::Pulse);
        let (mut fuels, taken) = counted(fuel(100, 1.0));
        for _ in 0..20 {
            rig.tick(false, &mut fuels);
        }
        assert_eq!(taken.get(), 0, "an unpowered pulse machine burns nothing");
        assert_eq!(rig.generator.energy, 0);
        // A rising edge buys one burn; after it is spent, twenty more powered
        // ticks buy none.
        rig.tick(true, &mut fuels);
        for _ in 0..20 {
            rig.tick(true, &mut fuels);
        }
        assert_eq!(taken.get(), 1, "the held signal is not a series of pulses");
        // An edge that arrives while the first burn is still paying is spent on
        // nothing: upstream asks the gate to *start* a burn, and this tick is
        // not one. The fuel stays put.
        rig.tick(false, &mut fuels);
        for _ in 0..10 {
            rig.tick(true, &mut fuels);
        }
        assert_eq!(
            taken.get(),
            1,
            "an edge inside a burn starts no second fuel"
        );
        // Once the burn is over, the next rising edge does start one.
        for _ in 0..20 {
            rig.tick(true, &mut fuels);
        }
        assert!(!rig.generator.burning(), "the first burn has ended");
        rig.tick(false, &mut fuels);
        for _ in 0..25 {
            rig.tick(true, &mut fuels);
        }
        assert_eq!(taken.get(), 2, "the next rising edge starts the next fuel");
    }

    #[test]
    fn high_mode_keeps_an_in_progress_burn_and_starts_no_new_one_when_power_drops() {
        let mut rig = Rig::with(RedstoneMode::High);
        let (mut fuels, taken) = counted(fuel(100, 1.0));
        assert_eq!(
            rig.tick(false, &mut fuels),
            0,
            "unpowered HIGH does not start"
        );
        assert_eq!(taken.get(), 0);
        assert_eq!(rig.tick(true, &mut fuels), 60, "powered HIGH burns");
        // The power drops mid-burn: the burn finishes, no new fuel is taken.
        let mut total = 60;
        for _ in 0..40 {
            total += rig.tick(false, &mut fuels);
        }
        assert_eq!(total, 1_500, "the burn that started runs to its end");
        assert_eq!(taken.get(), 1, "the drop in power starts nothing new");
    }

    #[test]
    fn a_pulse_does_not_top_up_the_tick_a_burn_ends_in() {
        // 110 burn ticks are 1,650 FE: 27 full 60 FE ticks and a last one of 30.
        // Upstream asks `isActiveRedstone()` again for the top-up, and in PULSE
        // the latch is already spent, so the boundary tick pays only its 30.
        let mut rig = Rig::with(RedstoneMode::Pulse);
        let (mut fuels, taken) = counted(fuel(110, 1.0));
        for _ in 0..27 {
            assert_eq!(rig.tick(true, &mut fuels), 60, "a full tick");
        }
        assert_eq!(
            rig.tick(true, &mut fuels),
            30,
            "the boundary tick, no top-up"
        );
        assert_eq!(rig.generator.energy, 1_650, "the whole fuel is still paid");
        assert_eq!(taken.get(), 1, "the spare fuel was not pulled in to top up");
    }

    #[test]
    fn ignored_still_tops_up_the_tick_a_burn_ends_in() {
        // The same two fuels without a mode: the boundary tick pays a whole 60,
        // which is the behaviour the slice must not regress.
        let mut rig = Rig::with(RedstoneMode::Ignored);
        let (mut fuels, taken) = counted(fuel(110, 1.0));
        for _ in 0..27 {
            assert_eq!(rig.tick(true, &mut fuels), 60);
        }
        assert_eq!(rig.tick(true, &mut fuels), 60, "IGNORED tops the tick up");
        assert_eq!(taken.get(), 2, "the second fuel pays the rest");
    }

    #[test]
    fn a_falling_edge_is_kept_even_when_nothing_burned() {
        // The signal drops while the machine has no fuel: nothing is produced and
        // the slot does not move, but the saved `receiving` must still be written
        // back — otherwise the next rising edge is not a rising edge any more and
        // `PULSE` never burns its second fuel.
        let before = RedstoneControl {
            mode: RedstoneMode::Pulse,
            receiving: true,
            pulsed: false,
        };
        let after = RedstoneControl {
            receiving: false,
            ..before
        };
        assert!(
            persists(0, 0, before, after),
            "a falling edge with no burn must still be written back"
        );
        assert!(
            !persists(0, 0, before, before),
            "a tick that changed nothing is not written back"
        );
        assert!(persists(60, 1, before, before), "a burn is written back");
        assert!(
            persists(0, 1, before, before),
            "a fuel pulled but not yet paid is written back"
        );
    }
}
