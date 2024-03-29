use bevy::{prelude::*, sprite::Anchor, utils::HashMap};

use crate::{tiles::spawn_floor, GameState, Position, TILE_SIZE};

pub struct EditPlugin;

#[derive(Resource, Default)]
struct EditingState {
    floors: HashMap<Position, Entity>,
    walls: HashMap<Position, Entity>,
    blocks: HashMap<Position, Entity>,
    goals: HashMap<Position, Entity>,
    player: Option<(Position, Entity)>,
}

impl EditingState {
    fn can_place(&self, position: &Position) -> bool {
        self.floors.contains_key(position)
            && !self.blocks.contains_key(position)
            && !self.goals.contains_key(position)
            && (self.player.is_none() || &self.player.unwrap().0 != position)
    }

    fn remove_object(&mut self, position: &Position) -> Option<Entity> {
        if self.blocks.contains_key(position) {
            return self.blocks.remove(position);
        } else if self.goals.contains_key(position) {
            return self.goals.remove(position);
        } else if self.player.is_some() && self.player.unwrap().0 == *position {
            let player_id = self.player.unwrap().1;
            self.player = None;
            return Some(player_id);
        } else {
            return None;
        }
    }

    fn serialize(&self) -> Vec<Vec<i32>> {
        let wall_positions = self.walls.keys();
        let min_x = wall_positions.clone().map(|p| p.x).min().unwrap();
        let max_x = wall_positions.clone().map(|p| p.x).max().unwrap();
        let min_y = wall_positions.clone().map(|p| p.y).min().unwrap();
        let max_y = wall_positions.clone().map(|p| p.y).max().unwrap();

        let mut level = vec![
            vec![0; (1 + max_x - min_x).try_into().unwrap()];
            (1 + max_y - min_y).try_into().unwrap()
        ];

        for wall_position in wall_positions {
            level[(wall_position.y - min_y) as usize][(wall_position.x - min_x) as usize] = 8;
        }

        for goal_position in self.goals.keys() {
            level[(goal_position.y - min_y) as usize][(goal_position.x - min_x) as usize] = 4;
        }

        for block_position in self.blocks.keys() {
            level[(block_position.y - min_y) as usize][(block_position.x - min_x) as usize] = 2;
        }

        if let Some((player_position, _)) = self.player {
            level[(player_position.y - min_y) as usize][(player_position.x - min_x) as usize] = 1;
        }

        level
    }
}

#[derive(Component)]
struct Cursor {
    action_timer: Timer,
}

