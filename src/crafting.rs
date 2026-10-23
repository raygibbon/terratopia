use crate::inventory::{Inventory, Item};
pub struct Recipe {
    pub output: Item,
    pub ingredients: &'static [(Item, u8)],
}
pub const RECIPES: [Recipe; 8] = [
    Recipe {
        output: Item::MiningTool,
        ingredients: &[(Item::Iron, 3), (Item::Carbon, 2)],
    },
    Recipe {
        output: Item::ShelterKit,
        ingredients: &[(Item::Iron, 4), (Item::Carbon, 4), (Item::Crystal, 2)],
    },
    Recipe {
        output: Item::OxygenCanister,
        ingredients: &[(Item::Carbon, 2), (Item::Crystal, 1)],
    },
    Recipe {
        output: Item::Battery,
        ingredients: &[(Item::Iron, 1), (Item::Crystal, 1)],
    },
    Recipe {
        output: Item::ThermalSuit,
        ingredients: &[(Item::Carbon, 3), (Item::Crystal, 1)],
    },
    Recipe {
        output: Item::MedicalKit,
        ingredients: &[(Item::Herbs, 2), (Item::Carbon, 1)],
    },
    Recipe {
        output: Item::HazardFilter,
        ingredients: &[(Item::Organics, 2), (Item::Crystal, 1)],
    },
    Recipe {
        output: Item::EnergyPack,
        ingredients: &[(Item::Organics, 2), (Item::Carbon, 1)],
    },
];
pub fn craft(inventory: &mut Inventory, index: usize) -> Result<Item, &'static str> {
    let recipe = RECIPES.get(index).ok_or("Unknown recipe")?;
    inventory.transact(recipe.ingredients, &[(recipe.output, 1)])?;
    Ok(recipe.output)
}
