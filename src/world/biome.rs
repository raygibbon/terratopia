#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Biome {
    Grassland,
    Forest,
    Desert,
    Mountains,
    Water,
    Frozen,
    Wasteland,
}
impl Biome {
    pub fn classify(elevation: i64, temperature: i64, moisture: i64) -> Self {
        if elevation < 22000 {
            Self::Water
        } else if elevation > 47000 {
            Self::Mountains
        } else if temperature < 22000 {
            Self::Frozen
        } else if temperature > 42000 && moisture < 28000 {
            Self::Desert
        } else if moisture < 21000 {
            Self::Wasteland
        } else if moisture > 37000 {
            Self::Forest
        } else {
            Self::Grassland
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Grassland => "Grassland",
            Self::Forest => "Forest",
            Self::Desert => "Desert",
            Self::Mountains => "Mountains",
            Self::Water => "Water",
            Self::Frozen => "Frozen terrain",
            Self::Wasteland => "Alien wasteland",
        }
    }
    pub fn passable(self) -> bool {
        !matches!(self, Self::Water | Self::Mountains)
    }
}
