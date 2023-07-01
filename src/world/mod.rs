// TODO: check if empty sections really are (and stay) None

mod biome;
mod block;
mod entity;
pub mod vanilla_village;

use anvil_region::{
    position::{RegionChunkPosition, RegionPosition},
    provider::{FolderRegionProvider, RegionProvider},
};
use anyhow::{anyhow, Result};
use itertools::Itertools;
use nbt::CompoundTag;
use rayon::prelude::*;
use std::{
    collections::HashMap,
    ops::Shr,
    path::{Path, PathBuf},
    sync::Mutex,
};

use crate::geometry::*;
pub use biome::*;
pub use block::*;
pub use entity::*;
use vanilla_village::Village;

// Ugh, can't impl Index and IndexMut because of orphan rules
pub trait WorldView {
    fn get(&self, pos: Pos) -> &Block;
    fn get_mut(&mut self, pos: Pos) -> &mut Block;
    fn get_mut_no_update_order(&mut self, pos: Pos) -> &mut Block;

    fn biome(&self, column: Column) -> Biome;

    /// Height of the ground, ignores vegetation
    fn height(&self, column: Column) -> i32;
    fn height_mut(&mut self, column: Column) -> &mut i32;

    fn water_level(&self, column: Column) -> Option<i32>;
    fn water_level_mut(&mut self, column: Column) -> &mut Option<i32>;

    fn area(&self) -> Rect;

    /// Convenience method
    fn set(&mut self, pos: Pos, block: impl BlockOrRef) {
        *self.get_mut(pos) = block.get();
    }
    fn set_override(&mut self, pos: Pos, block: impl BlockOrRef) {
        *self.get_mut_no_update_order(pos) = block.get();
    }
    /// Convenience method
    fn set_if_not_solid(&mut self, pos: Pos, block: impl BlockOrRef) {
        let block_ref = self.get_mut(pos);
        if !block_ref.solid() {
            *block_ref = block.get();
        }
    }
}

pub trait BlockOrRef {
    fn get(self) -> Block;
}

impl BlockOrRef for Block {
    fn get(self) -> Block {
        self
    }
}

impl BlockOrRef for &Block {
    fn get(self) -> Block {
        self.clone()
    }
}

// Maybe have a subworld not split into chunks for efficiency?
pub struct World {
    pub path: PathBuf,
    /// Loaded area; aligned with chunk borders (-> usually larger than area specified in new())
    /// Both minimum and maximum inclusive
    chunk_min: ChunkIndex,
    chunk_max: ChunkIndex,
    /// Sections in Z->X->Y order
    sections: Vec<Option<Box<Section>>>,
    /// Minecraft stores biomes in 3d, but we only store 2d (at height 64)
    biome: Vec<Biome>,
    heightmap: Vec<i32>,
    watermap: Vec<Option<i32>>,
    pub entities: Vec<Entity>,
    pub villages: Vec<Village>,
}

