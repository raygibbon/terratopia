#[derive(Debug)]
pub struct Survival {
    pub health: u8,
    pub oxygen: u8,
    pub energy: u8,
    ticks: u64,
}
impl Default for Survival {
    fn default() -> Self {
        Self {
            health: 100,
            oxygen: 100,
            energy: 100,
            ticks: 0,
        }
    }
}
impl Survival {
    pub fn day(&self) -> u64 {
        self.ticks / 480 + 1
    }
    pub fn tick(&mut self) -> bool {
        let before = self.day();
        self.ticks = self.ticks.saturating_add(1);
        self.day() != before
    }
}
