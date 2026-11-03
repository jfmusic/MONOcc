//! the macroquad render loop and simulator entry point.
//!
//! this is the only part of the crate that needs a gpu / window, so it is not
//! unit tested (the placement math in [`crate::geometry`] and the key mapping in
//! [`crate::input`] are). it builds a controller over the fake device, then each
//! frame applies keyboard input and draws the layout from a render snapshot.

use macroquad::prelude::*;
use std::collections::HashMap;

use sextant_core::component::kind;
use sextant_core::{image, resources, Configuration, Controller, Device, RenderComponent};
use sextant_fake::FakeDevice;

use crate::geometry::{
    self, Circle, ComponentGeometry, Connector, Edge, Rect, COMPONENT_COLOR,
    COMPONENT_SELECTED_COLOR, DISPLAY_SIZE,
};
use crate::input::{key_action, SimAction};

const INJECT_AMOUNT: i64 = 8;
// magnetic edge connectors render as small metallic dots; the base / core top
// edge is instead a usb port (a dark recessed slot). the attachment point (the
// parent-facing connector) is marked in a distinct color on every module.
const CONNECTOR_COLOR: Color = Color::new(0.78, 0.66, 0.30, 1.0);
const USB_COLOR: Color = Color::new(0.12, 0.12, 0.14, 1.0);
const ATTACHMENT_COLOR: Color = Color::new(1.0, 0.0, 0.0, 1.0);

fn window_conf() -> Conf {
    Conf {
        window_title: "sextant".to_owned(),
        window_width: DISPLAY_SIZE.0,
        window_height: DISPLAY_SIZE.1,
        window_resizable: true,
        ..Default::default()
    }
}

/// a per-frame hook that drives the gui's controller (e.g. the demoscene
/// animation) while the gui renders it, generic over the device backend.
pub type DeviceDriver<D> = Box<dyn FnMut(&Controller<D>)>;
/// the simulator's per-frame hook (the fake backend).
pub type FrameDriver = DeviceDriver<FakeDevice>;

/// launch the simulator window driving `device_layout` on the fake device. an
/// optional `driver` is invoked each frame, so the gui can render a running demo.
pub fn run(
    config: Configuration,
    device_layout: serde_json::Value,
    echo: bool,
    debug: bool,
    driver: Option<FrameDriver>,
) {
    macroquad::Window::from_config(
        window_conf(),
        amain(config, device_layout, echo, debug, driver),
    );
}

/// launch the gui rendering a live, already-connected device (real hardware). the
/// device's own input updates the render snapshot each frame; an optional `driver`
/// (e.g. a demo) drives the controller each frame too, so `demo --display` runs on
/// real hardware. generic over the backend so the closure can drive real usb.
pub fn run_device<D: Device + 'static>(controller: Controller<D>, driver: Option<DeviceDriver<D>>) {
    macroquad::Window::from_config(window_conf(), amain_device(controller, driver));
}

async fn amain_device<D: Device>(
    mut controller: Controller<D>,
    mut driver: Option<DeviceDriver<D>>,
) {
    let mut selected: i64 = 0;
    let mut show_help = false;
    let mut textures: HashMap<u8, Texture2D> = HashMap::new();
    fit_window(&controller);
    loop {
        if handle_nav(&controller, &mut selected, &mut show_help) {
            break;
        }
        if let Some(driver) = driver.as_mut() {
            driver(&controller);
        }
        clear_background(BLACK);
        draw_layout(&controller, selected, &mut textures);
        draw_help(show_help);
        next_frame().await;
    }
    controller.close();
}

/// resize the window to the layout's cell grid so the components fill it instead
/// of leaving empty margin (the slot size is then uniform in both dimensions).
fn fit_window<D: Device>(controller: &Controller<D>) {
    let (min_x, min_y, max_x, max_y) = controller.layout_extrema();
    let cols = (max_x - min_x + 1).max(1);
    let rows = (max_y - min_y + 1).max(1);
    let cell = (900 / cols.max(rows)).clamp(48, 240);
    request_new_screen_size((cols * cell) as f32, (rows * cell) as f32);
}

