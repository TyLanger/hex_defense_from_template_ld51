use bevy::{
    prelude::*,
    sprite::{collide_aabb::collide, MaterialMesh2dBundle},
    utils::FloatOrd,
};
use bevy_rapier2d::prelude::*;

use crate::{
    enemies::{BossSpawnEvent, Dead, Enemy},
    gold::*,
    hex::*,
    palette::ORANGE,
    tutorial::AcceptInput,
    MouseWorldPos,
};

const TOWER_COST_GROWTH: u32 = 2;
const TOWERS_TO_SPAWN_BOSS: u32 = 10; //10
pub struct TowerPlugin;

impl Plugin for TowerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<TowerBuiltEvent>()
            .add_event::<TowerRemoveEvent>()
            .add_event::<PlaceTowerPreviewEvent>()
            .add_event::<SpawnBulletEvent>()
            .add_event::<SpawnBombBulletEvent>()
            //.add_system(spawn_tower)
            //.add_system(tower_input)
            .insert_resource(TowerSpawnCost { cost: 5 })
            .insert_resource(TowerCount {
                count: 0,
                boss_spawned: false,
            })
            .add_system(tower_mouse_input)
            .add_system(tower_key_input)
            .add_system(spawn_tower_preview)
            .add_system(preview_paid_for)
            .add_system(remove_tower)
            .add_system(tower_shoot)
            .add_system(spawn_bullet)
            .add_system(tick_bullet)
            .add_system(move_bullet)
            .add_system(bullet_hit)
            .add_system(spawn_bomb_bullet)
            .add_system(tick_bomb_bullet)
            .add_system(bomb_tower_build)
            .add_system(tick_bomb_explosion)
            .add_system(bomb_test);
        //.add_system(rotate_sprite);
    }
}

#[derive(Component)]
pub struct Tower {
    pub coords: HexCoords,
    pub refund: u32,
    shoot_type: ShootType,
    shoot_timer: Timer,
    can_shoot: bool,
    range: f32,
}

impl Tower {
    pub fn new(coords: HexCoords, refund: u32) -> Self {
        Tower {
            coords,
            refund,
            shoot_type: ShootType::Bullet,
            shoot_timer: Timer::from_seconds(1.0, true),
            can_shoot: true,
            range: 200.0,
        }
    }
}

#[derive(PartialEq)]
enum ShootType {
    Bullet,
    Arc,
    //Pulse,
    //Laser,
}

#[derive(Component)]
pub struct TowerPreview {}

#[derive(Bundle)]
pub struct PreviewTowerBundle {
    pub preview: TowerPreview,
    pub pile: GoldPile,
}

#[derive(Component)]
struct TowerSprite;

struct PlaceTowerPreviewEvent {
    //position: Vec3,
    coords: HexCoords,
    shoot_type: ShootType,
}

// successfully build
pub struct TowerBuiltEvent {
    pub coords: HexCoords,
}

struct TowerRemoveEvent {
    coords: HexCoords,
}

struct TowerSpawnCost {
    cost: u32,
}

struct TowerCount {
    count: u32,
    boss_spawned: bool,
}

// fn rotate_sprite(
//     mut q_tower: Query<&mut Transform, With<Tower>>,
//     time: Res<Time>,
// ) {
//     for mut tower in q_tower.iter_mut() {
//         // doesn't look very good
//         //tower.rotate_x(time.delta_seconds() * 2.0);
//         // rotating around z looks fine. Kinda looks like a spinning attack charge up

//         // this looks much better
//         let y = (time.seconds_since_startup() * 5.0).sin() as f32;
//         tower.scale = Vec3{x: 1.0, y: y, z: 1.0};
//         // how to look once/twice?
//         // Timer?
//         // add a component, run a timer, remove component?
//     }
// }

