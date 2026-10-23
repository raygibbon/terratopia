//! Bounded stacks and atomic transactions shared by scavenging and crafting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Iron,
    Carbon,
    Crystal,
    Copper,
    Silica,
    BasicPick,
    MiningTool,
    OxygenCanister,
    Battery,
    ShelterKit,
    ThermalSuit,
}
impl Item {
    pub const ALL: [Self; 11] = [
        Self::Iron,
        Self::Carbon,
        Self::Crystal,
        Self::Copper,
        Self::Silica,
        Self::BasicPick,
        Self::MiningTool,
        Self::OxygenCanister,
        Self::Battery,
        Self::ShelterKit,
        Self::ThermalSuit,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Iron => "Iron",
            Self::Carbon => "Carbon",
            Self::Crystal => "Crystal",
            Self::Copper => "Copper",
            Self::Silica => "Silica",
            Self::BasicPick => "Salvage pick",
            Self::MiningTool => "Mining tool",
            Self::OxygenCanister => "Oxygen canister",
            Self::Battery => "Battery",
            Self::ShelterKit => "Shelter kit",
            Self::ThermalSuit => "Thermal suit",
        }
    }
    pub fn limit(self) -> u8 {
        if matches!(
            self,
            Self::BasicPick | Self::MiningTool | Self::ThermalSuit | Self::ShelterKit
        ) {
            1
        } else {
            20
        }
    }
    pub fn id(self) -> usize {
        Self::ALL.iter().position(|&i| i == self).unwrap()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stack {
    pub item: Item,
    pub count: u8,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Inventory {
    stacks: Vec<Stack>,
}
pub const CAPACITY: usize = 10;
impl Inventory {
    pub fn stacks(&self) -> &[Stack] {
        &self.stacks
    }
    pub fn count(&self, item: Item) -> u16 {
        self.stacks
            .iter()
            .filter(|s| s.item == item)
            .map(|s| u16::from(s.count))
            .sum()
    }
    pub fn from_stacks(stacks: Vec<Stack>) -> Result<Self, &'static str> {
        if stacks.len() > CAPACITY
            || stacks
                .iter()
                .any(|s| s.count == 0 || s.count > s.item.limit())
        {
            return Err("Invalid inventory capacity or stack");
        }
        Ok(Self { stacks })
    }
    pub fn add(&mut self, item: Item, mut count: u8) -> Result<(), &'static str> {
        let free: u16 = self
            .stacks
            .iter()
            .filter(|s| s.item == item)
            .map(|s| u16::from(item.limit() - s.count))
            .sum::<u16>()
            + (CAPACITY - self.stacks.len()) as u16 * u16::from(item.limit());
        if free < u16::from(count) {
            return Err("Inventory full: use or discard a stack with I");
        }
        for stack in self.stacks.iter_mut().filter(|s| s.item == item) {
            let amount = count.min(item.limit() - stack.count);
            stack.count += amount;
            count -= amount;
        }
        while count > 0 {
            let amount = count.min(item.limit());
            self.stacks.push(Stack {
                item,
                count: amount,
            });
            count -= amount;
        }
        Ok(())
    }
    pub fn remove(&mut self, item: Item, mut count: u8) -> Result<(), &'static str> {
        if self.count(item) < u16::from(count) {
            return Err("Missing ingredients or supplies");
        }
        for stack in self.stacks.iter_mut().filter(|s| s.item == item) {
            let n = count.min(stack.count);
            stack.count -= n;
            count -= n;
        }
        self.stacks.retain(|s| s.count > 0);
        Ok(())
    }
    pub fn transact(
        &mut self,
        ingredients: &[(Item, u8)],
        outputs: &[(Item, u8)],
    ) -> Result<(), &'static str> {
        let mut next = self.clone();
        for &(item, n) in ingredients {
            next.remove(item, n)?;
        }
        for &(item, n) in outputs {
            next.add(item, n)?;
        }
        *self = next;
        Ok(())
    }
    pub fn discard(&mut self, index: usize) -> Option<Stack> {
        if index < self.stacks.len() {
            Some(self.stacks.remove(index))
        } else {
            None
        }
    }
}

impl From<crate::world::landmark::Resource> for Item {
    fn from(resource: crate::world::landmark::Resource) -> Self {
        use crate::world::landmark::Resource;
        match resource {
            Resource::Iron => Self::Iron,
            Resource::Carbon => Self::Carbon,
            Resource::Crystal => Self::Crystal,
            Resource::Copper => Self::Copper,
            Resource::Silica => Self::Silica,
        }
    }
}