/// the device-agnostic navigation keys (quit / rotate / select); input injection
/// is simulator-only, so the live-device gui ignores those actions. rotate and
/// select fire on key-up, so both press and release events are consulted.
fn handle_nav<D: Device>(controller: &Controller<D>, selected: &mut i64, help: &mut bool) -> bool {
    let connected = controller.connected_count() as i64;
    let events = get_keys_pressed()
        .into_iter()
        .map(|key| (key, true))
        .chain(get_keys_released().into_iter().map(|key| (key, false)));
    for (key, pressed) in events {
        match key_action(key, pressed) {
            Some(SimAction::Quit) => return true,
            Some(SimAction::ToggleHelp) => *help = !*help,
            Some(SimAction::Rotate) => {
                controller.rotate(90);
                fit_window(controller);
            }
            Some(SimAction::SelectPrev) => *selected = (*selected - 1).rem_euclid(connected + 1),
            Some(SimAction::SelectNext) => *selected = (*selected + 1).rem_euclid(connected + 1),
            Some(SimAction::Select(n)) => *selected = n,
            _ => {}
        }
    }
    false
}

async fn amain(
    config: Configuration,
    device_layout: serde_json::Value,
    echo: bool,
    debug: bool,
    mut driver: Option<FrameDriver>,
) {
    let device = FakeDevice::new("DEAD:BEEF");
    device.components_connected(device_layout);
    let mut controller = Controller::new(config, device);
    controller.set_echo(echo);
    controller.set_debug(debug);
    if controller.open().is_err() || controller.connect().is_err() {
        return;
    }
    controller.wait_for_layout();

    let mut selected: i64 = 0;
    let mut show_help = false;
    let mut textures: HashMap<u8, Texture2D> = HashMap::new();
    fit_window(&controller);

    loop {
        if handle_input(&controller, &mut selected, &mut show_help) {
            break;
        }
        if let Some(driver) = driver.as_mut() {
            driver(&controller);
        }
        clear_background(BLACK);
        draw_layout(&controller, selected, &mut textures);
        draw_help(show_help);
        next_frame().await;
    }
    controller.close();
}

/// apply queued key events; returns true when the user asked to quit.
fn handle_input(controller: &Controller<FakeDevice>, selected: &mut i64, help: &mut bool) -> bool {
    let connected = controller.connected_count() as i64;
    for key in get_keys_pressed() {
        if let Some(action) = key_action(key, true) {
            if apply(controller, selected, help, connected, action) {
                return true;
            }
        }
    }
    for key in get_keys_released() {
        if let Some(action) = key_action(key, false) {
            apply(controller, selected, help, connected, action);
        }
    }
    false
}

fn apply(
    controller: &Controller<FakeDevice>,
    selected: &mut i64,
    help: &mut bool,
    connected: i64,
    action: SimAction,
) -> bool {
    let device = controller.device();
    match action {
        SimAction::Quit => return true,
        SimAction::ToggleHelp => *help = !*help,
        SimAction::Rotate => {
            controller.rotate(90);
            fit_window(controller);
        }
        SimAction::SelectPrev => *selected = (*selected - 1).rem_euclid(connected + 1),
        SimAction::SelectNext => *selected = (*selected + 1).rem_euclid(connected + 1),
        SimAction::Select(n) => *selected = n,
        SimAction::SlideUp => device.input_slide_up(*selected, INJECT_AMOUNT),
        SimAction::SlideDown => device.input_slide_down(*selected, INJECT_AMOUNT),
        SimAction::RotateLeft => device.input_rotation_left(*selected, INJECT_AMOUNT),
        SimAction::RotateRight => device.input_rotation_right(*selected, INJECT_AMOUNT),
        SimAction::Press => device.input_pressed(*selected),
        SimAction::Release => device.input_released(*selected),
    }
    false
}

