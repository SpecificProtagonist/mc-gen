mod block;
mod entity;
mod biome;
mod export_behavior;

use std::{ops::{Index, IndexMut}, num::NonZeroU8, path::PathBuf};
use std::collections::HashMap;
use anvil_region::*;
use nbt::CompoundTag;
use rayon::prelude::*;
use itertools::Itertools;

use crate::geometry::*;
pub use block::*;
pub use biome::*;
pub use entity::*;


const MAX_VERSION: i32 = 1343;


// Maybe have a subworld not split into chunks for efficiency?
pub struct World {
    path: PathBuf,
    chunks: HashMap<ChunkIndex, Chunk>,
    area: Rect
} 

impl World {
    pub fn new(path: &str, area: Rect) -> Self {
        let mut chunks = HashMap::new();

        let region_path = {
            let mut region_path = PathBuf::from(path);
            region_path.push("region");
            region_path.into_os_string().into_string().unwrap()
        };
        let chunk_provider = AnvilChunkProvider::new(&region_path);
        let chunk_min: ChunkIndex = (area.min - Vec2(crate::LOAD_MARGIN, crate::LOAD_MARGIN)).into();
        let chunk_max: ChunkIndex = (area.max + Vec2(crate::LOAD_MARGIN, crate::LOAD_MARGIN)).into();
        
        let indices: Vec<_> = (chunk_min.0..=chunk_max.0).cartesian_product(chunk_min.1..=chunk_max.1).collect();
        chunks.par_extend(indices.par_iter().map(
            |index| ((*index).into(), Chunk::load(&chunk_provider, (*index).into())
            .expect(&format!("Failed to load chunk ({},{}): ", index.0, index.1)))
        ));

        let mut world = World { path: PathBuf::from(path), chunks, area };
        // Global scoreboard keeper
        world.insert_entity(Entity { 
            pos: Pos((area.min.0+area.max.0)/2,0,(area.min.1+area.max.1)/2), 
            data: Marker,
            id: 0
        });

        world
    }

    pub fn save(&self) -> Result<(), ChunkSaveError> {
        // Write chunks
        {
            let mut region_path = self.path.clone();
            region_path.push("region");
            // Internally, AnvilChunkProvider stores a path. So why require a str??
            let region_path = region_path.into_os_string().into_string().unwrap();
            let chunk_provider = AnvilChunkProvider::new(&region_path);
            for chunk in self.chunks.values() {
                chunk.save(&chunk_provider)?;
            }
        }

        export_behavior::save_behavior(&self).expect("Failed to write mcfunctions");

        // Edit metadata
        {
            let level_nbt_path = self.path.clone().into_os_string().into_string().unwrap() + "/level.dat";
            let mut file = std::fs::File::open(&level_nbt_path).expect("Failed to open level.dat");
            let mut nbt = nbt::decode::read_gzip_compound_tag(&mut file).expect("Failed to open level.dat");
            let data: &mut CompoundTag = nbt.get_mut("Data").expect("Corrupt level.dat");

            let name: &mut String = data.get_mut("LevelName").expect("Corrupt level.dat");
            name.push_str(" [generated]");

            let timestamp: &mut i64 = data.get_mut("LastPlayed").unwrap();
            *timestamp += 10;

            data.insert_i8("Difficulty", 0);

            let gamerules: &mut CompoundTag = data.get_mut("GameRules").unwrap();
            gamerules.insert_str("commandBlockOutput", "false");
            gamerules.insert_str("gameLoopFunction", "mc-gen:loop");

            let mut file = std::fs::OpenOptions::new().write(true).open(&level_nbt_path)
                .expect("Failed to open level.dat");
            nbt::encode::write_gzip_compound_tag(&mut file, nbt).expect("Failed to write level.dat");
        }
        Ok(())
    }

    pub fn set_if_not_solid(&mut self, pos: Pos, block: Block) {
        let block_ref = &mut self[pos];
        if !block_ref.solid() {
            *block_ref = block;
        }
    }

    pub fn biome(&self, column: Column) -> Biome{
        self.chunks.get(&column.into()).expect("Tried to read biome outside of loaded chunks").biomes[
            (column.0.rem_euclid(16)
           + column.1.rem_euclid(16) * 16) as usize
        ]
    }

