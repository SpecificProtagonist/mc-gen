use hashbrown::HashSet;

use crate::*;

pub fn ground(level: &mut Level, area: Rect) {
    for column in area {
        let base_height = if let Some(water_height) = level.water_level(column) {
            water_height
        } else {
            level.height(column)
        };
        for z in base_height + 1..=base_height + 2 {
            let block = &mut level[column.extend(z)];
            if matches!(block, GroundPlant(..)) {
                *block = Block::Air
            }
        }
    }
}

pub fn find_trees(level: &Level, area: impl IntoIterator<Item = IVec2>) -> Vec<IVec3> {
    let mut trees = HashSet::new();
    for column in area {
        let z = level.height(column) + 1;
        if let Block::Log(..) = level[column.extend(z)] {
            // Check whether this is a tree instead of part of a man-made structure
            let mut pos = column.extend(z);
            while let Block::Log(..) = level[pos] {
                pos += IVec3::Z;
            }
            if !matches!(level[pos], Leaves(..)) {
                continue;
            }
            // Find origin
            // TODO: find connected blocks to make this work for all kinds of trees
            let mut pos = column.extend(z);
            if let Block::Log(..) = level[pos - IVec3::X] {
                pos -= IVec3::X
            }
            if let Block::Log(..) = level[pos - IVec3::Y] {
                pos -= IVec3::Y
            }
            trees.insert(pos);
        }
    }
    trees.into_iter().collect()
}

pub fn remove_tree(level: &mut Level, pos: IVec3) {
    let Log(species, ..) = level[pos] else {
        println!("Tried to remove tree at {pos:?} but not found");
        return;
    };
    // Store distance from log, 0 means log
    let mut blocks = vec![(pos, 0)];
    while let Some((pos, distance)) = blocks.pop() {
        level[pos] = Air;
        for off_x in -1..=1 {
            for off_y in -1..=1 {
                for off_z in -1..=1 {
                    let off = ivec3(off_x, off_y, off_z);
                    let pos = pos + off;
                    match level[pos] {
                        Log(s, ..) if (s == species) & (distance <= 1) => blocks.push((pos, 0)),
                        // Checking species can leave leaves behind when trees intersect
                        // Also, azalea
                        Leaves(_, Some(d)) if (d > distance) & (off.length_squared() == 1) => {
                            blocks.push((pos, d))
                        }
                        // TODO: Beehives
                        // TODO: Snoe
                        _ => (),
                    }
                }
            }
        }
    }
}

// Todo: remove_giant_mushroom()
