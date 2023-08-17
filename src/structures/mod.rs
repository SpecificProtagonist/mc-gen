use std::{collections::VecDeque, fs::File, path::Path, sync::Mutex};

use lazy_static::lazy_static;
use nbt::{decode::read_gzip_compound_tag, CompoundTag, CompoundTagError, Tag};

use crate::*;

// pub mod castle;
// pub mod dzong;
// pub mod farm;

#[derive(Clone)]
pub struct TemplateMark(IVec3, Option<HDir>, Vec<String>);

// Hand-build structure, stored via structure blocks
#[derive(Clone)]
pub struct Prefab {
    size: IVec3,
    blocks: VecDeque<(IVec3, Block)>,
    markers: HashMap<String, TemplateMark>,
}

// Cache the templates
lazy_static! {
    static ref PREFABS: Mutex<HashMap<String, &'static Prefab>> = Default::default();
}

impl Prefab {
    pub fn get(name: &str) -> &'static Self {
        // TODO: eagerly load all prefabs to init unknown blocks
        let mut prefabs = PREFABS.lock().unwrap();
        prefabs
            .entry(name.into())
            .or_insert_with(|| Box::leak(Box::new(Self::load(name))))
    }

    /// Panics when file is not found, isn't a valid structure or contains unknown blocks
    /// (since file is not specified by user)
    fn load(name: &str) -> Self {
        let mut path = Path::new("prefabs").join(name);
        path.set_extension("nbt");
        let mut file =
            File::open(&path).unwrap_or_else(|_| panic!("Structure file {:?} not found", path));
        let nbt =
            read_gzip_compound_tag(&mut file).unwrap_or_else(|_| panic!("Invalid nbt: {:?}", path));

        Self::load_from_nbt(&nbt, name)
            .unwrap_or_else(|err| panic!("Invalid structure {:?}: {:?}", path, err))
    }

    /// Can also panic, but eh, won't happen when the user is executing the program
    /// Oh, and of course CompountTagError holds a reference to the original tag
    /// so I can't just use anyhow (TODO: PR)
    fn load_from_nbt<'a>(
        nbt: &'a CompoundTag,
        name: &'a str,
    ) -> Result<Prefab, CompoundTagError<'a>> {
        #[allow(clippy::ptr_arg)]
        fn read_pos(nbt: &Vec<Tag>) -> IVec3 {
            match [&nbt[0], &nbt[1], &nbt[2]] {
                [Tag::Int(x), Tag::Int(z), Tag::Int(y)] => ivec3(*x, *y, *z),
                _ => panic!(),
            }
        }

        let size = read_pos(nbt.get("size")?);

        // Look for markers such as the origin
        let markers: HashMap<_, _> = nbt
            .get_compound_tag_vec("entities")?
            .iter()
            .filter_map(|nbt| {
                let pos = read_pos(nbt.get("blockPos").unwrap());
                let nbt = nbt.get_compound_tag("nbt").unwrap();
                if let Ok("minecraft:armor_stand") = nbt.get_str("id") {
                    let tags: Vec<String> = nbt
                        .get_str_vec("Tags")
                        .unwrap_or(Vec::new())
                        .iter()
                        .map(|tag| (*tag).to_owned())
                        .collect();
                    // For some reason, CustomName doesn't work anymore?
                    let name = tags
                        .iter()
                        .find(|tag| tag.starts_with("name:"))
                        .expect("Unnamed marker")
                        .strip_prefix("name:")
                        .unwrap()
                        .to_owned();

                    let dir = if tags.contains(&String::from("xpos")) {
                        Some(XPos)
                    } else if tags.contains(&String::from("xneg")) {
                        Some(XNeg)
                    } else if tags.contains(&String::from("zpos")) {
                        Some(YPos)
                    } else if tags.contains(&String::from("zneg")) {
                        Some(YNeg)
                    } else {
                        None
                    };
                    Some((name, TemplateMark(pos, dir, tags)))
                } else {
                    None
                }
            })
            .collect();

        let origin = markers
            .get("origin")
            .unwrap_or_else(|| panic!("Failed to load template {}: No origin set", name))
            .0;

        let palette: Vec<Block> = nbt
            .get_compound_tag_vec("palette")?
            .iter()
            .map(|nbt| Block::from_nbt(nbt))
            .collect();

        // for block in &palette {
        //     println!("{}", block.blockstate(&UNKNOWN_BLOCKS.read().unwrap()));
        // }

        let mut blocks = VecDeque::new();
        let mut air = VecDeque::new();

        for nbt in nbt.get_compound_tag_vec("blocks")?.into_iter().rev() {
            let pos = read_pos(nbt.get("pos")?);
            let block = palette[nbt.get_i32("state")? as usize];
            // TODO: nbt data
            if block == Air {
                // Clear out the area first (from top to bottom)
                air.push_front((pos - origin, Air));
            } else {
                // Then do the building (from bottom to top)
                blocks.push_back((pos - origin, block));
            }
        }
        blocks.extend(air);

        Ok(Self {
            size,
            blocks,
            markers,
        })
    }

    pub fn build(&self, level: &mut Level, pos: IVec3, facing: HDir, wood: TreeSpecies) {
        let rotation = facing as i32 + 4 - self.markers["origin"].1.unwrap() as i32;
        for (offset, block) in self.blocks.iter() {
            level[pos + offset.rotated(rotation)] = block.rotated(rotation).swap_wood_type(wood);
        }
    }

    pub fn build_clipped(&self, world: &mut Level, pos: IVec3, facing: HDir, area: Rect) {
        let rotation = facing as i32 + 4 - self.markers["origin"].1.unwrap() as i32;
        for (offset, block) in self.blocks.iter() {
            let pos = pos + offset.rotated(rotation);
            if area.contains(pos.truncate()) {
                world[pos] = block.rotated(rotation);
            }
        }
    }

    // TODO: palette swap
}
