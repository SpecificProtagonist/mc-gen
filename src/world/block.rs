use std::{borrow::Cow, fmt::Display, sync::Arc};

pub use self::GroundPlant::*;
use crate::geometry::*;
use nbt::{CompoundTag, CompoundTagError};
use num_derive::FromPrimitive;

pub use Block::*;
pub use Color::*;
pub use Material::*;
pub use TreeSpecies::*;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum Block {
    Air,
    FullBlock(Material),
    Slab(Material, Flipped),
    Stair(Material, HDir, Flipped),
    Planks(TreeSpecies),
    Fence(Material),
    Water,
    Lava,
    Soil(Soil),
    Log(TreeSpecies, LogType),
    Leaves(TreeSpecies),
    GroundPlant(GroundPlant),
    Wool(Color),
    Terracotta(Option<Color>),
    SmoothQuartz,
    SnowLayer,
    Glowstone,
    GlassPane(Option<Color>),
    WallBanner(HDir, Color),
    Hay,
    Cauldron { water: u8 },
    Bell(HDir, BellAttachment),
    Repeater(HDir, u8),
    Barrier,
    Bedrock,
    CommandBlock(Arc<String>),
    Other(Arc<Blockstate>),
}

impl Default for Block {
    fn default() -> Self {
        Air
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum LogType {
    Normal(Axis),
    FullBark,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, FromPrimitive)]
#[repr(u8)]
pub enum TreeSpecies {
    Oak,
    Spruce,
    Birch,
    Jungle,
    Acacia,
    DarkOak,
    Warped,
    Crimson,
}

impl TreeSpecies {
    pub fn to_str(self) -> &'static str {
        match self {
            Oak => "oak",
            Spruce => "spruce",
            Birch => "birch",
            Jungle => "jungle",
            Acacia => "acacia",
            DarkOak => "dark_oak",
            Warped => "warped",
            Crimson => "crimson",
        }
    }
}