fn color_of((r, g, b): (u8, u8, u8)) -> Color {
    Color::from_rgba(r, g, b, 255)
}

/// the keybinding overlay rows; the injection keys apply only to the simulator.
const HELP_BINDINGS: &[(&str, &str)] = &[
    ("?", "toggle this help"),
    ("Tab", "rotate the layout (and core screen)"),
    ("PageUp / PageDown", "select previous / next component"),
    ("0 - 9", "select component by index"),
    ("Left / Right", "rotate selected dial (simulator)"),
    ("Up / Down", "slide selected (simulator)"),
    ("Space", "press / release selected (simulator)"),
    ("Q / Esc", "quit"),
];

/// draw the keybinding overlay when open, otherwise a small hint to open it.
fn draw_help(open: bool) {
    let (sw, sh) = (screen_width(), screen_height());
    if !open {
        draw_text(
            "? help",
            8.0,
            sh - 8.0,
            18.0,
            Color::new(1.0, 1.0, 1.0, 0.5),
        );
        return;
    }
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.7));
    let line_h = 26.0;
    let pad = 24.0;
    let panel_w = sw.min(540.0);
    let panel_h = pad * 2.0 + line_h * (HELP_BINDINGS.len() as f32 + 1.5);
    let (px, py) = ((sw - panel_w) / 2.0, (sh - panel_h) / 2.0);
    draw_rectangle(px, py, panel_w, panel_h, Color::new(0.09, 0.09, 0.11, 0.97));
    draw_rectangle_lines(
        px,
        py,
        panel_w,
        panel_h,
        2.0,
        color_of(COMPONENT_SELECTED_COLOR),
    );
    draw_text("keybindings", px + pad, py + pad + 14.0, 26.0, WHITE);
    let mut ty = py + pad + line_h * 2.0;
    for (key, desc) in HELP_BINDINGS {
        draw_text(key, px + pad, ty, 20.0, color_of(COMPONENT_SELECTED_COLOR));
        draw_text(desc, px + pad + 170.0, ty, 20.0, WHITE);
        ty += line_h;
    }
}

fn draw_layout<D: Device>(
    controller: &Controller<D>,
    selected: i64,
    textures: &mut HashMap<u8, Texture2D>,
) {
    let (min_x, min_y, max_x, max_y) = controller.layout_extrema();
    // a minimized or zero-size surface yields a degenerate slot size; skip the
    // frame rather than feeding negative dimensions into the draw calls.
    let surface = (
        screen_width().max(1.0) as i32,
        screen_height().max(1.0) as i32,
    );
    let (initial_pos, slot_size) =
        geometry::initial_pos_and_slot_size(surface, min_x, min_y, max_x, max_y);
    if slot_size < 1 {
        return;
    }
    let orientation = controller.layout_orientation();
    let placed: Vec<(RenderComponent, ComponentGeometry)> = controller
        .render_components()
        .into_iter()
        .map(|component| {
            let geometry = ComponentGeometry::new(
                initial_pos,
                slot_size,
                component.slot,
                component.orientation,
                component.footprint,
            );
            (component, geometry)
        })
        .collect();
    // monogram modules plug into the core, so mark each module's attachment
    // facing it (their hardcoded geometry carries no male-port slot).
    let core = placed
        .iter()
        .find(|(c, _)| c.kind == kind::CORE)
        .map(|(_, g)| g.body);
    for (component, geometry) in &placed {
        let attachment = attachment_point(component, geometry, core);
        draw_component(
            component,
            geometry,
            selected,
            slot_size,
            orientation,
            textures,
            attachment,
        );
    }
}

