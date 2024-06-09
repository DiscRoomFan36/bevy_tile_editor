mod tile_grid;
mod icon_server;

use tile_grid::*;
use icon_server::*;

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use raylib::prelude::*;
use raylib::consts::{KeyboardKey, MouseButton};

const SQUARE_SIZE       : f32 = 64.0;
const SQUARE_SPACING    : f32 = 10.0;
const HIGHLIGHT_PADDING : f32 = SQUARE_SPACING / 2.0;

const TEXT_SIZE: i32 = 30;
const TEXT_PADDING: i32 = 10;

const SELECT_FOLDER_TEXT: &str = "Select Folder";

const HIGHLIGHT_COLOR       : Color = Color::ORANGE;
const PALLET_SELECTED_COLOR : Color = Color::RED;
const PALLET_DEFAULT_COLOR  : Color = Color::BLUE;

// TODO: Remove hardcode
const PATH: &str = "./assets/icons";

struct ImageContainer {
    image: Image,
    texture: Option<Texture2D>,
}

struct FileDialogContext {
    is_open: bool,
    
    current_path: PathBuf,
    
    width: i32,

    menu_position: Vector2,
    is_dragging: bool,
    menu_started_dragging_position: Vector2,
}

fn main() {
    let assets = get_images_from_path(Path::new(PATH));

    let mut icon_server = MyIconServer::new(assets);

    let mut grid = TileGrid::new(4, 6);

    // TODO: be smarter with this
    let start_pos = Vector2::new(100.0, 100.0);

    let (mut rl, thread) = raylib::init()
        .size(800, 600)
        .title("Tile Editor")
        .build();
    
    rl.set_target_fps(60);
    
    let mut file_dialog_context = FileDialogContext {
        is_open: false,
        current_path: ".".into(),
        width: rl.measure_text(SELECT_FOLDER_TEXT, TEXT_SIZE) + TEXT_PADDING * 2,
        menu_position: Vector2::new(20.0, 20.0),
        is_dragging: false,
        menu_started_dragging_position: Vector2::zero(),
    };

    let mut textures_dirty = true;
    // let dirty = true; // TODO: refactor for this

    /* -------------------- EVENT LOOP -------------------- */
    while !rl.window_should_close() {

        /* -------------------- KEY EVENT HANDLERS -------------------- */
    
        { // Quick (Save / Load) Handler
            const QUICK_SAVE_FILE: &str = "quick-save.json";
            if rl.is_key_pressed(KeyboardKey::KEY_P) {
                println!("Saving Grid!"); // TODO: draw something to the screen

                let json_string = grid.to_json().to_string();
                let mut output = fs::File::create(QUICK_SAVE_FILE).expect("File was created");
                write!(output, "{}", json_string).expect("Write to file");
            }
            if rl.is_key_pressed(KeyboardKey::KEY_L) {
                println!("Loading Saved Grid!"); // TODO: draw something to the screen

                if let Ok(mut input) = fs::File::open(QUICK_SAVE_FILE) {
                    let mut buffer = String::new();
                    input.read_to_string(&mut buffer).expect("Read to buffer");
        
                    grid = TileGrid::from_json(&json::parse(&buffer).unwrap()).expect("Grid loaded");
                } else {
                    println!("No quick save file");
                };
            }
        }

        { // Grid Resizing
            if rl.is_key_pressed(KeyboardKey::KEY_S) {                    grid.resize(grid.rows + 1, grid.cols    )  }
            if rl.is_key_pressed(KeyboardKey::KEY_W) { if grid.rows > 1 { grid.resize(grid.rows - 1, grid.cols    ) }}
            if rl.is_key_pressed(KeyboardKey::KEY_D) {                    grid.resize(grid.rows    , grid.cols + 1)  }
            if rl.is_key_pressed(KeyboardKey::KEY_A) { if grid.cols > 1 { grid.resize(grid.rows    , grid.cols - 1) }}
        }

        { // Selection cycling
            if rl.is_key_pressed(KeyboardKey::KEY_E) { icon_server.cycle_selected( 1) }
            if rl.is_key_pressed(KeyboardKey::KEY_Q) { icon_server.cycle_selected(-1) }
            if rl.is_key_pressed(KeyboardKey::KEY_X) { icon_server.cycle_default ( 1) }
            if rl.is_key_pressed(KeyboardKey::KEY_Z) { icon_server.cycle_default (-1) }
        }

        // File dialog
        if rl.is_key_pressed(KeyboardKey::KEY_O) { file_dialog_context.is_open = !file_dialog_context.is_open; }

        let mouse_pos = rl.get_mouse_position();


        /* -------------------- LOAD TEXTURES -------------------- */
        if textures_dirty {
            for (_, image_container) in icon_server.assets.iter_mut() {
                let mut image = image_container.image.clone();
                image.resize(SQUARE_SIZE as i32, SQUARE_SIZE as i32);
                
                let texture = rl.load_texture_from_image(&thread, &image).expect("load texture");
                image_container.texture = Some(texture)
            }

            textures_dirty = false;
        }
        
        /* -------------------- DRAWING -------------------- */
        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::LIGHTGRAY);


        /* -------------------- DRAW PALLET -------------------- */
        const PER_ROW: usize = 3;
        for i in 0..icon_server.assets.len() {
            let name = icon_server.assets[i].0.clone();
            let (x, y) = index_to_pos(i, (999, PER_ROW));

            let rec = new_square(Vector2::new(10.0, 10.0), (x, y));

            /* -------------------- ON HOVER PALLET -------------------- */
            if rec.check_collision_point_rec(mouse_pos) {
                // draw some highlighting around the hovered rectangle
                d.draw_rectangle_rec(pad_rectangle(rec, HIGHLIGHT_PADDING), HIGHLIGHT_COLOR);

                if d.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
                    icon_server.set_selected_by_name(&name);
                }
                if d.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_RIGHT) {
                    icon_server.set_default_by_name(&name);
                }
            }

            // Default and selected highlighting
            let is_default = icon_server.get_default_name() == name;
            if is_default {
                d.draw_rectangle_rec(pad_rectangle(rec, HIGHLIGHT_PADDING), PALLET_DEFAULT_COLOR);
            }

            if icon_server.get_selected_name() == name {
                let mut rec = pad_rectangle(rec, HIGHLIGHT_PADDING);
                if is_default { rec.width /= 2.0; }
                d.draw_rectangle_rec(rec, PALLET_SELECTED_COLOR);
            }

            let (_, image_container) = &icon_server.assets[i];

            d.draw_texture(
                image_container.texture.as_ref().unwrap(),
                rec.x as i32, rec.y as i32, Color::WHITE
            );
        }

        /* -------------------- DRAW GRID -------------------- */
        for i in 0..grid.rows*grid.cols {
            let (x, y) = index_to_pos(i, grid.size());

            let rec = new_square(start_pos, (x, y));
            
            /* -------------------- ON HOVER GRID -------------------- */
            if rec.check_collision_point_rec(mouse_pos) {
                // draw some highlighting around the hovered rectangle
                d.draw_rectangle_rec(pad_rectangle(rec, HIGHLIGHT_PADDING), HIGHLIGHT_COLOR);
    
                if d.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
                    grid.set((x, y), Some(icon_server.get_selected_name().to_string()));
                }
                if d.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_RIGHT) {
                    grid.set((x, y), None);
                }
            }
    
            let image_container = if let Some(name) = grid.get((x, y)) {
                icon_server.get_by_name(name).expect("Name exist in icon server")
            } else {
                icon_server.get_default_handle()
            };
    
            d.draw_texture(
                image_container.texture.as_ref().unwrap(),
                rec.x as i32, rec.y as i32, Color::WHITE
            );
        }
    
        // TODO: Make sure things below this cannot be touched
        /* -------------------- FILE DIALOG -------------------- */
        if file_dialog_context.is_open {
            let mut xx = file_dialog_context.menu_position.x as i32;
            let mut yy = file_dialog_context.menu_position.y as i32;

            let mut file_names = list_directory(&file_dialog_context.current_path);

            // TODO: Move up, near input section, consolidate
            { /* -------------------- FILE DIALOG --- HANDLE MOUSE EVENTS -------------------- */
                // Handle dragging
                let handle_rec = Rectangle {
                    x: xx as f32, y: yy as f32,
                    width: file_dialog_context.width as f32,
                    height: TEXT_SIZE as f32,
                };
                if handle_rec.check_collision_point_rec(mouse_pos) && d.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
                    file_dialog_context.is_dragging = true;
                    file_dialog_context.menu_started_dragging_position = mouse_pos;
                }

                if file_dialog_context.is_dragging {
                    let diff = mouse_pos - file_dialog_context.menu_started_dragging_position;
                    xx += diff.x as i32;
                    yy += diff.y as i32;

                    if d.is_mouse_button_released(MouseButton::MOUSE_BUTTON_LEFT) {
                        file_dialog_context.menu_position += diff;
                        file_dialog_context.is_dragging = false;
                    }
                }

                let mut xx = xx;
                let mut yy = yy;

                yy += TEXT_SIZE; // for current folder text
                
                xx += TEXT_PADDING; // indent for text
                yy += TEXT_PADDING; // indent for backing padding

                let mut refile = false;
                for file in &file_names {

                    let rec = Rectangle {
                        x: xx as f32, y: yy as f32,
                        width: (file_dialog_context.width - TEXT_PADDING * 2) as f32,
                        height: TEXT_SIZE as f32
                    };

                    if rec.check_collision_point_rec(mouse_pos) && d.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
                        assert!(!refile);
                        refile = true;
                        
                        let p = file_dialog_context.current_path.join(&file);

                        assert!(p.exists());
                        assert!(p.is_dir(), "Can only click on directories"); // TODO

                        // TODO: Clean up path at some point, it gets dirty really fast, collecting a lot of /src/../src
                        file_dialog_context.current_path.push(&file);
                    }
                    yy += TEXT_SIZE;
                }
                yy += TEXT_PADDING;
                
                if refile { file_names = list_directory(&file_dialog_context.current_path); }

                // check if we need to make the width bigger
                let width = file_names
                    .iter()
                    .chain([file_dialog_context.current_path.to_str().unwrap().to_string()].iter())
                    .map(|file| d.measure_text(&file, TEXT_SIZE))
                    .max()
                    .unwrap_or_default()
                    + TEXT_PADDING * 2;
                if width > file_dialog_context.width { file_dialog_context.width = width; }

                // Handle Select Folder Button
                let select_folder_rec = Rectangle {
                    x: xx as f32,
                    y: (yy + TEXT_PADDING) as f32,
                    width: (file_dialog_context.width - TEXT_PADDING * 2) as f32,
                    height: TEXT_SIZE as f32
                };
                // TODO: Factor out that d.is_mouse_button_pressed, to some struct that holds all the button states
                if select_folder_rec.check_collision_point_rec(mouse_pos) && d.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
                    icon_server.load_images(&mut get_images_from_path(&file_dialog_context.current_path));
                    textures_dirty = true; // Remember to call when

                    file_dialog_context.is_open = false; // Close it because we done here
                }
            }

            { // Draw current folder
                const FILE_DIALOG_CURRENT_FOLDER_COLOR: Color = Color::MAROON;
                const FILE_DIALOG_CURRENT_FOLDER_TEXT_COLOR: Color = Color::GOLDENROD;

                d.draw_rectangle(xx, yy, file_dialog_context.width, TEXT_SIZE, FILE_DIALOG_CURRENT_FOLDER_COLOR);
                d.draw_text(file_dialog_context.current_path.to_str().unwrap(), xx + TEXT_PADDING, yy, TEXT_SIZE, FILE_DIALOG_CURRENT_FOLDER_TEXT_COLOR);
                yy += TEXT_SIZE;
            }

            { // Draw Backing Box
                const FILE_DIALOG_BACKING_BOX_COLOR: Color = Color::DARKGRAY;

                let total_file_names_height = file_names.len() as i32 * TEXT_SIZE + TEXT_PADDING * 2;
                d.draw_rectangle(xx, yy, file_dialog_context.width, total_file_names_height, FILE_DIALOG_BACKING_BOX_COLOR);
            }

            { /* -------------------- DRAW LABELS -------------------- */
                const FILE_DIALOG_LABEL_HOVER_COLOR: Color = Color::ORANGE;
                const FILE_DIALOG_LABEL_TEXT_COLOR: Color = Color::GOLD;

                let mut xx = xx;

                xx += TEXT_PADDING;
                yy += TEXT_PADDING;
                for file in file_names {
                    let rec = Rectangle {
                        x: xx as f32,
                        y: yy as f32,
                        width: (file_dialog_context.width - TEXT_PADDING * 2) as f32,
                        height: TEXT_SIZE as f32
                    };

                    if rec.check_collision_point_rec(mouse_pos) {
                        let padded = pad_rectangle_ex(rec, (TEXT_PADDING / 2) as f32, (TEXT_PADDING / 2) as f32, 0.0, 0.0);
                        d.draw_rectangle_rec(padded, FILE_DIALOG_LABEL_HOVER_COLOR);
                    }

                    d.draw_text(&file, xx, yy, TEXT_SIZE, FILE_DIALOG_LABEL_TEXT_COLOR);
                    yy += TEXT_SIZE;
                }

                yy += TEXT_PADDING;
            }

            { // Select folder button
                let mut xx = xx;
                
                d.draw_rectangle(xx, yy, file_dialog_context.width, TEXT_SIZE + TEXT_PADDING * 2, Color::GREEN);

                xx += TEXT_PADDING;
                yy += TEXT_PADDING;

                let rec = Rectangle {
                    x: xx as f32,
                    y: yy as f32,
                    width: (file_dialog_context.width - TEXT_PADDING * 2) as f32,
                    height: TEXT_SIZE as f32
                };

                if rec.check_collision_point_rec(mouse_pos) {
                    let padded = pad_rectangle_ex(rec, (TEXT_PADDING / 2) as f32, (TEXT_PADDING / 2) as f32, 0.0, 0.0);
                    d.draw_rectangle_rec(padded, Color::WHEAT);
                }

                // TODO: Center this
                d.draw_text(SELECT_FOLDER_TEXT, xx, yy, TEXT_SIZE, Color::BLACK);

                // yy += TEXT_SIZE + TEXT_PADDING;
            }
        }
    }
}