impl Display for TreeSpecies {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum Soil {
    Dirt,
    Grass,
    Sand,
    Gravel,
    Farmland,
    Path,
    Podzol,
    CoarseDirt,
    SoulSand,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum GroundPlant {
    Sapling(TreeSpecies),
    Cactus,
    Reeds,
    Pumpkin,
    Small(SmallPlant),
    Tall(TallPlant, Flipped),
    Crop(Crop),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum SmallPlant {
    Grass,
    DeadBush,
    Fern,
    BrownMushroom,
    RedMushroom,
    Dandelion,
    Poppy,
    BlueOrchid,
    Allium,
    AzureBluet,
    RedTulip,
    OrangeTulip,
    WhiteTulip,
    PinkTulip,
    OxeyeDaisy,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum TallPlant {
    Grass,
    Fern,
    Sunflower,
    Lilac,
    Rose,
    Peony,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum Crop {
    Wheat,
    Carrot,
    Potato,
    Beetroot,
}

// Note: for dyes, id order is reversed
#[derive(Debug, Copy, Clone, Eq, PartialEq, FromPrimitive, Hash)]
#[repr(u8)]
pub enum Color {
    White,
    Orange,
    Magenta,
    LightBlue,
    Yellow,
    Lime,
    Pink,
    Gray,
    LightGray,
    Cyan,
    Purple,
    Blue,
    Brown,
    Green,
    Red,
    Black,
}

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                White => "white",
                Orange => "orange",
                Magenta => "magenta",
                LightBlue => "light_blue",
                Yellow => "yellow",
                Lime => "lime",
                Pink => "pink",
                Gray => "gray",
                LightGray => "light_gray",
                Cyan => "cyan",
                Purple => "purple",
                Blue => "blue",
                Brown => "brown",
                Green => "green",
                Red => "red",
                Black => "black",
            }
        )
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Flipped(pub bool);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Material {
    Stone,
    Granite,
    PolishedGranite,
    Diorite,
    PolishedDiorite,
    Andesite,
    PolishedAndesite,
    Wood(TreeSpecies),
    Cobble,
    MossyCobble,
    Stonebrick,
    MossyStonebrick,
    Brick,
    Sandstone,
    SmoothSandstone,
    RedSandstone,
    SmoothRedSandstone,
    Blackstone,
    PolishedBlackstone,
    PolishedBlackstoneBrick,
}

impl Display for Material {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

impl Material {
    pub fn to_str(self) -> &'static str {
        match self {
            Stone => "stone",
            Diorite => "diorite",
            PolishedDiorite => "polished_diorite",
            Granite => "granite",
            PolishedGranite => "polished_granite",
            Andesite => "andesite",
            PolishedAndesite => "polished_andesite",
            Wood(species) => species.to_str(),
            Cobble => "cobblestone",
            MossyCobble => "mossy_cobblestone",
            Brick => "brick",
            Stonebrick => "stone_brick",
            MossyStonebrick => "mossy_stone_brick",
            Sandstone => "sandstone",
            SmoothSandstone => "smooth_sandstone",
            RedSandstone => "red_sandstone",
            SmoothRedSandstone => "smooth_red_sandstone",
            Blackstone => "blackstone",
            PolishedBlackstone => "polished_blackstone",
            PolishedBlackstoneBrick => "polished_blackstone_brick",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum BellAttachment {
    Floor,
    Ceiling,
    SingleWall,
    DoubleWall,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Blockstate(
    pub Cow<'static, str>,
    pub Vec<(Cow<'static, str>, Cow<'static, str>)>,
);

impl Display for Blockstate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)?;
        if self.1.len() > 0 {
            write!(f, "[")?;
            for (i, (name, state)) in self.1.iter().enumerate() {
                write!(f, "{}={}", name, state)?;
                if i + 1 < self.1.len() {
                    write!(f, ",")?;
                }
            }
            write!(f, "]")?;
        }
        Ok(())
    }
}

impl Block {
    // TODO: blockstates for fences need context... ugh
    pub fn blockstate(&self) -> Blockstate {
        impl<Name: Into<Cow<'static, str>>> From<Name> for Blockstate {
            fn from(name: Name) -> Self {
                Self(name.into(), vec![])
            }
        }

        match self {
            Air => "air".into(),
            FullBlock(material) => match material {
                Brick => "bricks".into(),
                Stonebrick => "stone_bricks".into(),
                PolishedBlackstoneBrick => "polished_blackstone_bricks".into(),
                material => material.to_str().into(),
            },
            Planks(species) => format!("{}_planks", species).into(),
            Soil(soil_type) => match soil_type {
                Soil::Grass => "grass_block".into(),
                Soil::Dirt => "dirt".into(),
                Soil::Sand => "sand".into(),
                Soil::Gravel => "gravel".into(),
                Soil::Farmland => "farmland".into(),
                Soil::Path => "grass_path".into(),
                Soil::CoarseDirt => "coarse_dirt".into(),
                Soil::Podzol => "podzol".into(),
                Soil::SoulSand => "soul_sand".into(),
            },
            Bedrock => "bedrock".into(),
            // TODO: water level
            Water => "water".into(),
            Lava => "lava".into(),
            Log(species, log_type) => match log_type {
                LogType::Normal(axis) => Blockstate(
                    match species {
                        Warped | Crimson => format!("{}_stem", species),
                        _ => format!("{}_log", species),
                    }
                    .into(),
                    vec![("axis".into(), axis.to_str().into())],
                ),
                LogType::FullBark => Blockstate(
                    match species {
                        Warped | Crimson => format!("{}_hyphae", species),
                        _ => format!("{}_wood", species),
                    }
                    .into(),
                    vec![],
                ),
            },
            Leaves(species) => Blockstate(
                format!("{}_leaves", species).into(),
                vec![("persistent".into(), "true".into())],
            ),
            GroundPlant(plant) => match plant {
                GroundPlant::Sapling(species) => format!("{}_sapling", species).into(),
                GroundPlant::Small(plant) => match plant {
                    SmallPlant::Grass => "grass".into(),
                    SmallPlant::Fern => "fern".into(),
                    SmallPlant::DeadBush => "dead_bush".into(),
                    SmallPlant::Dandelion => "dandelion".into(),
                    SmallPlant::Poppy => "poppy".into(),
                    SmallPlant::BlueOrchid => "blue_orchid".into(),
                    SmallPlant::Allium => "allium".into(),
                    SmallPlant::AzureBluet => "azure_bluet".into(),
                    SmallPlant::RedTulip => "red_tulip".into(),
                    SmallPlant::OrangeTulip => "orange_tulip".into(),
                    SmallPlant::WhiteTulip => "white_tulip".into(),
                    SmallPlant::PinkTulip => "pink_tulip".into(),
                    SmallPlant::OxeyeDaisy => "oxeye_daisy".into(),
                    SmallPlant::BrownMushroom => "brown_mushroom".into(),
                    SmallPlant::RedMushroom => "red_mushroom".into(),
                },
                GroundPlant::Cactus => "cactus".into(),
                GroundPlant::Reeds => "sugar_cane".into(),
                GroundPlant::Pumpkin => "pumpkin".into(),
                GroundPlant::Tall(plant, Flipped(upper)) => Blockstate(
                    match plant {
                        TallPlant::Sunflower => "sunflower".into(),
                        TallPlant::Lilac => "lilac".into(),
                        TallPlant::Grass => "tall_grass".into(),
                        TallPlant::Fern => "large_fern".into(),
                        TallPlant::Rose => "rose_bush".into(),
                        TallPlant::Peony => "peony".into(),
                    },
                    vec![("half".into(), if *upper { "upper" } else { "lower" }.into())],
                ),
                GroundPlant::Crop(crop) => match crop {
                    Crop::Wheat => Blockstate("wheat".into(), vec![("age".into(), "7".into())]),
                    Crop::Carrot => Blockstate("carrot".into(), vec![("age".into(), "7".into())]),
                    Crop::Potato => Blockstate("potato".into(), vec![("age".into(), "7".into())]),
                    Crop::Beetroot => {
                        Blockstate("beetroot".into(), vec![("age".into(), "3".into())])
                    }
                },
            },
            Fence(material) => match material {
                Wood(species) => format!("{}_fence", species).into(),
                material => format!("{}_wall", material).into(),
            },
            Wool(color) => format!("{}_wool", color).into(),
            Terracotta(Some(color)) => format!("{}_terracotta", color).into(),
            Terracotta(None) => "terracotta".into(),
            SmoothQuartz => "smooth_quartz".into(),
            SnowLayer => Blockstate("snow".into(), vec![("layers".into(), "1".into())]),
            Glowstone => "glowstone".into(),
            GlassPane(color) => {
                if let Some(color) = color {
                    format!("{}_stained_glass_pane", color).into()
                } else {
                    "glass_pane".into()
                }
            }
            WallBanner(facing, color) => Blockstate(
                format!("{}_wall_banner", color).into(),
                vec![("facing".into(), facing.to_str().into())],
            ),
            Hay => "hay_block".into(),
            Slab(material, Flipped(flipped)) => Blockstate(
                format!("{}_slab", material).into(),
                vec![(
                    "type".into(),
                    if *flipped { "top" } else { "bottom" }.into(),
                )],
            ),
            Stair(material, dir, Flipped(flipped)) => Blockstate(
                format!("{}_stairs", material).into(),
                vec![
                    (
                        "half".into(),
                        if *flipped { "top" } else { "bottom" }.into(),
                    ),
                    ("facing".into(), dir.to_str().into()),
                ],
            ),
            Cauldron { water } => Blockstate(
                "cauldron".into(),
                vec![(
                    "level".into(),
                    match water {
                        0 => "0".into(),
                        1 => "1".into(),
                        2 => "2".into(),
                        3 => "3".into(),
                        _ => panic!("Cauldron water level {}", water),
                    },
                )],
            ),
            Bell(facing, attachment) => Blockstate(
                "bell".into(),
                vec![
                    ("facing".into(), facing.to_str().into()),
                    (
                        "attachment".into(),
                        match attachment {
                            BellAttachment::Floor => "floor",
                            BellAttachment::Ceiling => "ceiling",
                            BellAttachment::DoubleWall => "double_wall",
                            BellAttachment::SingleWall => "single_wall",
                        }
                        .into(),
                    ),
                ],
            ),
            Repeater(dir, delay) => Blockstate(
                "repeater".into(),
                vec![
                    (
                        "delay".into(),
                        match delay {
                            1 => "1".into(),
                            2 => "2".into(),
                            3 => "3".into(),
                            4 => "4".into(),
                            _ => panic!("Repeater delay {}", delay),
                        },
                    ),
                    ("facing".into(), dir.to_str().into()),
                ],
            ),
            Barrier => "barrier".into(),
            CommandBlock(_) => "command_block".into(),
            Other(blockstate) => (**blockstate).clone(), // Unneccesary clone?
        }
    }

    pub fn tile_entity_nbt(&self, pos: Pos) -> Option<CompoundTag> {
        match self {
            Bell(..) => {
                let mut nbt = CompoundTag::new();
                nbt.insert_str("id", "bell");
                Some(nbt)
            }
            WallBanner(..) => {
                let mut nbt = CompoundTag::new();
                nbt.insert_str("id", "banner");
                Some(nbt)
            }
            CommandBlock(command) => {
                let mut nbt = CompoundTag::new();
                nbt.insert_str("id", "command_block");
                nbt.insert_str("Command", &command);
                nbt.insert_bool("TrackOutput", false);
                Some(nbt)
            }
            _ => None,
        }
        .map(|mut nbt| {
            nbt.insert_i32("x", pos.0);
            nbt.insert_i32("y", pos.1 as i32);
            nbt.insert_i32("z", pos.2);
            nbt
        })
    }

    /// This is for loading of the structure block format and very much incomplete
    /// (and panics on invalid blocks)
    pub fn from_nbt(nbt: &CompoundTag) -> Block {
        let name = nbt.get_str("Name").expect("Invalid block: no name");
        let name = name.strip_prefix("minecraft:").unwrap_or(name);
        let default_props = CompoundTag::new();
        let props = nbt.get_compound_tag("Properties").unwrap_or(&default_props);

        fn slab(material: Material, props: &CompoundTag) -> Block {
            match props.get_str("type").unwrap() {
                "top" => Slab(material, Flipped(true)),
                "double" => FullBlock(material),
                _ => Slab(material, Flipped(false)),
            }
        }

        fn stair(material: Material, props: &CompoundTag) -> Block {
            Stair(
                material,
                HDir::from_str(props.get_str("facing").unwrap()).unwrap(),
                Flipped(props.get_str("half").unwrap() == "top"),
            )
        }

        fn log(species: TreeSpecies, props: &CompoundTag) -> Block {
            Log(
                species,
                match props.get_str("axis").unwrap() {
                    "x" => LogType::Normal(Axis::X),
                    "y" => LogType::Normal(Axis::Y),
                    "z" => LogType::Normal(Axis::Z),
                    "none" => LogType::FullBark,
                    unknown => panic!("Invalid log axis {}", unknown),
                },
            )
        }

        fn wall_banner(color: Color, props: &CompoundTag) -> Block {
            WallBanner(
                HDir::from_str(props.get_str("facing").unwrap()).unwrap(),
                color,
            )
        }

        fn known_block<'a>(
            name: &str,
            props: &'a CompoundTag,
        ) -> Result<Block, CompoundTagError<'a>> {
            // TODO: expand this
            Ok(match name {
                "air" | "cave_air" => Air,
                "stone" => FullBlock(Stone),
                "granite" => FullBlock(Granite),
                "diorite" => FullBlock(Diorite),
                "andesite" => FullBlock(Andesite),
                "cobblestone" => FullBlock(Cobble),
                "bricks" => FullBlock(Brick),
                "stone_bricks" => FullBlock(Stonebrick),
                "bedrock" => Bedrock,
                "gravel" => Soil(Soil::Gravel),
                "grass_block" => Soil(Soil::Grass),
                "sand" => Soil(Soil::Sand),
                "dirt" if matches!(props.get_str("variant"), Err(_)) => Soil(Soil::Dirt),
                "dirt" if matches!(props.get_str("variant")?, "coarse_dirt") => {
                    Soil(Soil::CoarseDirt)
                }
                "oak_log" => log(Oak, props),
                "spruce_log" => log(Spruce, props),
                "birch_log" => log(Birch, props),
                "jungle_log" => log(Jungle, props),
                "acacia_log" => log(Acacia, props),
                "dark_oak_log" => log(DarkOak, props),
                "oak_leaves" => Leaves(Oak),
                "spruce_leaves" => Leaves(Spruce),
                "birch_leaves" => Leaves(Birch),
                "jungle_leaves" => Leaves(Jungle),
                "acacie_leaves" => Leaves(Acacia),
                "dark_oak_leaves" => Leaves(DarkOak),
                "grass" => GroundPlant(GroundPlant::Small(SmallPlant::Grass)),
                "fence" => Fence(Wood(Oak)),
                "cobblestone_wall" => Fence(MossyCobble),
                "mossy_cobblestone_wall" => Fence(MossyCobble),
                "oak_slab" => slab(Wood(Oak), props),
                "spruce_slab" => slab(Wood(Spruce), props),
                "birch_slab" => slab(Wood(Birch), props),
                "jungle_slab" => slab(Wood(Jungle), props),
                "acacia_slab" => slab(Wood(Acacia), props),
                "dark_oak_slab" => slab(Wood(DarkOak), props),
                "cobblestone_slab" => slab(Cobble, props),
                "mossy_cobblestone_slab" => slab(MossyCobble, props),
                "stone_brick_slab" => slab(Stonebrick, props),
                "mossy_stone_brick_slab" => slab(MossyStonebrick, props),
                "blackstone_slab" => slab(Blackstone, props),
                "polished_blackstone_slab" => slab(PolishedBlackstone, props),
                "oak_stairs" => stair(Wood(Oak), props),
                "spruce_stairs" => stair(Wood(Spruce), props),
                "birch_stairs" => stair(Wood(Birch), props),
                "jungle_stairs" => stair(Wood(Jungle), props),
                "acacia_stairs" => stair(Wood(Acacia), props),
                "dark_oak_stairs" => stair(Wood(DarkOak), props),
                "stone_brick_stairs" => stair(Stonebrick, props),
                "blackstone_stairs" => stair(Blackstone, props),
                "terracotta" => Terracotta(None),
                "white_terracotta" => Terracotta(Some(White)),
                "orange_terracotta" => Terracotta(Some(Orange)),
                "magenta_terracotta" => Terracotta(Some(Magenta)),
                "light_blue_terracotta" => Terracotta(Some(LightBlue)),
                "yellow_terracotta" => Terracotta(Some(Yellow)),
                "lime_terracotta" => Terracotta(Some(Lime)),
                "pink_terracotta" => Terracotta(Some(Pink)),
                "gray_terracotta" => Terracotta(Some(Gray)),
                "light_gray_terracotta" => Terracotta(Some(LightGray)),
                "cyan_terracotta" => Terracotta(Some(Cyan)),
                "purple_terracotta" => Terracotta(Some(Purple)),
                "blue_terracotta" => Terracotta(Some(Blue)),
                "brown_terracotta" => Terracotta(Some(Brown)),
                "green_terracotta" => Terracotta(Some(Green)),
                "red_terracotta" => Terracotta(Some(Red)),
                "black_terracotta" => Terracotta(Some(Black)),
                "cauldron" => Cauldron {
                    water: props.get_str("level")?.parse().unwrap(),
                },
                "bell" => Bell(
                    HDir::from_str(props.get_str("facing").unwrap()).unwrap(),
                    match props.get_str("attachment").unwrap() {
                        "floor" => BellAttachment::Floor,
                        "ceiling" => BellAttachment::Ceiling,
                        "single_wall" => BellAttachment::SingleWall,
                        _ => BellAttachment::DoubleWall,
                    },
                ),
                "red_wall_banner" => wall_banner(Red, props),
                "white_wall_banner" => wall_banner(Red, props),
                "blue_wall_banner" => wall_banner(Red, props),
                "green_wall_banner" => wall_banner(Red, props),
                "yellow_wall_banner" => wall_banner(Red, props),
                // This is quite hacky, maybe just use anyhow?
                _ => Err(CompoundTagError::TagNotFound {
                    name: "this is an unknown block",
                })?,
            })
        }

        known_block(name, props).unwrap_or_else(|_| {
            Other(Arc::new(Blockstate(
                name.to_owned().into(),
                if let Ok(props) = nbt.get_compound_tag("Properties") {
                    props
                        .iter()
                        .map(|(name, value)| {
                            (
                                name.clone().into(),
                                if let nbt::Tag::String(value) = value {
                                    value.clone().into()
                                } else {
                                    panic!("Non-string blockstate value")
                                },
                            )
                        })
                        .collect()
                } else {
                    Vec::new()
                },
            )))
        })
    }

    pub fn to_nbt(&self) -> CompoundTag {
        let blockstate = self.blockstate();
        let mut nbt = CompoundTag::new();
        nbt.insert("Name", blockstate.0.into_owned());
        if blockstate.1.len() > 0 {
            nbt.insert("Properties", {
                let mut props = CompoundTag::new();
                for (prop, value) in blockstate.1 {
                    props.insert_str(prop, value);
                }
                props
            });
        }
        nbt
    }

    pub fn solid(&self) -> bool {
        // Todo: expand this
        !matches!(
            self,
            Air | Water | Lava | GroundPlant(..) | Leaves(..) | SnowLayer
        )
    }

    pub fn rotated(&self, turns: u8) -> Self {
        match self {
            Log(species, LogType::Normal(Axis::X)) => Log(*species, LogType::Normal(Axis::Z)),
            Log(species, LogType::Normal(Axis::Z)) => Log(*species, LogType::Normal(Axis::X)),
            Stair(material, facing, flipped) => Stair(*material, facing.rotated(turns), *flipped),
            WallBanner(facing, color) => WallBanner(facing.rotated(turns), *color),
            Repeater(dir, delay) => Repeater(dir.rotated(turns), *delay),
            _ => self.clone(),
        }
    }
}