    // load_area must have been called before
    // also, since we're going to be working with a fixed area, 
    // this could be moved to the generation part (maybe make readonly and rename to heightmap_orig)
    pub fn heightmap(&self, column: Column) -> u8 {
        self.chunks.get(&column.into()).expect("Tried to read heightmap outside of loaded chunks").heightmap[
            (column.0.rem_euclid(16)
           + column.1.rem_euclid(16) * 16) as usize
        ]
    }

    pub fn heightmap_mut(&mut self, column: Column) -> &mut u8 {
        &mut self.chunks.get_mut(&column.into()).expect("Tried to access heightmap outside of loaded chunks").heightmap[
            (column.0.rem_euclid(16)
           + column.1.rem_euclid(16) * 16) as usize
        ]
    }

    pub fn watermap(&self, column: Column) -> Option<NonZeroU8> {
        self.chunks.get(&column.into()).expect("Tried to read watermap outside of loaded chunks").watermap[
            (column.0.rem_euclid(16)
           + column.1.rem_euclid(16) * 16) as usize
        ]
    }

    pub fn watermap_mut(&mut self, column: Column) -> &mut Option<NonZeroU8> {
        &mut self.chunks.get_mut(&column.into()).expect("Tried to access watermap outside of loaded chunks").watermap[
            (column.0.rem_euclid(16)
           + column.1.rem_euclid(16) * 16) as usize
        ]
    }

    pub fn area(&self) -> Rect {
        self.area
    }

    pub fn insert_entity(&mut self, entity: Entity) {
        &mut self.chunks.get_mut(&entity.pos.into()).unwrap()
            .entities.push(entity);
    }
}

// load_area must have been called before
// todo: remove this requirement
impl Index<Pos> for World {
    type Output = Block;
    fn index(&self, pos: Pos) -> &Self::Output {
        let chunk = self.chunks.get(&pos.into()).expect("Tried to read block outside of loaded chunks");
        if let Some(section) = &chunk.sections[pos.1 as usize / 16] {
            &section.blocks[
                pos.0.rem_euclid(16) as usize
              + pos.1.rem_euclid(16) as usize * 16 * 16
              + pos.2.rem_euclid(16) as usize * 16
            ]
        } else {
            &Block::Air
        }
    }
}

impl IndexMut<Pos> for World {
    fn index_mut(&mut self, pos: Pos) -> &mut Self::Output {
        let chunk = self.chunks.get_mut(&pos.into()).expect("Tried to access block outside of loaded chunks");
        let section = chunk.sections[pos.1 as usize / 16].get_or_insert_with(||
            Box::new(Section::default())
        );
        &mut section.blocks[
            pos.0.rem_euclid(16) as usize
            + pos.1.rem_euclid(16) as usize * 16 * 16
            + pos.2.rem_euclid(16) as usize * 16
        ]
    }
}

pub struct Chunk {
    index: ChunkIndex,
    sections: [Option<Box<Section>>; 16],
    biomes: [Biome; 16 * 16],
    heightmap: [u8; 16 * 16],
    watermap: [Option<NonZeroU8>; 16 * 16], 
    entities: Vec<Entity>
    // Todo: TileEntities
}