fn new_square(start_pos: Vector2, pos: (usize, usize)) -> Rectangle {
    let (x, y) = pos;
    Rectangle {
        x: start_pos.x + x as f32 * (SQUARE_SIZE + SQUARE_SPACING),
        y: start_pos.y + y as f32 * (SQUARE_SIZE + SQUARE_SPACING),
        width: SQUARE_SIZE, height: SQUARE_SIZE,
    }
}

fn pad_rectangle_ex(rec: Rectangle, left: f32, right: f32, top: f32, bottom: f32) -> Rectangle {
    Rectangle {
        x:      rec.x      - left,
        y:      rec.y      - top,
        width:  rec.width  + right * 2.0,
        height: rec.height + bottom * 2.0,
    }
}
fn pad_rectangle(rec: Rectangle, padding: f32) -> Rectangle {
    pad_rectangle_ex(rec, padding, padding, padding, padding)
}

impl ToAndFromJsonValue for String {
    fn to_json(&self) -> json::JsonValue { json::from(self.to_owned()) }
    fn from_json(json: &json::JsonValue) -> Option<Self> {
        json.as_str().and_then(|str| Some(str.to_owned()))
    }
}

fn get_images_from_path(path: &Path) -> Vec<(String, ImageContainer)> {
    let paths = fs::read_dir(path).expect("Valid directory");

    let names: Vec<_> = paths
        .map(|path| path.unwrap())
        .map(|path|
            path.path().to_str().expect("Valid path").to_string()
        )
        .collect();
    
    let images: Vec<_> = names
        .iter()
        .map(|path|
            raylib::texture::Image::load_image(&path).expect("Is Valid Image")
        )
        .collect();

    names.into_iter()
        .zip(images)
        .map(|(s, image)|
            (s, ImageContainer { image, texture: None }
        ))
        .collect()
}

fn list_directory(path: &Path) -> Vec<String> {
    let dir = fs::read_dir(path).expect("Path is valid");
    let mut file_names: Vec<_> = dir
        .map(|path| {
            path.unwrap()
                .file_name()
                .into_string()
                .unwrap()
        })
        .filter(|name| !name.starts_with(".")) // filter out hidden files
        .filter(|name| {
            if path.join(name).is_dir() { return true; }
            // filter out things that aren't drawable (aka png's)
            name.split_once(".")
                .map(|(_, end)| end == "png")
                .unwrap_or_default()
        })
        .collect();

    file_names.reverse();

    file_names.insert(0, "..".into());
    // file_names.insert(0, ".".into()); // TODO: Remove this?

    // TODO: sort by type then name

    file_names
}