/// the body edge the base / core usb port faces. it is north when screen-upright
/// and turns clockwise with the layout orientation.
fn usb_port_edge(orientation: i32) -> Edge {
    match orientation.rem_euclid(360) {
        90 => Edge::East,
        180 => Edge::South,
        270 => Edge::West,
        _ => Edge::North,
    }
}

/// draw a module's per-cell connectors: metallic dots, except the base / core
/// usb edge which renders as a recessed dark slot.
fn draw_connectors(connectors: &[Connector], usb_edge: Option<Edge>) {
    for c in connectors {
        let (px, py, r) = (c.pos.x as f32, c.pos.y as f32, c.pos.r as f32);
        if usb_edge == Some(c.edge) {
            draw_rectangle(px - r * 1.4, py - r * 0.8, r * 2.8, r * 1.6, USB_COLOR);
        } else {
            draw_circle(px, py, r, CONNECTOR_COLOR);
        }
    }
}

/// mark a module's attachment point: an equilateral triangle inscribed in the
/// connector circle (a small margin inside it), apex pointing outward along `edge`
/// (`>` toward what the module plugs into).
fn draw_attachment(pos: Circle, edge: Edge) {
    // `pos.r` is the marker radius, ~10% inside the bronze connector (which is
    // 1.1x this), so vertices placed on it inset the triangle by that margin.
    let (cx, cy, r) = (pos.x as f32, pos.y as f32, pos.r as f32);
    let (dx, dy) = match edge {
        Edge::East => (1.0, 0.0),
        Edge::West => (-1.0, 0.0),
        Edge::South => (0.0, 1.0),
        Edge::North => (0.0, -1.0),
    };
    // vertices at 0 / +120 / -120 degrees around the connector from the apex.
    let vertex = |deg: f32| {
        let (s, c) = deg.to_radians().sin_cos();
        vec2(cx + r * (dx * c - dy * s), cy + r * (dx * s + dy * c))
    };
    draw_triangle(vertex(0.0), vertex(120.0), vertex(-120.0), ATTACHMENT_COLOR);
}

/// where to mark a module's attachment, snapped onto a real connector so it sits
/// centered on one. monogram modules plug into the core, so their marker sits on
/// the connector facing it; everything else uses the model's male (parent-facing)
/// port. picks the connector on that edge nearest the reference point.
fn attachment_point(
    component: &RenderComponent,
    geometry: &ComponentGeometry,
    core: Option<Rect>,
) -> (Circle, Edge) {
    let (edge, rx, ry) = match (component.kind, core) {
        (kind::ORBITER | kind::TRI_DIAL, Some(core)) => (
            geometry::edge_toward(geometry.body, core),
            core.x + core.w / 2,
            core.y + core.h / 2,
        ),
        _ => {
            let rp = geometry.root_port;
            (geometry::male_edge(component.orientation), rp.x, rp.y)
        }
    };
    let pos = geometry::connector_on_edge(
        &geometry.connectors(),
        edge,
        rx,
        ry,
        geometry.connector_size,
    )
    .unwrap_or(geometry.root_port);
    (pos, edge)
}

