use crate::{
    crafting,
    entities::player::Player,
    exploration::{Exploration, ScanReport},
    input::Action,
    inventory::{Inventory, Item},
    planet::Camp,
    survival::{Environment, NIGHT_TICKS, Survival},
    world::{World, seed::planet_name},
};
use std::{collections::VecDeque, time::Duration};
pub const STEP: Duration = Duration::from_millis(250);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    MainMenu,
    NewWorld,
    Playing,
    Paused,
    Inventory,
    Crafting,
    Help,
    Dead,
    Quit,
}
pub struct Game {
    pub state: State,
    pub world: World,
    pub player: Player,
    pub survival: Survival,
    pub planet: String,
    pub message: String,
    pub messages: VecDeque<String>,
    pub exploration: Exploration,
    pub last_scan: Option<ScanReport>,
    pub inventory: Inventory,
    pub camp: Camp,
    pub selected: usize,
    pub started: bool,
    return_state: State,
    accumulator: Duration,
}
impl Game {
    pub fn new(seed: u64) -> Self {
        let mut world = World::new(seed, 128);
        let (x, y) = world.landing_site();
        world.focus(x, y);
        let message = "Crash landing. E beside A: emergency cache. H: survival guide.".to_string();
        Self {
            state: State::MainMenu,
            world,
            player: Player { x, y },
            survival: Survival::default(),
            planet: planet_name(seed),
            messages: VecDeque::from([message.clone()]),
            message,
            exploration: Exploration::default(),
            last_scan: None,
            inventory: Inventory::default(),
            camp: Camp::new((x, y)),
            selected: 0,
            started: false,
            return_state: State::Playing,
            accumulator: Duration::ZERO,
        }
    }
    pub fn say(&mut self, message: impl Into<String>) {
        self.message = message.into();
        self.messages.push_back(self.message.clone());
        if self.messages.len() > 6 {
            self.messages.pop_front();
        }
    }
    pub fn environment(&mut self) -> Environment {
        let p = Camp::point(self.player);
        Environment {
            biome: self.camp.tile(&mut self.world, p.0, p.1).biome,
            sheltered: self.camp.sheltered(p),
            protected: self.inventory.count(Item::ThermalSuit) > 0,
            powered: self.camp.powered(p),
        }
    }
    pub fn objective(&self) -> &'static str {
        if self.camp.completed {
            "COMPLETE: survived a full night!"
        } else if !self.camp.cache_claimed {
            "E beside A: scavenge supplies"
        } else if self.inventory.count(Item::MiningTool) == 0 {
            "Gather iron/carbon; C: mining tool"
        } else if self.camp.shelters.is_empty() {
            "Gather crystal; C: shelter kit; B: build"
        } else {
            "Stay on H for one full night (80s-120s)"
        }
    }
    pub fn help_lines(&self) -> Vec<String> {
        let game = self;
        let mut lines = vec![
            format!("Goal: {}", game.objective()),
            "CRASH -> SCAVENGE -> GATHER -> CRAFT -> SHELTER -> NIGHT".into(),
            "E beside A: cache (pick, 2 oxygen canisters, battery).".into(),
            "Start: i iron two west; k carbon two north; v crystal two east/two south.".into(),
            "Stand on a deposit; E gathers 2 units (12 per cell).".into(),
            "Carbon: hands. Iron/copper/silica: pick. Crystal: mining tool.".into(),
            "C: recipes. Up/Down select, Enter craft. Ingredients shown below.".into(),
            "Craft mining tool, harvest crystal, craft shelter kit, B builds here.".into(),
            "Stay on H for a whole night: night 80s-120s; daylight 0s-80s.".into(),
            "I: inventory. Up/Down select; Enter use. X discards entire stack.".into(),
            "Battery near A/H powers oxygen for 120s. Tools/suits work when carried.".into(),
            "O uses a canister (+50 O2); O2 drains 1 every 3s away from power.".into(),
            "At zero O2 lose 3 HP/sec. Low O2 warning at 25. Shelters heal slowly.".into(),
            "Night is 20C colder. Thermal suit protects -40..65C and wasteland radiation.".into(),
            "A/H stations refill O2 nearby when powered. Stand ON H for protection.".into(),
            "WASD/arrows move; R scans. P pauses; I/C/H screens also pause.".into(),
            "F5 saves; F9/L loads (paused). Q/Esc quits and auto-saves.".into(),
            "Backspace closes screens. Up/Down scroll help. No time passes in menus.".into(),
            "Recent events:".into(),
        ];
        lines.extend(game.messages.iter().cloned());
        lines
    }
    pub fn handle(&mut self, action: Action) -> bool {
        let before = self.state;
        if action == Action::Quit {
            self.state = State::Quit;
            return true;
        }
        if matches!(self.state, State::Inventory | State::Crafting | State::Help) {
            match action {
                Action::Back | Action::Pause => self.state = self.return_state,
                Action::Inventory if self.state == State::Inventory => {
                    self.state = self.return_state
                }
                Action::Craft if self.state == State::Crafting => self.state = self.return_state,
                Action::Help if self.state == State::Help => self.state = self.return_state,
                Action::Move(_, dy) => {
                    let count = match self.state {
                        State::Inventory => self.inventory.stacks().len(),
                        State::Crafting => crafting::RECIPES.len(),
                        _ => self.help_lines().len(),
                    };
                    if dy < 0 {
                        self.selected = self.selected.saturating_sub(1);
                    } else if dy > 0 {
                        self.selected = (self.selected + 1).min(count.saturating_sub(1));
                    }
                }
                Action::Accept if self.state == State::Inventory => self.use_selected(),
                Action::Discard if self.state == State::Inventory => {
                    if let Some(stack) = self.inventory.discard(self.selected) {
                        self.say(format!("Discarded {} x{}.", stack.item.name(), stack.count));
                    }
                    self.selected = self
                        .selected
                        .min(self.inventory.stacks().len().saturating_sub(1));
                }
                Action::Accept if self.state == State::Crafting => {
                    match crafting::craft(&mut self.inventory, self.selected) {
                        Ok(item) => self.say(format!("Crafted {}.", item.name())),
                        Err(e) => self.say(e),
                    }
                }
                _ => return false,
            }
            return true;
        }
        match (self.state, action) {
            (State::MainMenu | State::Dead, Action::Accept | Action::NewWorld) => {
                self.state = State::NewWorld
            }
            (State::NewWorld, Action::Accept) => {
                *self = Self::new(self.world.seed());
                self.state = State::Playing;
                self.started = true;
                self.observe(false);
            }
            (State::Playing | State::Paused, Action::Inventory | Action::Craft | Action::Help) => {
                self.return_state = self.state;
                self.selected = 0;
                self.state = match action {
                    Action::Inventory => State::Inventory,
                    Action::Craft => State::Crafting,
                    _ => State::Help,
                };
            }
            (State::Playing, Action::Move(dx, dy)) => {
                if self.player.step_if(dx, dy, |x, y| {
                    self.camp.tile(&mut self.world, x, y).biome.passable()
                }) {
                    self.world.focus(self.player.x, self.player.y);
                    self.survival.energy = self.survival.energy.saturating_sub(1);
                    self.observe(false);
                    let p = Camp::point(self.player);
                    let message = if self.camp.sheltered(p) {
                        "Inside shelter: protected. Use battery to power oxygen."
                    } else if p == self.camp.ship() {
                        "Damaged spacecraft. E: scavenge or power oxygen station."
                    } else {
                        self.camp
                            .tile(&mut self.world, p.0, p.1)
                            .landmark
                            .map_or("Exploring the surface.", |s| s.name())
                    };
                    self.say(message);
                } else {
                    self.say("Route blocked: water, mountains or world boundary.");
                }
                return true;
            }
            (State::Playing, Action::Interact) => {
                self.interact();
                return true;
            }
            (State::Playing, Action::Build) => {
                let result = self.camp.build(
                    &mut self.world,
                    &mut self.inventory,
                    Camp::point(self.player),
                );
                self.say(match result {
                    Ok(()) => {
                        "Shelter built! Stay on H through a full night. Battery powers oxygen."
                    }
                    Err(e) => e,
                });
                return true;
            }
            (State::Playing, Action::UseOxygen) => {
                self.use_item(Item::OxygenCanister);
                return true;
            }
            (State::Playing, Action::Scan) => {
                let report = self.observe(true);
                self.say(format!(
                    "Scan: +{} cells, +{} sites. {}",
                    report.new_cells,
                    report.new_sites,
                    report
                        .sites
                        .first()
                        .map_or("No signals nearby.".into(), |s| format!(
                            "{} at {},{}",
                            s.kind.name(),
                            s.x,
                            s.y
                        ))
                ));
                self.last_scan = Some(report);
                return true;
            }
            (State::Playing, Action::Pause) => self.state = State::Paused,
            (State::Paused, Action::Pause | Action::Accept) => self.state = State::Playing,
            (State::Paused | State::NewWorld, Action::Menu) => self.state = State::MainMenu,
            _ => {}
        }
        if self.state != before {
            self.accumulator = Duration::ZERO;
        }
        self.state != before
    }
    fn use_selected(&mut self) {
        if let Some(stack) = self.inventory.stacks().get(self.selected) {
            self.use_item(stack.item);
        }
        self.selected = self
            .selected
            .min(self.inventory.stacks().len().saturating_sub(1));
    }
    fn use_item(&mut self, item: Item) {
        let result = match item {
            Item::OxygenCanister => {
                if self.survival.oxygen >= 100 {
                    Err("Oxygen full; canister preserved")
                } else {
                    self.inventory.remove(item, 1).map(|()| {
                        self.survival.oxygen = self.survival.oxygen.saturating_add(50).min(100);
                    })
                }
            }
            Item::Battery => self
                .camp
                .power_station(&mut self.inventory, Camp::point(self.player)),
            _ => {
                self.say("Tools and suits work while carried. Use B to place a shelter kit.");
                return;
            }
        };
        self.say(match result {
            Ok(()) => {
                if item == Item::Battery {
                    "Oxygen station powered for 120 more seconds."
                } else {
                    "Oxygen replenished (+50)."
                }
            }
            Err(e) => e,
        });
    }
    fn interact(&mut self) {
        let p = Camp::point(self.player);
        if crate::planet::near(p, self.camp.ship()) && !self.camp.cache_claimed {
            let result = self.camp.scavenge(&mut self.inventory);
            self.say(match result {Ok(())=>"Cache: salvage pick, 2 oxygen canisters, 1 battery. Gather i/k, then craft (C).",Err(e)=>e});
        } else if matches!(
            self.camp.tile(&mut self.world, p.0, p.1).landmark,
            Some(crate::world::landmark::Landmark::Deposit(_))
        ) {
            match self.camp.harvest(&mut self.world, &mut self.inventory, p) {
                Ok((resource, n)) => {
                    self.survival.energy = self.survival.energy.saturating_sub(2);
                    self.say(format!(
                        "Harvested {} x{n}. Remaining {}.",
                        resource.name(),
                        crate::planet::DEPOSIT_UNITS - self.camp.harvested[&p]
                    ));
                }
                Err(e) => self.say(e),
            }
        } else if self.camp.sheltered(p) || crate::planet::near(p, self.camp.ship()) {
            self.use_item(Item::Battery);
        } else {
            self.say("E: stand on a deposit, beside A, or inside H. R scans; H opens help.");
        }
    }
    pub fn observe(&mut self, scan: bool) -> ScanReport {
        self.exploration
            .reveal_with(self.player.x, self.player.y, scan, |x, y| {
                self.camp.site(&mut self.world, x, y)
            })
    }
    /// Four fixed updates maximum; discard excess elapsed time after a stall.
    pub fn advance(&mut self, elapsed: Duration) -> bool {
        if self.state != State::Playing {
            return false;
        }
        self.accumulator += elapsed.min(STEP * 4);
        let mut redraw = false;
        for _ in 0..4 {
            if self.accumulator < STEP {
                break;
            }
            self.accumulator -= STEP;
            let was_night = self.survival.night();
            let env = self.environment();
            redraw |= self.survival.tick(env);
            if self.survival.night() {
                if env.sheltered {
                    self.camp.night_progress = (self.camp.night_progress + 1).min(NIGHT_TICKS);
                } else {
                    self.camp.night_progress = 0;
                }
            } else if was_night {
                if self.camp.night_progress >= NIGHT_TICKS
                    && self.survival.health > 0
                    && !self.camp.completed
                {
                    self.camp.completed = true;
                    self.say("OBJECTIVE COMPLETE: you survived a full night in your shelter!");
                    redraw = true;
                }
                self.camp.night_progress = 0;
            }
            if self.survival.ticks().is_multiple_of(4) {
                self.camp.spend_power();
            }
            if self.survival.health == 0 {
                self.state = State::Dead;
                self.say("Life support failed. N: new expedition. F9: load saved expedition.");
                return true;
            }
            if !was_night && self.survival.night() {
                self.say("Nightfall: colder and more dangerous. Return to H!");
                redraw = true;
            }
        }
        redraw
    }
}