fn tower_mouse_input(
    mut ev_place_preview: EventWriter<PlaceTowerPreviewEvent>,
    q_selection: Query<&Hex, With<Selection>>,
    input: Res<Input<MouseButton>>,
    accept: Res<AcceptInput>,
) {
    if accept.0 && input.just_pressed(MouseButton::Left) {
        for hex in q_selection.iter() {
            ev_place_preview.send(PlaceTowerPreviewEvent {
                //position: trans.translation,
                coords: hex.coords,
                shoot_type: ShootType::Bullet,
            });
        }
    }
    // spawn a bomb tower
    if accept.0 && input.just_pressed(MouseButton::Right) {
        for hex in q_selection.iter() {
            ev_place_preview.send(PlaceTowerPreviewEvent {
                //position: trans.translation,
                coords: hex.coords,
                shoot_type: ShootType::Arc,
            });
        }
    }
}

fn tower_key_input(
    mut ev_remove_tower: EventWriter<TowerRemoveEvent>,
    q_selection: Query<&Hex, With<Selection>>,
    input: Res<Input<KeyCode>>,
) {
    if input.just_pressed(KeyCode::X) {
        for hex in q_selection.iter() {
            ev_remove_tower.send(TowerRemoveEvent { coords: hex.coords });
        }
    }
}

// where a tower will be
// Still needs gold brought to it to build it
fn spawn_tower_preview(
    mut commands: Commands,
    mut ev_place_preview: EventReader<PlaceTowerPreviewEvent>,
    q_empty_hexes: Query<
        Entity,
        (
            With<Hex>,
            (Without<TowerPreview>, Without<Tower>, Without<GoldPile>),
        ),
    >,
    asset_server: Res<AssetServer>,
    mut cost: ResMut<TowerSpawnCost>,
    hex_collect: Res<HexCollection>,
) {
    for ev in ev_place_preview.iter() {
        // use the hashmap of hexes
        // instead of iterating over all of them.
        // if let Some
        // if let Ok
        // replaces
        // for
        // if ==
        // it's probably faster
        if let Some(&e) = hex_collect.hexes.get(&ev.coords) {
            if let Ok(ent) = q_empty_hexes.get(e) {
                // empty hex exists
                commands
                    .entity(ent)
                    .insert_bundle(PreviewTowerBundle {
                        preview: TowerPreview {},
                        pile: GoldPile::new(cost.cost),
                    })
                    .with_children(|parent| {
                        parent
                            .spawn_bundle(SpriteBundle {
                                texture: asset_server.load("sprites/UnbuiltTower.png"),
                                // sprite: Sprite {
                                //     color: LIGHT_BLUE,
                                //     custom_size: Some(Vec2::new(20.0, 20.0)),
                                //     ..default()
                                // },
                                transform: Transform {
                                    // spawn on top of the underlying hex
                                    translation: Vec3 {
                                        x: 0.0,
                                        y: 0.0,
                                        z: 0.2,
                                    },
                                    // undo the hex's rotation
                                    rotation: Quat::from_rotation_z(-30.0 * DEG_TO_RAD),
                                    ..default()
                                },
                                ..default()
                            })
                            .insert(TowerSprite);
                    });

                if ev.shoot_type == ShootType::Arc {
                    commands.entity(ent).insert(BombTower);
                }
                // it is now a Hex, TowerPreview, GoldPile,
                // with a sprite child
                cost.cost += TOWER_COST_GROWTH;
            }
        }
    }
}

fn preview_paid_for(
    mut commands: Commands,
    mut ev_pile_cap: EventReader<PileCapEvent>,
    q_preview_towers: Query<(Entity, &Children), (With<Hex>, With<GoldPile>, With<TowerPreview>)>,
    mut q_child: Query<&mut Handle<Image>, With<TowerSprite>>,
    asset_server: Res<AssetServer>,
    mut tower_count: ResMut<TowerCount>,
    mut ev_boss: EventWriter<BossSpawnEvent>,
    mut ev_remove_pile: EventWriter<PileRemoveEvent>,
    hex_collect: Res<HexCollection>,
) {
    for ev in ev_pile_cap.iter() {
        if let Some(&e) = hex_collect.hexes.get(&ev.coords) {
            if let Ok((ent, children)) = q_preview_towers.get(e) {
                // for (ent, children, hex, _pile) in q_preview_towers.iter() {
                //     if ev.coords == hex.coords {
                //println!("Upgrade {:?}", hex.coords);

                // change the color of the preview to a tower color
                for &child in children.iter() {
                    let sprite = q_child.get_mut(child);

                    // change the sprite of the preview tower sprite to the built tower
                    if let Ok(mut s) = sprite {
                        *s = asset_server.load("sprites/Tower.png");
                    }
                }

                commands
                    .entity(ent)
                    //.remove_children(children)
                    //.remove_bundle::<PreviewTowerBundle>()
                    .remove::<TowerPreview>()
                    .insert(Tower::new(ev.coords, (ev.amount as f32 * 0.8) as u32))
                    .insert(GoldSpawner::new());

                if !tower_count.boss_spawned {
                    tower_count.count += 1;
                    if tower_count.count == TOWERS_TO_SPAWN_BOSS {
                        tower_count.boss_spawned = true;
                        ev_boss.send(BossSpawnEvent);
                    }
                }

                ev_remove_pile.send(PileRemoveEvent { coords: ev.coords });

                break;
            }
        }
    }
}