fn draw_component(
    component: &RenderComponent,
    geometry: &ComponentGeometry,
    selected: i64,
    slot_size: i32,
    orientation: i32,
    textures: &mut HashMap<u8, Texture2D>,
    attachment: (Circle, Edge),
) {
    let body_color = if component.index == selected {
        COMPONENT_SELECTED_COLOR
    } else {
        COMPONENT_COLOR
    };
    let (x, y, w, h) = (
        geometry.x as f32,
        geometry.y as f32,
        geometry.w as f32,
        geometry.h as f32,
    );
    draw_rectangle(x, y, w, h, color_of(body_color));
    draw_rectangle_lines(
        geometry.led.x as f32,
        geometry.led.y as f32,
        geometry.led.w as f32,
        geometry.led.h as f32,
        (geometry.led_thickness * 2) as f32,
        color_of(component.led),
    );
    // both families share the per-cell magnetic connector layout; the base and
    // core additionally expose a usb port on their top edge, and every module
    // marks its attachment (parent-facing) point.
    let usb_edge =
        matches!(component.kind, kind::CORE | kind::BASE).then(|| usb_port_edge(orientation));
    draw_connectors(&geometry.connectors(), usb_edge);
    draw_attachment(attachment.0, attachment.1);

    let font_size = (slot_size / 16).max(8) as f32;
    let inset = geometry.led_thickness as f32 + 8.0;
    let label = format!(
        "[{}] {} ({})",
        component.uid, component.index, component.kind_name
    );
    draw_text(&label, x + inset, y + inset + font_size, font_size, WHITE);
    let slot = format!("({}, {})", component.slot.0, component.slot.1);
    draw_text(&slot, x + inset, y + h - inset, font_size, WHITE);
    if let Some(behavior) = component.behavior {
        draw_text(
            behavior,
            x + inset,
            y + h - inset - font_size,
            font_size,
            WHITE,
        );
    }

    if component.kind == kind::BASE {
        draw_base_screen(component, x, y, w, h, slot_size, textures);
    } else if component.kind == kind::CORE {
        draw_core_screen(component, x, y, w, h, textures);
    } else {
        let text = format!("{:?}", component.values);
        // measure so the values stay centered at any font size (a fixed per-char
        // offset drifts as the window -- and thus the font -- grows).
        let dims = measure_text(&text, None, font_size as u16, 1.0);
        draw_text(
            &text,
            x + (w - dims.width) / 2.0,
            y + h / 2.0,
            font_size,
            WHITE,
        );
    }
}

/// the core screen has a fixed physical orientation (it does not turn with the
/// layout), so its content is drawn at this fixed rotation -- 0 = upright, as the
/// device renders it. set to 90 / 180 / 270 to match a differently-mounted screen.
const SCREEN_ROTATION_OFFSET: i32 = 0;

/// draw the monogram core display: the vendored logo in the core's first (square)
/// cell -- inset below the label so it does not occlude it -- plus the live
/// screen string over it. the screen orientation is fixed (see the offset above).
fn draw_core_screen(
    component: &RenderComponent,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    textures: &mut HashMap<u8, Texture2D>,
) {
    let cell = w.min(h);
    let top = (cell * 0.16).max(14.0); // leave room for the component label
    let pad = (cell * 0.08).max(5.0);
    let size = (cell - top - pad).max(8.0);
    let (cx, cy) = (x + (cell - size) / 2.0, y + top);
    if let Some(texture) = monogram_logo_texture(textures) {
        let rotation = SCREEN_ROTATION_OFFSET.rem_euclid(360) as f32;
        draw_texture_ex(
            &texture,
            cx,
            cy,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(size, size)),
                rotation: rotation.to_radians(),
                ..Default::default()
            },
        );
    }
    if !component.screen_string.is_empty() {
        let font_size = (size / 9.0).max(9.0);
        let text_w = component.screen_string.len() as f32 * font_size * 0.55;
        draw_text(
            &component.screen_string,
            cx + (size - text_w) / 2.0,
            cy + size * 0.9,
            font_size,
            WHITE,
        );
    }
    // the rest of the core's footprint is the buttons cell; show its input
    // values (v[0] is the press bitmask) so live input is visible there too.
    let values = format!("{:?}", component.values);
    let vfs = (cell / 14.0).max(9.0);
    let (vx, vy) = if w >= h {
        (x + cell + (w - cell) / 2.0, y + h / 2.0)
    } else {
        (x + w / 2.0, y + cell + (h - cell) / 2.0)
    };
    draw_text(
        &values,
        vx - values.len() as f32 * vfs * 0.28,
        vy,
        vfs,
        WHITE,
    );
}