impl World {
    // No nice error handling, but we don't really need that for just the three invocations
    pub fn new(path: &str, area: Rect) -> Self {
        let region_path = {
            let mut region_path = PathBuf::from(path);
            region_path.push("region");
            region_path.into_os_string().into_string().unwrap()
        };
        let chunk_provider = FolderRegionProvider::new(&region_path);
        let chunk_min: ChunkIndex =
            (area.min - Vec2(crate::LOAD_MARGIN, crate::LOAD_MARGIN)).into();
        let chunk_max: ChunkIndex =
            (area.max + Vec2(crate::LOAD_MARGIN, crate::LOAD_MARGIN)).into();

        let chunk_count =
            ((chunk_max.0 - chunk_min.0 + 1) * (chunk_max.1 - chunk_min.1 + 1)) as usize;

        let mut sections = vec![None; chunk_count * 24];
        let mut biome = vec![Biome::default(); chunk_count * 4 * 4];
        let mut heightmap = vec![0; chunk_count * 16 * 16];
        let mut watermap = vec![None; chunk_count * 16 * 16];
        let mut villages = Vec::new();

        let villages_mutex = Mutex::new(&mut villages);

        // Load chunks. Collecting indexes to vec neccessary for zip
        (chunk_min.1..=chunk_max.1)
            .flat_map(|z| (chunk_min.0..=chunk_max.0).map(move |x| (x, z)))
            .collect_vec()
            .par_iter() //TMP no par
            .zip(sections.par_chunks_exact_mut(24))
            .zip(biome.par_chunks_exact_mut(4 * 4))
            .zip(heightmap.par_chunks_exact_mut(16 * 16))
            .zip(watermap.par_chunks_exact_mut(16 * 16))
            .for_each(|((((index, sections), biome), heightmap), watermap)| {
                load_chunk(
                    &chunk_provider,
                    (*index).into(),
                    sections,
                    biome,
                    heightmap,
                    watermap,
                    &villages_mutex,
                )
                .expect(&format!("Failed to load chunk ({},{}): ", index.0, index.1))
            });

        // Check if there are some villages in the 1.12 format
        if let Ok(mut village_dat) = std::fs::File::open(Path::new(path).join("data/Village.dat")) {
            let nbt = nbt::decode::read_gzip_compound_tag(&mut village_dat).unwrap();
            let nbt = nbt.get_compound_tag("data").unwrap();
            for (_, nbt) in nbt.get_compound_tag("Features").unwrap().iter() {
                if let nbt::Tag::Compound(nbt) = nbt {
                    villages.push(Village::from_nbt(nbt));
                }
            }
        }

        Self {
            path: PathBuf::from(path),
            chunk_min,
            chunk_max,
            sections,
            biome,
            heightmap,
            watermap,
            villages,
            entities: Vec::new(),
        }
    }

    pub fn save(&self) -> Result<()> {
        // Write chunks
        let mut region_path = self.path.clone();
        region_path.push("region");
        // Internally, AnvilChunkProvider stores a path. So why require a str??
        let region_path = region_path.into_os_string().into_string().unwrap();
        let chunk_provider = FolderRegionProvider::new(&region_path);

        let chunk_count = ((self.chunk_max.0 - self.chunk_min.0 + 1)
            * (self.chunk_max.1 - self.chunk_min.1 + 1)) as usize;
        let mut entities_chunked = vec![vec![]; chunk_count];
        for entity in &self.entities {
            entities_chunked[self.chunk_index(entity.pos.into())].push(entity);
        }

        // Saveing isn't thread safe
        for ((index, sections), entities) in (self.chunk_min.1..=self.chunk_max.1)
            .flat_map(|z| (self.chunk_min.0..=self.chunk_max.0).map(move |x| (x, z)))
            .zip(self.sections.chunks_exact(24))
            .zip(entities_chunked)
        {
            // Don't save outermost chunks, since we don't modify them & leaving out the border simplifies things
            if (index.0 > self.chunk_min.0)
                & (index.0 < self.chunk_max.0)
                & (index.1 > self.chunk_min.1)
                & (index.1 < self.chunk_max.1)
            {
                save_chunk(&chunk_provider, index.into(), sections, &entities)
                    .unwrap_or_else(|_| panic!("Failed to save chunk ({},{}): ", index.0, index.1))
            }
        }

        // Edit metadata
        let level_nbt_path =
            self.path.clone().into_os_string().into_string().unwrap() + "/level.dat";
        let mut file = std::fs::File::open(&level_nbt_path).expect("Failed to open level.dat");
        let mut nbt =
            nbt::decode::read_gzip_compound_tag(&mut file).expect("Failed to open level.dat");
        let data: &mut CompoundTag = nbt.get_mut("Data").expect("Corrupt level.dat");

        let name: &mut String = data.get_mut("LevelName").expect("Corrupt level.dat");
        name.push_str(" [generated]");

        data.insert(
            "LastPlayed",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
        );

        data.insert_i8("Difficulty", 0);

        let gamerules: &mut CompoundTag = data.get_mut("GameRules").unwrap();
        gamerules.insert_str("commandBlockOutput", "false");
        gamerules.insert_str("gameLoopFunction", "mc-gen:loop");

        // Set spawn to the center of the area to ensure all command blocks stay loaded
        data.insert_i32("SpawnX", self.area().center().0);
        data.insert_i32("SpawnZ", self.area().center().1);

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(&level_nbt_path)
            .expect("Failed to open level.dat");
        nbt::encode::write_gzip_compound_tag(&mut file, &nbt).expect("Failed to write level.dat");
        Ok(())
    }