impl Chunk {
    fn load(chunk_provider: &AnvilChunkProvider, index: ChunkIndex) -> Result<Self, ChunkLoadError> {
        let nbt = chunk_provider.load_chunk(index.0, index.1)?;
        let version = nbt.get_i32("DataVersion").unwrap();
        if version > MAX_VERSION {
            // Todo: 1.13+ support (palette)
            println!("Unsupported version: {}. Only 1.12 is supported currently.", version);
        }

        let level_nbt = nbt.get_compound_tag("Level").unwrap();

        let mut biomes = [Biome::default(); 16 * 16];
        let biome_ids = level_nbt.get_i8_vec("Biomes").unwrap();
        for i in 0..(16*16) {
            biomes[i] = Biome::from_bytes(biome_ids[i] as u8);
        }


        let mut sections: [Option<Box<Section>>; 16] = Default::default();
        let sections_nbt = level_nbt.get_compound_tag_vec("Sections").unwrap();
        
        for section_nbt in sections_nbt {
            let index = section_nbt.get_i8("Y").unwrap();
            sections[index as usize] = Some(Box::new(Default::default()));
            let section = sections[index as usize].as_mut().unwrap();
            // Ignore Add tag (not neccessary for vanilla)
            let block_ids = section_nbt.get_i8_vec("Blocks").unwrap();
            let block_data = section_nbt.get_i8_vec("Data").unwrap();
            for i in 0..(16*16*16) {
                section.blocks[i] = Block::from_bytes(
                    block_ids[i] as u8, 
                    {
                        let byte = block_data[i/2] as u8;
                        if i%2 == 0 { byte % 16 } else { byte >> 4 }
                    }
                )
            }
        }

        let mut heightmap = [0; 16 * 16];
        let mut watermap = [None; 16 * 16];
        for x in 0..16 {
            for z in 0..16 {
                'column: for section_index in (0..16).rev() {
                    if let Some(section) = &sections[section_index] {
                        for y in (0..16).rev() {
                            let block = section.blocks[x + z*16 + y*16*16];
                            let height = (section_index * 16 + y) as u8;
                            if match block {Block::Log(..) => false, _ => block.solid() } {
                                heightmap[x + z*16] = height;
                                break 'column;
                            } else if match block { Block::Water => height > 0, _ => false } {
                                watermap[x + z*16].get_or_insert(
                                    unsafe { NonZeroU8::new_unchecked((section_index * 16 + y) as u8) }
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(Chunk {
            index,
            sections,
            biomes,
            heightmap,
            watermap,
            entities: Vec::new()
        })
    }

    fn save(&self, chunk_provider: &AnvilChunkProvider) -> Result<(), ChunkSaveError> {
        chunk_provider.save_chunk(self.index.0, self.index.1, {
            let mut nbt = CompoundTag::new();
            nbt.insert_i32("DataVersion", 1343);
            nbt.insert_compound_tag("Level", {
                let mut nbt = CompoundTag::new();
                nbt.insert_i32("xPos", self.index.0);
                nbt.insert_i32("zPos", self.index.1);

                nbt.insert_i64("LastUpdate", 0);
                nbt.insert_i8("LightPopulated", 0);
                nbt.insert_i8("TerrainPopulated", 1);
                nbt.insert_i64("InhabitetTime", 0);

                nbt.insert_compound_tag_vec("Entities", Vec::new());
                nbt.insert_compound_tag_vec("TileEntities", Vec::new());
                // Todo: correct heightmap
                nbt.insert_i8_vec("HeightMap", vec![0; 16*16]);

                // Minecraft actually loads the chunk if the biomes tag is missing,
                // but regenerates the biomes incorrectly
                nbt.insert_i8_vec("Biomes", 
                    self.biomes.iter().map(|biome|biome.to_bytes() as i8).collect()
                );

                nbt.insert_compound_tag_vec("Sections", {
                    self.sections.iter().enumerate().filter_map(|(y_index, section)|
                        if let Some(section) = section {
                            let mut nbt = CompoundTag::new();
                            nbt.insert_i8("Y", y_index as i8);
                            let mut block_ids = Vec::new();
                            let mut block_data = Vec::new();
                            for (i, block) in section.blocks.iter().enumerate() {
                                let (id, data) = block.to_bytes();
                                block_ids.push(id as i8);
                                if i % 2 == 0 {
                                    block_data.push(data as i8);
                                } else {
                                    let prev_data = block_data.last_mut().unwrap();
                                    *prev_data = ((*prev_data as u8) + (data << 4)) as i8;
                                }
                            }
                            nbt.insert_i8_vec("Blocks", block_ids);
                            nbt.insert_i8_vec("Data", block_data);

                            // Todo: correct lighting (without these tags, minecraft rejects the chunk)
                            // maybe use commandblocks to force light update?
                            nbt.insert_i8_vec("BlockLight", vec![0; 16*16*16/2]);
                            nbt.insert_i8_vec("SkyLight", vec![0; 16*16*16/2]);

                            Some(nbt)
                        } else {
                            None
                        }
                    ).collect()
                });

                nbt.insert_compound_tag_vec("Entities", 
                    self.entities.iter().map(Entity::to_nbt).collect());

                nbt
            });
            nbt
        })
    }
}


pub struct Section {
    blocks: [Block; 16 * 16 * 16]
}

impl Default for Section {
    fn default() -> Self {
        Section {
            blocks: [Block::Air; 16 * 16 * 16]
        }
    }
}