/// the monogram logo (240x240 big-endian rgb565) as a texture, cached under a
/// reserved key separate from the palettegear icon indices.
fn monogram_logo_texture(textures: &mut HashMap<u8, Texture2D>) -> Option<Texture2D> {
    const LOGO_KEY: u8 = 255;
    if let Some(texture) = textures.get(&LOGO_KEY) {
        return Some(texture.clone());
    }
    let logo = resources::monogram_logo()?;
    let dim = 240usize;
    let mut rgba = vec![0u8; dim * dim * 4];
    for i in 0..dim * dim {
        let value = ((logo[i * 2] as u16) << 8) | logo[i * 2 + 1] as u16;
        let r = (((value >> 11) & 0x1f) << 3) as u8;
        let g = (((value >> 5) & 0x3f) << 2) as u8;
        let b = ((value & 0x1f) << 3) as u8;
        rgba[i * 4..i * 4 + 4].copy_from_slice(&[r, g, b, 255]);
    }
    let texture = Texture2D::from_rgba8(dim as u16, dim as u16, &rgba);
    texture.set_filter(FilterMode::Nearest);
    textures.insert(LOGO_KEY, texture.clone());
    Some(texture)
}

fn draw_base_screen(
    component: &RenderComponent,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    slot_size: i32,
    textures: &mut HashMap<u8, Texture2D>,
) {
    let size = (slot_size as f32 / 1.5).max(8.0);
    let (cx, cy) = (x + w / 2.0 - size / 2.0, y + h / 2.0 - size / 2.0);
    if let Some(texture) = screen_texture(textures, component.screen_display) {
        draw_texture_ex(
            &texture,
            cx,
            cy,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(size, size)),
                ..Default::default()
            },
        );
    }
    let font_size = (slot_size / 12).max(10) as f32;
    draw_text(
        &component.screen_string,
        x + w / 2.0 - component.screen_string.len() as f32 * font_size * 0.25,
        cy + size,
        font_size,
        WHITE,
    );
}

/// decode an embedded `.raw` screen image into a texture, caching by index.
fn screen_texture(textures: &mut HashMap<u8, Texture2D>, index: u8) -> Option<Texture2D> {
    if let Some(texture) = textures.get(&index) {
        return Some(texture.clone());
    }
    let raw = resources::asset_bytes(index as u32, "raw")?;
    let texture = texture_from_raw(&raw);
    textures.insert(index, texture.clone());
    Some(texture)
}

/// decode a `.raw` screen image into a nearest-filtered texture.
fn texture_from_raw(raw: &[u8]) -> Texture2D {
    let decoded = image::decode(raw);
    let dim = image::IMAGE_DIM as u16;
    let mut rgba = vec![0u8; (dim as usize) * (dim as usize) * 4];
    for (i, &pixel) in decoded.pixels.iter().enumerate() {
        let (r, g, b) = decoded
            .palette
            .get(pixel as usize)
            .copied()
            .unwrap_or((0, 0, 0));
        rgba[i * 4..i * 4 + 4].copy_from_slice(&[r, g, b, 255]);
    }
    let texture = Texture2D::from_rgba8(dim, dim, &rgba);
    texture.set_filter(FilterMode::Nearest);
    texture
}

/// open a window displaying a `.raw` screen image until q / escape is pressed.
pub fn show_image(raw: Vec<u8>) {
    let conf = Conf {
        window_title: "sextant image".to_owned(),
        window_width: 1024,
        window_height: 1024,
        window_resizable: true,
        ..Default::default()
    };
    macroquad::Window::from_config(conf, async move {
        let texture = texture_from_raw(&raw);
        loop {
            if is_key_down(KeyCode::Q) || is_key_down(KeyCode::Escape) {
                break;
            }
            clear_background(WHITE);
            draw_texture_ex(
                &texture,
                0.0,
                0.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(screen_width(), screen_height())),
                    ..Default::default()
                },
            );
            next_frame().await;
        }
    });
}