    pub fn redstone_processing_area(&self) -> Rect {
        let min = self.area().center() - Vec2(111, 111);
        let max = self.area().center() + Vec2(111, 111);
        Rect {
            min: Column((min.0 / 16) * 16, (min.1 / 16) * 16),
            max: Column((max.0 / 16) * 16 + 15, (max.1 / 16) * 16 + 15),
        }
    }

    fn chunk_index(&self, chunk: ChunkIndex) -> usize {
        if (chunk.0 < self.chunk_min.0)
            | (chunk.0 > self.chunk_max.0)
            | (chunk.1 < self.chunk_min.1)
            | (chunk.1 > self.chunk_max.1)
        {
            panic!("Out of bounds access to chunk {}, {}", chunk.0, chunk.1);
        } else {
            ((chunk.0 - self.chunk_min.0)
                + (chunk.1 - self.chunk_min.1) * (self.chunk_max.0 - self.chunk_min.0 + 1))
                as usize
        }
    }

    fn section_index(&self, pos: Pos) -> usize {
        self.chunk_index(pos.into()) * 24 + (pos.1 / 16 + 4) as usize
    }

    fn column_index(&self, column: Column) -> usize {
        self.chunk_index(column.into()) * 16 * 16
            + (column.0.rem_euclid(16) + column.1.rem_euclid(16) * 16) as usize
    }

    fn block_in_section_index(pos: Pos) -> usize {
        (pos.0.rem_euclid(16) + pos.1.rem_euclid(16) * 16 * 16 + pos.2.rem_euclid(16) * 16) as usize
    }

    pub fn chunk_min(&self) -> ChunkIndex {
        self.chunk_min
    }

    pub fn chunk_max(&self) -> ChunkIndex {
        self.chunk_max
    }

    pub fn chunks(&self) -> impl Iterator<Item = ChunkIndex> {
        (self.chunk_min.0..=self.chunk_max.0)
            .cartesian_product(self.chunk_min.1..=self.chunk_max.1)
            .map(|(x, z)| ChunkIndex(x, z))
    }

    pub fn area(&self) -> Rect {
        Rect {
            min: Column(self.chunk_min.0 * 16, self.chunk_min.1 * 16),
            max: Column(self.chunk_max.0 * 16 + 15, self.chunk_max.1 * 16 + 15),
        }
        .shrink(crate::LOAD_MARGIN)
    }
}

impl WorldView for World {
    fn get(&self, pos: Pos) -> &Block {
        if let Some(section) = &self.sections[self.section_index(pos)] {
            &section.blocks[Self::block_in_section_index(pos)]
        } else {
            &Block::Air
        }
    }

    fn get_mut(&mut self, pos: Pos) -> &mut Block {
        let index = self.section_index(pos);
        let section = self.sections[index].get_or_insert_default();
        &mut section.blocks[Self::block_in_section_index(pos)]
    }

    fn get_mut_no_update_order(&mut self, pos: Pos) -> &mut Block {
        self.get_mut(pos)
    }

    fn biome(&self, column: Column) -> Biome {
        if let Some(biome) = self.biome.get(
            self.chunk_index(column.into()) * 4 * 4
                + (column.0.rem_euclid(16) / 4 + column.1.rem_euclid(16) / 4 * 4) as usize,
        ) {
            *biome
        } else {
            panic!("Tried to access biome at {:?}", column);
        }
    }

    fn height(&self, column: Column) -> i32 {
        self.heightmap[self.column_index(column)]
    }