// updates the tower to use bomb logic for shooting
fn bomb_tower_build(
    mut q_bomb: Query<(&mut Tower, &mut GoldSpawner), (Added<Tower>, With<BombTower>)>,
) {
    for (mut t, mut spawner) in q_bomb.iter_mut() {
        t.shoot_type = ShootType::Arc;
        spawner.radius = 2;
    }
}

fn remove_tower(
    mut commands: Commands,
    mut ev_remove: EventReader<TowerRemoveEvent>,
    mut ev_spawn_gold: EventWriter<SpawnGoldEvent>,
    q_towers: Query<(
        Entity,
        &Children,
        &Transform,
        &Hex,
        Option<&TowerPreview>,
        Option<&GoldPile>,
        Option<&Tower>,
        Option<&BombTower>,
    )>,
    q_sprite: Query<Entity, With<TowerSprite>>,
    mut counter: ResMut<TowerCount>,
    mut cost: ResMut<TowerSpawnCost>,
    hex_collect: Res<HexCollection>,
    //mut q_child: Query<&mut Sprite>,
) {
    for ev in ev_remove.iter() {
        if let Some(&e) = hex_collect.hexes.get(&ev.coords) {
            if let Ok((ent, children, trans, _hex, opt_preview, opt_pile, opt_tower, opt_bomb)) =
                q_towers.get(e)
            {
                // for (ent, children, trans, hex, opt_preview, opt_pile, opt_tower, opt_bomb) in
                //     q_towers.iter()
                // {
                //     if ev.coords == hex.coords {
                // this is just a gold pile, not a preview
                if opt_preview.is_none() && opt_pile.is_some() {
                    break;
                }

                if let Some(tower) = opt_tower {
                    for _ in 0..tower.refund {
                        ev_spawn_gold.send(SpawnGoldEvent {
                            position: trans.translation,
                        });
                    }
                }

                for &child in children {
                    // despawn child if it has a TowerSprite component
                    if q_sprite.get(child).is_ok() {
                        commands.entity(child).despawn_recursive();
                    }
                }

                if opt_bomb.is_some() {
                    commands.entity(ent).remove::<BombTower>();
                }

                if opt_preview.is_some() {
                    commands.entity(ent).remove::<TowerPreview>();
                }

                if opt_tower.is_some() {
                    commands.entity(ent).remove::<GoldSpawner>();
                    commands.entity(ent).remove::<Tower>();

                    if !counter.boss_spawned {
                        // probably can't underflow
                        // can only destroy a tower if it exists
                        // but to be safe
                        if counter.count > 1 {
                            counter.count -= 1;
                        }
                    }
                }
                // probably shouldn't need this check either
                if cost.cost > TOWER_COST_GROWTH {
                    cost.cost -= TOWER_COST_GROWTH;
                }
            }
        }
    }
}

