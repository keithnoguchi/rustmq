// SPDX-License-Identifier: GPL-2.0
//! Flatbuffer tutorial module, explained in [tutorial](https://google.github.io/flatbuffers/flatbuffers_guide_tutorial.html).

#[allow(unused_imports)]
use flatbuffers::{FlatBufferBuilder, WIPOffset};
#[allow(unused_imports)]
use gen::my_game::sample::{
    self, get_root_as_monster, Color, Equipment, MonsterArgs, Vec3, Weapon, WeaponArgs,
};

/// Flatbuffer auto-generated sample module explained in the [tutorial](https://google.github.io/flatbuffers/flatbuffers_guide_tutorial.html).
pub mod gen {
    #![allow(
        unused_imports,
        clippy::extra_unused_lifetimes,
        clippy::needless_lifetimes,
        clippy::redundant_closure,
        clippy::redundant_static_lifetimes
    )]
    include!("../flatbuf/monster_generated.rs");
}

struct Monster;

impl Monster {
    #[allow(dead_code)]
    fn create<'b>(b: &mut FlatBufferBuilder<'b>, name: &str) -> WIPOffset<sample::Monster<'b>> {
        let name1 = b.create_string("Axe");
        let name2 = b.create_string("Sword");
        println!("axe name: {:?}", name1);
        let axe = Weapon::create(
            b,
            &WeaponArgs {
                name: Some(name1),
                damage: 5,
            },
        );
        println!("axe: {:?}", axe);
        println!("sword name: {:?}", name2);
        let sword = Weapon::create(
            b,
            &WeaponArgs {
                name: Some(name2),
                damage: 3,
            },
        );
        println!("sword: {:?}", sword);
        let weapons = b.create_vector(&[axe, sword]);
        println!("weapons: {:?}", weapons);
        let name = b.create_string(name);
        println!("name: {:?}", name);
        let inventory = b.create_vector(&[0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        println!("inventory: {:?}", inventory);
        let x = Vec3::new(1.0, 2.0, 3.0);
        println!("x: {:?}", x);
        let y = Vec3::new(4.0, 5.0, 6.0);
        println!("x: {:?}", y);
        let path = b.create_vector(&[x, y]);
        println!("path: {:?}", path);
        let orc = sample::Monster::create(
            b,
            &MonsterArgs {
                pos: Some(&Vec3::new(1.0f32, 2.0f32, 3.0f32)),
                //mana: 150, // It's a default value which is filled in below
                hp: 80,
                name: Some(name),
                inventory: Some(inventory),
                color: Color::Red,
                weapons: Some(weapons),
                equipped_type: Equipment::Weapon,
                equipped: Some(axe.as_union_value()),
                path: Some(path),
                ..Default::default()
            },
        );
        println!("monster: {:?}", orc);
        orc
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builder_with_different_capacities() {
        let capacities = [1usize, 16, 32, 64, 128, 256, 1024, 2048, 4096];
        for &t in &capacities {
            let _builder = FlatBufferBuilder::new_with_capacity(t);
        }
    }
    #[test]
    fn serialize_sword_and_axe() {
        let mut b = FlatBufferBuilder::new();
        let name = b.create_string("Sword");
        let _sword = Weapon::create(
            &mut b,
            &WeaponArgs {
                name: Some(name),
                damage: 3,
            },
        );
        let name = b.create_string("Axe");
        let _axe = Weapon::create(
            &mut b,
            &WeaponArgs {
                name: Some(name),
                damage: 5,
            },
        );
    }
    #[test]
    fn serialize_weapons() {
        let mut b = FlatBufferBuilder::new_with_capacity(1);
        let name = b.create_string("Sword");
        let sword = Weapon::create(
            &mut b,
            &WeaponArgs {
                name: Some(name),
                damage: 3,
            },
        );
        let name = b.create_string("Axe");
        let axe = Weapon::create(
            &mut b,
            &WeaponArgs {
                name: Some(name),
                damage: 5,
            },
        );
        let _weapons = b.create_vector(&[sword, axe]);
    }
    #[test]
    fn serialize_monster() {
        let mut builder = FlatBufferBuilder::new_with_capacity(1);
        let orc = super::Monster::create(&mut builder, "ore");
        builder.finish(orc, None);
    }
    #[test]
    fn serialize_and_deserialize_monster() {
        let mut builder = FlatBufferBuilder::new();
        let godzilla = super::Monster::create(&mut builder, "godzilla");
        builder.finish(godzilla, None);
        let _buf = builder.finished_data(); // Of type `&[u8]`
    }
}