fn remove_level(mut commands: Commands, almost_everything_query: Query<Entity, Without<Window>>) {
    for entity in almost_everything_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn show_cursor(mut commands: Commands, asset_server: Res<AssetServer>) {
    let camera_position = Vec3::new(TILE_SIZE / 2.0, -(TILE_SIZE) / 2.0, 1000.0);
    commands.spawn(Camera2dBundle {
        transform: Transform {
            translation: camera_position,
            scale: Vec3::new(0.5, 0.5, 1.0),
            ..default()
        },
        ..default()
    });

    commands.spawn((
        Cursor {
            action_timer: Timer::from_seconds(0.2, TimerMode::Once),
        },
        SpriteBundle {
            sprite: Sprite {
                anchor: Anchor::TopLeft,
                ..default()
            },
            texture: asset_server.load("cursor.png"),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
            ..default()
        },
    ));

    commands.insert_resource(EditingState::default());
}

fn handle_edit_input(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut editing_state: ResMut<EditingState>,
    mut cursor_query: Query<(&mut Cursor, &mut Transform)>,
) {
    let Some((mut cursor, mut transform)) = cursor_query.iter_mut().next() else {
        return;
    };

    if keyboard_input.pressed(KeyCode::E) {
        dbg!(editing_state.serialize());
    }

    if !cursor.action_timer.finished() {
        cursor.action_timer.tick(time.delta());
        return;
    }

    let mut movement: Option<(i32, i32)> = None;
    if keyboard_input.pressed(KeyCode::Up) {
        movement = Some((0, -1));
    } else if keyboard_input.pressed(KeyCode::Down) {
        movement = Some((0, 1));
    } else if keyboard_input.pressed(KeyCode::Left) {
        movement = Some((-1, 0));
    } else if keyboard_input.pressed(KeyCode::Right) {
        movement = Some((1, 0));
    }

    let mut cursor_position = Position::from_translation(transform.translation);

    if let Some((move_x, move_y)) = movement {
        cursor.action_timer.reset();

        cursor_position = cursor_position.add(move_x, move_y);
        transform.translation = cursor_position.to_translation_z(2.0);
    }

    if keyboard_input.pressed(KeyCode::Z) && !editing_state.floors.contains_key(&cursor_position) {
        cursor.action_timer.reset();

        let floor_entity = commands
            .spawn(spawn_floor(&asset_server, cursor_position))
            .id();

        editing_state.floors.insert(cursor_position, floor_entity);

        if let Some(wall_entity) = editing_state.walls.get(&cursor_position) {
            commands.entity(*wall_entity).despawn();
            editing_state.walls.remove(&cursor_position);
        }

        let wall_combinations = vec![
            (-1, -1),
            (-1, 0),
            (-1, 1),
            (0, -1),
            (0, 1),
            (1, -1),
            (1, 0),
            (1, 1),
        ];
        for (relative_x, relative_y) in wall_combinations {
            let wall_position = cursor_position.add(relative_x, relative_y);

            if !editing_state.floors.contains_key(&wall_position)
                && !editing_state.walls.contains_key(&wall_position)
            {
                let wall_id = commands
                    .spawn(SpriteBundle {
                        sprite: Sprite {
                            anchor: Anchor::TopLeft,
                            ..default()
                        },
                        texture: asset_server.load("wall.png"),
                        transform: Transform::from_translation(wall_position.to_translation()),
                        ..default()
                    })
                    .id();
                editing_state.walls.insert(wall_position, wall_id);
            }
        }
    } else if keyboard_input.pressed(KeyCode::X) && editing_state.can_place(&cursor_position) {
        cursor.action_timer.reset();

        let block_translation = cursor_position.to_translation();

        let block_id = commands
            .spawn(SpriteBundle {
                sprite: Sprite {
                    anchor: Anchor::TopLeft,
                    ..default()
                },
                texture: asset_server.load("block.png"),
                transform: Transform::from_translation(block_translation),
                ..default()
            })
            .id();
        editing_state.blocks.insert(cursor_position, block_id);
    } else if keyboard_input.pressed(KeyCode::C) && editing_state.can_place(&cursor_position) {
        cursor.action_timer.reset();

        let goal_translation = cursor_position.to_translation_z(0.5);

        let goal_id = commands
            .spawn(SpriteBundle {
                sprite: Sprite {
                    anchor: Anchor::TopLeft,
                    ..default()
                },
                texture: asset_server.load("goal.png"),
                transform: Transform::from_translation(goal_translation),
                ..default()
            })
            .id();
        editing_state.goals.insert(cursor_position, goal_id);
    } else if keyboard_input.pressed(KeyCode::V) && editing_state.can_place(&cursor_position) {
        cursor.action_timer.reset();

        let player_translation = cursor_position.to_translation();

        let player_id = commands
            .spawn(SpriteBundle {
                sprite: Sprite {
                    anchor: Anchor::TopLeft,
                    ..default()
                },
                texture: asset_server.load("player.png"),
                transform: Transform::from_translation(player_translation),
                ..default()
            })
            .id();

        if editing_state.player.is_some() {
            commands.entity(editing_state.player.unwrap().1).despawn();
        }
        editing_state.player = Some((cursor_position, player_id));
    } else if keyboard_input.pressed(KeyCode::S) {
        let Some(removed_entity) = editing_state.remove_object(&cursor_position) else {
            return;
        };

        commands.entity(removed_entity).despawn();
    }
}

impl Plugin for EditPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Editing), (remove_level, show_cursor))
            .add_systems(
                Update,
                handle_edit_input.run_if(in_state(GameState::Editing)),
            );
    }
}