fn tower_shoot(
    mut q_towers: Query<(&Transform, &mut Tower)>,
    q_enemies: Query<(&Transform, &Enemy)>,
    mut ev_shoot: EventWriter<SpawnBulletEvent>,
    mut ev_bomb: EventWriter<SpawnBombBulletEvent>,
    time: Res<Time>,
) {
    for (t_trans, mut t) in q_towers.iter_mut() {
        if !t.can_shoot {
            // tick between shots when you can't shoot
            if t.shoot_timer.tick(time.delta()).just_finished() {
                t.can_shoot = true;
            }
        } else {
            // can shoot
            // find a target

            match t.shoot_type {
                ShootType::Bullet => {
                    let direction = q_enemies
                        .iter()
                        .min_by_key(|target_transform| {
                            FloatOrd(Vec3::distance_squared(
                                target_transform.0.translation,
                                t_trans.translation,
                            ))
                        })
                        .map(|closest_target| closest_target.0.translation - t_trans.translation);

                    if let Some(direction) = direction {
                        // only shoot if within range
                        if direction.length_squared() < (t.range * t.range) {
                            ev_shoot.send(SpawnBulletEvent {
                                pos: t_trans.translation.truncate(),
                                dir: direction.truncate(),
                            });

                            t.can_shoot = false;
                        }
                    }
                }
                ShootType::Arc => {
                    let pos = q_enemies
                        .iter()
                        .min_by_key(|target_transform| {
                            FloatOrd(Vec3::distance_squared(
                                target_transform.0.translation,
                                t_trans.translation,
                            ))
                        })
                        .map(|closest| {
                            // dist travelled in 1s
                            // bomb travels for 1s
                            // speed * frame_time * frames in a second
                            // 100 * 0.0167 * 60 = 100
                            closest.0.translation.truncate()
                                + closest.1.dir.normalize_or_zero() * 100.
                        });

                    if let Some(pos_prediction) = pos {
                        let direction = pos_prediction - t_trans.translation.truncate();

                        if direction.length_squared() < (t.range * t.range) {
                            ev_bomb.send(SpawnBombBulletEvent {
                                start_pos: t_trans.translation.truncate(),
                                target_dir: pos_prediction - t_trans.translation.truncate(),
                            });
                            t.can_shoot = false;
                        }
                    }
                }
            }
        }
    }
}

#[derive(Component)]
pub struct Bullet {
    dir: Vec2,
    timer: Timer,
}

impl Bullet {
    pub fn new(dir: Vec2) -> Self {
        Bullet {
            dir,
            timer: Timer::from_seconds(1.0, false),
        }
    }
}

struct SpawnBulletEvent {
    pos: Vec2,
    dir: Vec2,
}

fn spawn_bullet(
    mut commands: Commands,
    mut ev_spawn_bullet: EventReader<SpawnBulletEvent>,
    asset_server: Res<AssetServer>,
) {
    for ev in ev_spawn_bullet.iter() {
        //println!("Spawn a bullet. pos: {:?}, dir: {:?}", ev.pos, ev.dir);
        commands
            .spawn_bundle(SpriteBundle {
                texture: asset_server.load("sprites/Missile.png"),
                sprite: Sprite {
                    // Flip the logo to the left
                    flip_x: { ev.dir.x > 0.0 },
                    // And don't flip it upside-down ( the default )
                    flip_y: false,
                    ..default()
                },
                // sprite: Sprite {
                //     color: PURPLE,
                //     custom_size: Some(Vec2::new(6.0, 6.0)),
                //     ..default()
                // },
                transform: Transform {
                    translation: Vec3 {
                        x: ev.pos.x,
                        y: ev.pos.y,
                        z: 0.5,
                    },
                    ..default()
                },
                ..default()
            })
            .insert(Bullet::new(ev.dir.normalize_or_zero()));
    }
}

fn tick_bullet(
    mut commands: Commands,
    mut q_bullet: Query<(Entity, &mut Bullet)>,
    time: Res<Time>,
) {
    for (ent, mut b) in q_bullet.iter_mut() {
        if b.timer.tick(time.delta()).just_finished() {
            commands.entity(ent).despawn_recursive();
        }
    }
}

fn move_bullet(mut q_bullet: Query<(&mut Transform, &Bullet)>, time: Res<Time>) {
    for (mut trans, b) in q_bullet.iter_mut() {
        trans.translation += b.dir.extend(0.0) * time.delta_seconds() * 400.0;
    }
}

