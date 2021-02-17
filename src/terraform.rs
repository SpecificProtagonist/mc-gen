use std::{collections::HashMap, usize};

use crate::*;

pub enum WallCrest {
    None,
    Full,
    Fence,
    Wall,
}

pub fn make_retaining_wall(
    world: &mut impl WorldView,
    area: &Polygon,
    height: u8,
    crest: WallCrest,
) {
    // Placement order matters for replay -> build wall first
    let wall_block = &Stone(Stone::Cobble);
    let crest = &match crest {
        WallCrest::None => Air,
        WallCrest::Full => wall_block.clone(),
        WallCrest::Fence => {
            Block::Fence(Fence::Wood(world.biome(area.0[0]).default_tree_species()))
        }
        WallCrest::Wall => Block::Fence(Fence::Stone { mossy: false }),
    };

    for column in area.border(LineStyle::ThickWobbly) {
        let mut y = world.heightmap(column);
        // Check if wall is neccessary
        if y > height || (y == height && !side_exposted(world, column.at(y))) {
            // Todo: also skip this column if the only exposed side is within the polygon
            continue;
        }

        // Build wall
        while matches!(world.get(column.at(y)), Soil(_)) {
            y -= 1;
        }
        for y in y..=height {
            world.set(column.at(y), wall_block)
        }
        let above = world.get_mut(column.at(height + 1));
        if matches!((crest, &above), (Air, GroundPlant(_))) {
            *above = Air
        } else {
            *above = crest.clone()
        }

        *world.heightmap_mut(column) = height;
    }

    // Then fill
    // TODO: bottom to top
    for column in area.iter() {
        if world.heightmap(column) < height {
            let soil = &Soil(get_filling_soil(world, column));
            for y in world.heightmap(column)..=height {
                world.set(column.at(y), soil)
            }
            *world.heightmap_mut(column) = height;
        }
    }
}

fn get_filling_soil(world: &impl WorldView, column: Column) -> Soil {
    if let Soil(soil) = *world.get(column.at(world.heightmap(column))) {
        soil
    } else {
        world.biome(column).default_topsoil()
    }
}

pub fn make_foundation(world: &mut impl WorldView, area: Rect, height: u8, block: BuildBlock) {
    for column in area.iter() {
        world.set(column.at(height), block.full());
        let mut y = height - 1;
        let ground_height = world.heightmap(column);
        while (y > ground_height) | soil_exposted(world, column.at(y)) {
            world.set(column.at(y), block.full());
            y -= 1;
        }
        for y in (height + 1)..=ground_height {
            world.set(column.at(y), Air);
        }
    }

    make_support(
        world,
        ((area.min.0 + 1)..area.max.0).map(|x| Column(x, area.min.1)),
        height,
        HDir::ZPos,
        block,
    );
    make_support(
        world,
        ((area.min.0 + 1)..area.max.0).map(|x| Column(x, area.max.1)),
        height,
        HDir::ZNeg,
        block,
    );
    make_support(
        world,
        ((area.min.1 + 1)..area.max.1).map(|z| Column(area.min.0, z)),
        height,
        HDir::XPos,
        block,
    );
    make_support(
        world,
        ((area.min.1 + 1)..area.max.1).map(|z| Column(area.max.0, z)),
        height,
        HDir::XNeg,
        block,
    );

    fn make_support(
        world: &mut impl WorldView,
        columns: impl Iterator<Item = Column>,
        y: u8,
        facing: HDir,
        block: BuildBlock,
    ) {
        let support_chance = 0.7;
        let min_height = 3;
        let max_height = 6;
        let mut just_placed = false;
        for column in columns {
            let column = column - Vec2::from(facing);
            let mut ground_distance = y.saturating_sub(world.heightmap(column));
            while soil_exposted(world, column.at(y - ground_distance - 1)) {
                ground_distance += 1;
            }
            just_placed = if (ground_distance >= min_height)
                & (ground_distance <= max_height)
                & !just_placed
                & rand(support_chance)
            {
                world.set(column.at(y), Stair(block, facing, Flipped(false)));
                for y in (y - ground_distance as u8)..y {
                    world.set(column.at(y), block.full());
                }
                true
            } else {
                false
            };
        }
    }
}

pub fn soil_exposted(world: &impl WorldView, pos: Pos) -> bool {
    matches!(world.get(pos), Soil(..)) & side_exposted(world, pos)
}

pub fn side_exposted(world: &impl WorldView, pos: Pos) -> bool {
    !(world.get(pos + Vec2(0, 1)).solid()
        && world.get(pos + Vec2(0, -1)).solid()
        && world.get(pos + Vec2(1, 0)).solid()
        && world.get(pos + Vec2(-1, 0)).solid())
}

pub fn average_height(world: &impl WorldView, area: impl Iterator<Item = Column>) -> u8 {
    let mut sum = 0.0;
    let mut count = 0;
    for column in area {
        sum += world.heightmap(column) as f32;
        count += 1;
    }
    (sum / count as f32) as u8
}

pub fn slope(world: &impl WorldView, column: Column) -> Vec2 {
    let mut neighbors = [0; 9];
    for dx in -1..=1 {
        for dz in -1..=1 {
            neighbors[(4 + dx + 3 * dz) as usize] = world.heightmap(column + Vec2(dx, dz)) as i32;
        }
    }
    // Sobel kernel
    let slope_x = (neighbors[2] + 2 * neighbors[5] + neighbors[8])
        - (neighbors[0] + 2 * neighbors[3] + neighbors[6]);
    let slope_z = (neighbors[6] + 2 * neighbors[7] + neighbors[8])
        - (neighbors[0] + 2 * neighbors[1] + neighbors[2]);
    Vec2(slope_x, slope_z)
}

/*
/// Neighborborhood_size specifies a square. Results aren't fully acurate, but that's ok
pub fn find_local_maxima(world: &impl WorldView, area: Rect, neighborhood_size: u8) -> Vec<Pos> {
    // Divide area into cells
    let cell_size = neighborhood_size as i32 / 3;
    let cell_count = area.size() / cell_size;
    // Actually searched area is rounded down to integer number of cells
    let area = {
        let min = area.min + (area.size() % cell_size) / 2;
        Rect {
            min,
            max: min + cell_count * cell_size,
        }
    };
    for z in (area.min.1..area.max.1).step_by(cell_size as usize) {
        for x in (area.min.0..area.max.0).step_by(cell_size as usize) {
            Rect {
                min: Vec2(x, z),
                max: Vec2(x + cell_size, z + cell_size),
            }
            .iter()
        }
    }
    // find highest in each cell
    // return highest in cell when n higher in surrounding cells

    todo!()
}
*/

// TODO: add average
// TODO: move into World, cache
pub fn max_chunk_heights(world: &World) -> HashMap<ChunkIndex, u8> {
    world
        .chunks()
        .map(|chunk| {
            (
                chunk,
                chunk
                    .area()
                    .iter()
                    .map(|column| world.heightmap(column))
                    .max()
                    .unwrap(),
            )
        })
        .collect()
}
