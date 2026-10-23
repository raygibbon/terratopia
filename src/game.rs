use crate::{
    crafting,
    entities::player::Player,
    exploration::{Exploration, ScanReport},
    input::Action,
    inventory::{Inventory, Item},
    planet::Camp,
    survival::{Conditions, Environment, NIGHT_TICKS, Survival},
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
    Survey,
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
    pub wildlife: crate::wildlife::Wildlife,
    pub filter_ticks: u16,
    pub resting: bool,
    pub scan_contacts: Vec<String>,
    pub scan_active: bool,
    last_weather: crate::weather::Weather,
    return_state: State,
    clock: terragame_core::FixedStep,
}
impl Game {
    pub fn new(seed: u64) -> Self {
        Self::from_world(World::new(seed, 128))
    }
    pub fn legacy(seed: u64) -> Self {
        Self::from_world(World::legacy(seed, 128))
    }
    fn from_world(mut world: World) -> Self {
        let seed = world.seed();
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
            wildlife: crate::wildlife::Wildlife::default(),
            filter_ticks: 0,
            resting: false,
            scan_contacts: Vec::new(),
            scan_active: false,
            last_weather: crate::weather::Weather::default(),
            return_state: State::Playing,
            clock: terragame_core::FixedStep::new(STEP, 4).expect("valid simulation clock"),
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
    pub fn weather(&mut self) -> crate::weather::Weather {
        self.conditions_at(self.survival.ticks()).weather
    }
    fn conditions_at(&mut self, ticks: u64) -> Conditions {
        let biome = self
            .camp
            .tile(&mut self.world, self.player.x, self.player.y)
            .biome;
        Conditions {
            planet: self.world.profile(),
            weather: crate::weather::Weather::at(
                self.world.seed(),
                self.world.profile(),
                biome,
                self.player.x,
                self.player.y,
                ticks,
            ),
            filtered: self.filter_ticks > 0,
            crash_shield: self.camp.clearing(self.player.x, self.player.y),
            resting: self.resting,
        }
    }
    pub fn conditions(&mut self) -> Conditions {
        self.conditions_at(self.survival.ticks())
    }
    pub fn refresh_wildlife(&mut self) {
        self.wildlife
            .refresh(&mut self.world, &self.camp, Camp::point(self.player));
    }
    fn energy_check(&mut self, cost: u8) -> bool {
        if self.survival.energy < cost {
            self.resting = true;
            self.say("Exhausted. Z rests; energy packs restore 40. Resting recovers energy safely in A/H.");
            false
        } else {
            true
        }
    }
    pub fn survey_lines(&mut self) -> Vec<String> {
        let p = self.world.profile();
        let weather = self.weather();
        let mut lines = vec![
            format!("{} / {}", self.planet, p.archetype.name()),
            format!("Atmosphere: {} / oxygen {}%", p.atmosphere(), p.oxygen),
            format!(
                "Climate {}C, night drop {}C / gravity {}%",
                p.temperature, p.variation, p.gravity
            ),
            format!(
                "Radiation {} / toxicity {} / water {}",
                p.radiation, p.toxicity, p.water
            ),
            format!(
                "Vegetation {} / wildlife {} / mineral richness {}%",
                p.vegetation, p.wildlife, p.minerals
            ),
            format!(
                "Weather: {} (severity {})",
                weather.current.name(),
                weather.severity
            ),
            weather
                .warning
                .map_or("Forecast: no imminent transition".into(), |s| {
                    format!(
                        "WARNING: {} in {s}s. Return to H or crash shield!",
                        weather.next.name()
                    )
                }),
            format!(
                "Active wildlife {} / scanned {}",
                self.wildlife.active.len(),
                self.wildlife.scanned.len()
            ),
            "g grazer / d territorial defender / p predator / s scavenger".into(),
            format!(
                "Ruins salvaged {} / plants harvested {} / filter {}s",
                self.camp.looted_ruins.len(),
                self.camp.plants.len(),
                self.filter_ticks.div_ceil(4)
            ),
            format!("Goal: {}", self.objective()),
            "Visible wildlife:".into(),
        ];
        for c in self
            .wildlife
            .active
            .iter()
            .filter(|c| {
                self.exploration
                    .visibility(c.state.position.0, c.state.position.1)
                    == crate::exploration::Visibility::Visible
            })
            .take(8)
        {
            lines.push(format!(
                "{} {} / {} HP / {} at {},{}",
                c.species.behaviour.glyph(),
                c.species.name,
                c.state.health,
                if c.state.angry {
                    "HOSTILE"
                } else {
                    c.state.needs.activity.name()
                },
                c.state.position.0,
                c.state.position.1
            ));
            lines.push(format!(
                "  Hunger {} / energy {} / fear {}",
                c.state.needs.hunger, c.state.needs.energy, c.state.needs.fear
            ));
        }
        lines.push("Last scan contacts:".into());
        lines.extend(self.scan_contacts.iter().cloned());
        if !self.camp.looted_ruins.is_empty() {
            lines.push(format!(
                "Recovered record: {} / {}.",
                p.archetype.name(),
                p.atmosphere()
            ));
            lines.push(
                "Survey log: medicinal starleaf treats wounds; shelters survived the storms."
                    .into(),
            );
        }
        lines
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
            format!(
                "Crash ship A at {},{}; nearest station fuel: {}s",
                game.camp.ship().0,
                game.camp.ship().1,
                game.camp.power_remaining(Camp::point(game.player))
            ),
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
            "Ctrl+Q quits without saving if saving fails.".into(),
            "Backspace closes screens. Up/Down scroll help. No time passes in menus.".into(),
            "M opens planet survey: atmosphere, weather, species and scan contacts.".into(),
            "g grazers flee; d defenders guard territory; p predators chase nearby prey.".into(),
            "F attacks adjacent creatures (4 energy); E on x recovers organics.".into(),
            "Z rests (+5 energy/s, +8 in shelter). Resting does NOT pause danger.".into(),
            "Follow the east trail: + medicinal at 5, glow fungus at 7, ruin # at 12.".into(),
            "Harvest plants with E. Medical kits heal; energy packs restore; filters last 120s."
                .into(),
            "Storms warn before arrival. H shelters and the crash shield block exposure.".into(),
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
        if matches!(
            self.state,
            State::Inventory | State::Crafting | State::Help | State::Survey
        ) {
            match action {
                Action::Back | Action::Pause => self.state = self.return_state,
                Action::Inventory if self.state == State::Inventory => {
                    self.state = self.return_state
                }
                Action::Craft if self.state == State::Crafting => self.state = self.return_state,
                Action::Help if self.state == State::Help => self.state = self.return_state,
                Action::Menu if self.state == State::Survey => self.state = self.return_state,
                Action::Move(_, dy) => {
                    let count = match self.state {
                        State::Inventory => self.inventory.stacks().len(),
                        State::Crafting => crafting::RECIPES.len(),
                        State::Survey => self.survey_lines().len(),
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
                *self = if self.world.generator() == 2 {
                    Self::legacy(self.world.seed())
                } else {
                    Self::new(self.world.seed())
                };
                self.state = State::Playing;
                self.started = true;
                self.refresh_wildlife();
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
            (State::Playing, Action::Menu) => {
                self.return_state = State::Playing;
                self.selected = 0;
                self.state = State::Survey;
            }
            (State::Playing, Action::Rest) => {
                self.resting = !self.resting;
                self.say(if self.resting {
                    "Resting: +5 energy/s, +8 in shelter. The world still moves; Z cancels."
                } else {
                    "Rest ended."
                });
                return true;
            }
            (State::Playing, Action::Attack) => {
                if !self.energy_check(4) {
                    return true;
                }
                match self.wildlife.attack(Camp::point(self.player)) {
                    Ok(message) => {
                        self.survival.energy -= 4;
                        self.resting = false;
                        self.say(message);
                    }
                    Err(e) => self.say(e),
                };
                return true;
            }
            (State::Playing, Action::Move(dx, dy)) => {
                let cost = if self.camp.clearing(self.player.x, self.player.y) {
                    1
                } else {
                    self.world.profile().gravity.div_ceil(100) as u8
                };
                if !self.energy_check(cost) {
                    return true;
                }
                if self.player.step_if(dx, dy, |x, y| {
                    !self.wildlife.occupied((x, y))
                        && self.camp.tile(&mut self.world, x, y).biome.passable()
                }) {
                    self.world.focus(self.player.x, self.player.y);
                    self.survival.energy = self.survival.energy.saturating_sub(cost);
                    self.resting = false;
                    self.refresh_wildlife();
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
                    self.say("Route blocked: terrain or wildlife. F attacks adjacent creatures.");
                }
                return true;
            }
            (State::Playing, Action::Interact) => {
                self.interact();
                if self.survival.health == 0 {
                    self.state = State::Dead;
                    self.say("Life support failed. N: new expedition. F9: load saved expedition.");
                }
                return true;
            }
            (State::Playing, Action::Build) => {
                if self.wildlife.occupied(Camp::point(self.player)) {
                    self.say("Wildlife blocks construction");
                    return true;
                }
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
                self.scan_contacts.clear();
                for site in report.sites.iter().take(8) {
                    self.scan_contacts.push(format!(
                        "{} {} at {},{}",
                        site.kind.glyph(),
                        site.kind.name(),
                        site.x,
                        site.y
                    ));
                }
                for c in &self.wildlife.active {
                    if c.state.health > 0
                        && self
                            .exploration
                            .visibility(c.state.position.0, c.state.position.1)
                            == crate::exploration::Visibility::Visible
                    {
                        if self.wildlife.scanned.len() < 100_000 {
                            self.wildlife.scanned.insert(c.state.id);
                        }
                        self.scan_contacts.push(format!(
                            "{} {} ({}) at {},{}",
                            c.species.behaviour.glyph(),
                            c.species.name,
                            c.species.behaviour.name(),
                            c.state.position.0,
                            c.state.position.1
                        ));
                    }
                }
                let weather = self.weather();
                if weather.current.dangerous()
                    || weather.next.dangerous()
                    || self.world.profile().toxicity > 35
                    || self.world.profile().radiation > 35
                {
                    self.scan_contacts.push(format!(
                        "Anomaly: {} / radiation {} / toxicity {}",
                        weather.next.name(),
                        self.world.profile().radiation,
                        self.world.profile().toxicity
                    ));
                }
                self.last_scan = Some(report);
                return true;
            }
            (State::Playing, Action::Pause) => self.state = State::Paused,
            (State::Paused, Action::Pause | Action::Accept) => self.state = State::Playing,
            (State::Paused | State::NewWorld, Action::Menu) => self.state = State::MainMenu,
            _ => {}
        }
        if self.state != before {
            self.clock.reset();
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
            Item::MedicalKit | Item::EnergyPack | Item::HazardFilter => {
                if item == Item::MedicalKit && self.survival.health == 100
                    || item == Item::EnergyPack && self.survival.energy == 100
                    || item == Item::HazardFilter && self.filter_ticks > 480
                {
                    Err("Already replenished; supply preserved")
                } else {
                    self.inventory.remove(item, 1).map(|()| match item {
                        Item::MedicalKit => {
                            self.survival.health = self.survival.health.saturating_add(35).min(100)
                        }
                        Item::EnergyPack => {
                            self.survival.energy = self.survival.energy.saturating_add(40).min(100)
                        }
                        _ => self.filter_ticks += 480,
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
                } else if item == Item::OxygenCanister {
                    "Oxygen replenished (+50)."
                } else {
                    "Survival supply used: health, energy or hazard protection restored."
                }
            }
            Err(e) => e,
        });
    }
    fn interact(&mut self) {
        let p = Camp::point(self.player);
        if let Some(id) = self.wildlife.corpse(p) {
            match self.inventory.add(Item::Organics, 2) {
                Ok(()) => {
                    self.wildlife.loot_corpse(id);
                    self.say("Recovered organics x2 from wildlife.");
                }
                Err(e) => self.say(e),
            };
            return;
        }
        if matches!(
            self.camp.tile(&mut self.world, p.0, p.1).landmark,
            Some(crate::world::landmark::Landmark::Plant(_))
        ) {
            if !self.energy_check(3) {
                return;
            }
            match self
                .camp
                .harvest_plant(&mut self.world, &mut self.inventory, p)
            {
                Ok(plant) => {
                    self.survival.energy -= 3;
                    self.resting = false;
                    if plant == crate::world::flora::Flora::Toxic && self.filter_ticks == 0 {
                        self.survival.health = self.survival.health.saturating_sub(3);
                    }
                    self.say(format!(
                        "Harvested {}. C crafts medical kits, filters and energy packs.",
                        plant.name()
                    ));
                }
                Err(e) => self.say(e),
            }
            return;
        }
        if self.camp.tile(&mut self.world, p.0, p.1).landmark
            == Some(crate::world::landmark::Landmark::Ruin)
        {
            if !self.energy_check(2) {
                return;
            }
            match self.camp.loot_ruin(&mut self.world, &mut self.inventory, p) {
                Ok(record) => {
                    self.survival.energy -= 2;
                    self.resting = false;
                    self.say(record);
                }
                Err(e) => self.say(e),
            }
            return;
        }
        if crate::planet::near(p, self.camp.ship()) && !self.camp.cache_claimed {
            let result = self.camp.scavenge(&mut self.inventory);
            self.say(match result {
                Ok(()) => "Cache: salvage pick, 2 oxygen canisters, 1 battery. Gather i/k, then craft (C).",
                Err(e) => e,
            });
        } else if matches!(
            self.camp.tile(&mut self.world, p.0, p.1).landmark,
            Some(crate::world::landmark::Landmark::Deposit(_))
        ) {
            if !self.energy_check(2) {
                return;
            }
            match self.camp.harvest(&mut self.world, &mut self.inventory, p) {
                Ok((resource, n)) => {
                    self.survival.energy = self.survival.energy.saturating_sub(2);
                    self.resting = false;
                    self.say(format!(
                        "Harvested {} x{n}. Remaining {}.",
                        resource.name(),
                        self.camp.deposit_capacity(&self.world, p) - self.camp.harvested[&p]
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
        self.scan_active = scan;
        let radius = self.weather().radius(scan);
        self.exploration
            .reveal_radius(self.player.x, self.player.y, radius, |x, y| {
                self.camp.site(&mut self.world, x, y)
            })
    }
    /// Four fixed updates maximum; discard excess elapsed time after a stall.
    pub fn advance(&mut self, elapsed: Duration) -> bool {
        if self.state != State::Playing {
            return false;
        }
        let steps = self.clock.advance(elapsed);
        let mut redraw = false;
        for _ in 0..steps {
            let was_night = self.survival.night();
            let env = self.environment();
            let context = self.conditions_at(self.survival.ticks().saturating_add(1));
            redraw |= self.survival.tick_living(env, context);
            self.filter_ticks = self.filter_ticks.saturating_sub(1);
            if context.weather.current != self.last_weather.current {
                self.say(format!(
                    "Weather: {}. Shelter and crash shield protect against storms.",
                    context.weather.current.name()
                ));
                self.observe(self.scan_active);
                redraw = true;
            } else if context.weather.warning.is_some() && self.last_weather.warning.is_none() {
                self.say(format!(
                    "STORM WARNING: {} in {} seconds. Return to H or crash shield!",
                    context.weather.next.name(),
                    context.weather.warning.unwrap_or(0)
                ));
                redraw = true;
            }
            if context.weather.current != self.last_weather.current {
                self.wildlife
                    .notify(crate::wildlife::EcosystemEvent::WeatherChanged(
                        context.weather.current as u8,
                    ));
            }
            self.last_weather = context.weather;
            let (injury, events) = self.wildlife.tick(
                &mut self.world,
                &mut self.camp,
                Camp::point(self.player),
                self.survival.ticks(),
            );
            if injury > 0 {
                self.survival.health = self.survival.health.saturating_sub(injury);
                self.say(format!("Wildlife attack: -{injury} HP. Escape or F fight!"));
                redraw = true;
            }
            // Gameplay queue is bounded and separate from UI messages. Rules
            // commit at their source; consumers observe in stable FIFO order.
            self.wildlife.events.drain().for_each(drop);
            redraw |= self.survival.ticks().is_multiple_of(4);
            for event in events.into_iter().take(2) {
                self.say(event);
                redraw = true;
            }

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