    fn height_mut(&mut self, column: Column) -> &mut i32 {
        let index = self.column_index(column);
        &mut self.heightmap[index]
    }

    fn water_level(&self, column: Column) -> Option<i32> {
        self.watermap[self.column_index(column)]
    }

    fn water_level_mut(&mut self, column: Column) -> &mut Option<i32> {
        let index = self.column_index(column);
        &mut self.watermap[index]
    }

    fn area(&self) -> Rect {
        Rect {
            min: Column(self.chunk_min.0 * 16, self.chunk_min.1 * 16),
            max: Column(self.chunk_max.0 * 16 + 15, self.chunk_max.1 * 16 + 15),
        }
    }
}

fn load_chunk(
    chunk_provider: &FolderRegionProvider,
    chunk_index: ChunkIndex,
    sections: &mut [Option<Box<Section>>],
    _biomes: &mut [Biome],
    heightmap: &mut [i32],
    watermap: &mut [Option<i32>],
    villages: &Mutex<&mut Vec<Village>>,
) -> Result<()> {
    let nbt = chunk_provider
        .get_region(RegionPosition::from_chunk_position(
            chunk_index.0,
            chunk_index.1,
        ))?
        .read_chunk(RegionChunkPosition::from_chunk_position(
            chunk_index.0,
            chunk_index.1,
        ))
        .map_err(|_| anyhow!("Chunk read error"))?;
    let version = nbt.get_i32("DataVersion").unwrap();
    if version != 3465 {
        println!(
            "Unsupported version: {}. Only 1.20.1 is currently tested.",
            version
        );
    }

    if let Ok(structures) = nbt.get_compound_tag("Structures") {
        let structures = structures.get_compound_tag("Starts").unwrap();
        if let Ok(nbt) = structures.get_compound_tag("village") {
            if nbt.get_str("id").unwrap() != "INVALID" {
                villages.lock().unwrap().push(Village::from_nbt(nbt));
            }
        }
    }

    // TODO: store CarvingMasks::AIR, seems useful
    // Also, check out Heightmaps. Maybe we can reuse them or gleam additional information from them

    let sections_nbt = nbt.get_compound_tag_vec("sections").unwrap();

    for section_nbt in sections_nbt {
        let y_index = section_nbt.get_i8("Y").unwrap();

        // TODO: support full chunk height
        if !(0..15).contains(&y_index) {
            continue;
        }

        // TODO: load biome

        let block_states = section_nbt.get_compound_tag("block_states").unwrap();
        let palette = block_states.get_compound_tag_vec("palette").unwrap();
        // Build the palette. Yes, this doesn't deduplicate unrecognised blockstates between sections
        let palette: Vec<Block> = palette.iter().map(|nbt| Block::from_nbt(nbt)).collect();

        sections[(y_index + 4) as usize] = Some(Default::default());
        let section = sections[(y_index + 4) as usize].as_mut().unwrap();
        let Ok(indices) = block_states.get_i64_vec("data") else {continue};
        let bits_per_index = bits_per_index(palette.len());

        let mut current_long = 0;
        let mut current_bit_shift = 0;
        for i in 0..(16 * 16 * 16) {
            let packed = indices[current_long] as u64;
            let index = packed.shr(current_bit_shift) as usize % (1 << bits_per_index);
            section.blocks[i] = palette[index].clone();

            current_bit_shift += bits_per_index;
            if current_bit_shift > (64 - bits_per_index) {
                current_bit_shift = 0;
                current_long += 1;
            }
        }
    }

    // Build water- & heightmap
    for x in 0..16 {
        for z in 0..16 {
            'column: for section_index in (-4..20).rev() {
                if let Some(section) = &sections[(section_index + 4i32) as usize] {
                    for y in (0..16).rev() {
                        let block = &section.blocks[x + z * 16 + y as usize * 16 * 16];
                        let height = (section_index - 4) * 16 + y;
                        if match block {
                            Block::Log(..) => false,
                            _ => block.solid(),
                        } {
                            heightmap[x + z * 16] = height;
                            break 'column;
                        } else if matches!(block, Block::Water /*TODO: | Block::Ice*/) {
                            watermap[x + z * 16].get_or_insert(section_index * 16 + y);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn bits_per_index(palette_len: usize) -> usize {
    for bits in 4.. {
        if palette_len <= 1 << bits {
            return bits;
        }
    }
    unreachable!()
}

fn save_chunk(
    chunk_provider: &FolderRegionProvider,
    index: ChunkIndex,
    sections: &[Option<Box<Section>>],
    entities: &[&Entity],
) -> Result<()> {
    chunk_provider
        .get_region(RegionPosition::from_chunk_position(index.0, index.1))?
        .write_chunk(
            RegionChunkPosition::from_chunk_position(index.0, index.1),
            {
                let mut nbt = CompoundTag::new();
                nbt.insert_i32("DataVersion", 3465);
                nbt.insert_i32("xPos", index.0);
                nbt.insert_i32("zPos", index.1);

                nbt.insert_i64("LastUpdate", 0);
                nbt.insert_i8("TerrainPopulated", 1);
                nbt.insert_i64("InhabitetTime", 0);
                nbt.insert_str("Status", "full");

                // Collect tile entities
                let mut tile_entities = Vec::new();

                nbt.insert_compound_tag_vec("sections", {
                    sections
                        .iter()
                        .enumerate()
                        .filter_map(|(y_index, section)| {
                            let y_index = y_index as i32 - 4;
                            let Some(section) = section else {return None};
                            let mut nbt = CompoundTag::new();
                            nbt.insert_i8("Y", y_index as i8);

                            let mut block_states = CompoundTag::new();
                            // Build the palette first (for length)
                            // Minecraft seems to always have Air as id 0 even if there is none
                            let mut palette = HashMap::new();
                            block_states.insert_compound_tag_vec(
                                "palette",
                                Some(Air)
                                    .iter()
                                    .chain(section.blocks.iter())
                                    .flat_map(|block| {
                                        if !palette.contains_key(block) {
                                            palette.insert(block.clone(), palette.len());
                                            Some(block.to_nbt())
                                        } else {
                                            None
                                        }
                                    }),
                            );

                            let bits_per_index = bits_per_index(palette.len());
                            let mut blocks = vec![0];
                            let mut current_long = 0;
                            let mut current_bit_shift = 0;

                            for (i, block) in section.blocks.iter().enumerate() {
                                blocks[current_long] |=
                                    (palette[block] << current_bit_shift) as i64;
                                current_bit_shift += bits_per_index;
                                if current_bit_shift > 64 - bits_per_index {
                                    current_bit_shift = 0;
                                    current_long += 1;
                                    // If there's an unnecessary empty long at the end,
                                    // the chunk can't be loaded
                                    if (i < 4095) | (64 % bits_per_index != 0) {
                                        blocks.push(0);
                                    }
                                }

                                // Collect TileEntity data
                                {
                                    let section_base =
                                        Pos(index.0 * 16, y_index * 16, index.1 * 16);
                                    let pos = section_base
                                        + Vec3(
                                            i as i32 % 16,
                                            i as i32 / (16 * 16),
                                            i as i32 % (16 * 16) / 16,
                                        );
                                    tile_entities.extend(block.tile_entity_nbt(pos));
                                }
                            }
                            block_states.insert_i64_vec("data", blocks);
                            nbt.insert("block_states", block_states);

                            Some(nbt)
                        })
                });

                nbt.insert_compound_tag_vec("block_entities", tile_entities);

                nbt
            },
        )
        .map_err(|_| anyhow!("Chunk write error"))?;
    Ok(())
}

#[derive(Clone)]
pub struct Section {
    blocks: [Block; 16 * 16 * 16],
}

impl Default for Section {
    fn default() -> Self {
        const AIR: Block = Block::Air;
        Section {
            blocks: [AIR; 16 * 16 * 16],
        }
    }
}