pub fn bullet_hit(
    mut commands: Commands,
    q_bullet: Query<(Entity, &Transform), With<Bullet>>,
    q_enemies: Query<(Entity, &Transform), (Without<Bullet>, Without<Dead>, With<Enemy>)>,
) {
    for (b_ent, b_trans) in q_bullet.iter() {
        for (e_ent, e_trans) in q_enemies.iter() {
            if collide(
                b_trans.translation,
                Vec2::new(6., 6.),
                e_trans.translation,
                Vec2::new(15., 15.),
            )
            .is_some()
            {
                //println!("Blam!");
                // Todo drop gold
                commands.entity(e_ent).insert(Dead);

                commands.entity(b_ent).despawn_recursive();

                // println!("Grabbed a gold");
                // enemy.has_gold = true;
                // commands.entity(ent).remove::<Gold>();

                // commands.entity(e_ent).add_child(ent);
                // gold_trans.translation = Vec3::new(0.0, 0.0, 0.1);
                break;
            }
        }
    }
}

#[derive(Component)]
struct BombBullet {
    start_pos: Vec3,
    start_dir: Vec2,
    end_dir: Vec2,
    //offset_dir: Vec2,
    timer: Timer,
}

struct SpawnBombBulletEvent {
    start_pos: Vec2,
    target_dir: Vec2,
}

#[derive(Component)]
struct BombTower;

#[derive(Component)]
struct BombExplosion {
    danger_timer: Timer,
    lifetime_timer: Timer,
}

impl BombExplosion {
    fn new() -> Self {
        BombExplosion {
            danger_timer: Timer::from_seconds(0.15, false),
            lifetime_timer: Timer::from_seconds(0.3, false),
        }
    }
}

// struct ArcInfo {
//     // Before spawn
//     arc_scale: f32,
//     arc_lerp_percent: f32,
//     // Tick()
//     start_lerp_type: EaseType,
//     end_lerp_type: EaseType,
//     combo_lerp_type: EaseType,
// }

// enum EaseType {
//     Linear,
//     InCubic,
//     OutCubic
// }

// version 5
// straight dir to target
// offset vector
// perp to the dir on the up side
// pos = dir + offset
// offset grows, reaches its peak at 0.4 to 0.5, then decays to 0
// this makes the arc consistent no matter the angle. It looks the same, just rotated
// what ease works like that?
// sin(0) -> sin(PI/2) -> sin(PI) = 0 -> 1 -> 0
// how do I make the arc front-loaded?
// if x < 0.5, outcubic(2x), else sin(x). At 0.5, they both = 1.0
// how to curve the right way
// take the largest y
// (x, y) perp -> (-y, x)
// (3,5)   -> (-5, 3)
// (-3,5)  -> (-5, -3) -> (5, 3)
// (3,-5)  -> (5, 3)
// (-3,-5) -> (5, -3) -> (-5, 3)
// if y < 0, -perp
// happens when x < 0

fn spawn_bomb_bullet(
    mut commands: Commands,
    mut ev_spawn_bomb: EventReader<SpawnBombBulletEvent>,
    asset_server: Res<AssetServer>,
) {
    for ev in ev_spawn_bomb.iter() {
        // make an arc
        let dir = ev.target_dir;
        let mag = dir.length();
        // *5.0 scales the length of the arc
        // 0.7 is the angle. 0.5 would be split in half.
        // bigger numbers make the arc bigger
        let start_dir = dir.lerp(Vec2::Y * mag * 5.0, 0.7);
        let end_dir = dir - start_dir;

        // Levers
        // that I can pull to edit this: defaults
        // scale: 5.0
        // lerp percent: 0.7
        // tick() lerp function: linear
        // t*t probably works if scale is lower than 5.0
        // tick() lifetime: 1.0s
        //
        // scale: 1.0
        // start_lerp (out_cubic)

        // returns the perp vec
        // gives the version that points up
        // let perp_up = if dir.x > 0.0 {
        //     dir.perp()
        // } else {
        //     -dir.perp()
        // };

        // println!(
        //     "dir: {:?}, blend: {:?}, start_dir: {:?}, end_dir: {:?}",
        //     dir, blend, start_dir, end_dir
        // );

        let start_pos = Vec3 {
            x: ev.start_pos.x,
            y: ev.start_pos.y,
            z: 0.6,
        };

        commands
            .spawn_bundle(SpriteBundle {
                texture: asset_server.load("sprites/Gold1.png"),
                sprite: Sprite {
                    // Flip the logo to the left
                    flip_x: { dir.x > 0.0 },
                    // And don't flip it upside-down ( the default )
                    flip_y: false,
                    ..default()
                },

                transform: Transform {
                    translation: start_pos,
                    ..default()
                },
                ..default()
            })
            .insert(BombBullet {
                start_pos,
                start_dir,
                end_dir,
                //offset_dir: perp_up.normalize() * 100.0,
                timer: Timer::from_seconds(1.0, false),
            });
    }
}

fn tick_bomb_bullet(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut q_bombs: Query<(Entity, &mut Transform, &mut BombBullet)>,
    time: Res<Time>,
) {
    for (ent, mut trans, mut bomb) in q_bombs.iter_mut() {
        if bomb.timer.tick(time.delta()).just_finished() {
            // blow up
            commands
                .spawn_bundle(MaterialMesh2dBundle {
                    mesh: meshes.add(shape::Circle::new(30.0).into()).into(),
                    material: materials.add(ColorMaterial::from(ORANGE)),
                    transform: Transform::from_translation(trans.translation),
                    ..default()
                })
                .insert(Collider::ball(30.0))
                .insert(Sensor)
                .insert(BombExplosion::new());
            commands.entity(ent).despawn_recursive();
        }

        let t = bomb.timer.percent();
        // grows early and then levels out
        // opposite of x*x*x
        //let out_cubic = 1.0 - (1.0 - t).powi(3);

        let start_lerp = Vec2::lerp(Vec2::ZERO, bomb.start_dir, t);
        let end_lerp = Vec2::lerp(Vec2::ZERO, bomb.end_dir, t);

        // arc version 5
        // let lerp_offset = if t < 0.5 {
        //     let x = 1.0 - (1.0 - 2.0 * t).powi(3);
        //     Vec2::lerp(Vec2::ZERO, bomb.offset_dir, x)
        // } else {
        //     let x = f32::sin(t * 3.14);
        //     Vec2::lerp(Vec2::ZERO, bomb.offset_dir, x)
        // };
        // doesn't really look better.
        // esp weird when shooting straight up or down.
        //let pos = start_lerp + lerp_offset;

        //let t = t * t;
        // 1 - Math.pow(1 - x, 3);
        // x ^ y
        //let t = 1.0 - (1.0 - t).powi(3);
        let pos = Vec2::lerp(start_lerp, start_lerp + end_lerp, t);

        trans.translation = bomb.start_pos + pos.extend(0.0);
    }
}

fn tick_bomb_explosion(
    mut commands: Commands,
    rapier_context: Res<RapierContext>,
    mut q_bombs: Query<(Entity, &mut BombExplosion)>,
    q_enemies: Query<Entity, With<Enemy>>,
    time: Res<Time>,
) {
    for (bomb_ent, mut bomb) in q_bombs.iter_mut() {
        for enemy_ent in q_enemies.iter() {
            if rapier_context.intersection_pair(bomb_ent, enemy_ent) == Some(true) {
                commands.entity(enemy_ent).insert(Dead);
            }
        }
        // remove the art after 3s
        if bomb.lifetime_timer.tick(time.delta()).just_finished() {
            commands.entity(bomb_ent).despawn_recursive();
        }
        // remove the danger after 1.5s
        if bomb.danger_timer.tick(time.delta()).just_finished() {
            commands.entity(bomb_ent).remove::<Collider>();
        }
    }
}

fn bomb_test(
    input: Res<Input<KeyCode>>,
    mouse: Res<MouseWorldPos>,
    mut ev_bomb: EventWriter<SpawnBombBulletEvent>,
) {
    if input.just_pressed(KeyCode::Space) {
        ev_bomb.send(SpawnBombBulletEvent {
            start_pos: Vec2::ZERO,
            target_dir: mouse.0,
        })
    }
}
