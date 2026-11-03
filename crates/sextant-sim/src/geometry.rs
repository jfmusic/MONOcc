//! pure layout geometry for the simulator, free of any gpu / windowing code.
//!
//! given the surface size and the layout's slot extrema, this computes where
//! each component's body rectangle, inset led outline, root-port marker, and
//! per-cell edge connectors land in pixels. the macroquad renderer draws from
//! these; the contract test pins the exact pixels against the upstream pygame
//! implementation.

use sextant_core::coordinates::{Orientation, Slot};

pub const DISPLAY_SIZE: (i32, i32) = (1024, 576);
pub const COMPONENT_MARGIN: i32 = 4;
pub const COMPONENT_COLOR: (u8, u8, u8) = (164, 164, 164);
pub const COMPONENT_SELECTED_COLOR: (u8, u8, u8) = (164, 164, 255);

/// a pixel rectangle (top-left origin), matching pygame's `Rect`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// a pixel circle marker.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Circle {
    pub x: i32,
    pub y: i32,
    pub r: i32,
}

/// which side of the component body a connector sits on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    North,
    South,
    East,
    West,
}

/// a magnetic (or usb-c) connector at the midpoint of one cell edge.
#[derive(Clone, Copy, Debug)]
pub struct Connector {
    pub pos: Circle,
    pub edge: Edge,
}

/// every per-cell connector on a component's perimeter: one at the midpoint of
/// each outward cell edge (modules magnetically couple edge-to-edge). the grid
/// is read back from the placed body so it is correct at any orientation.
fn perimeter_connectors(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    slot_size: i32,
    connector_size: i32,
) -> Vec<Connector> {
    let step = slot_size + COMPONENT_MARGIN;
    let cols = ((w + COMPONENT_MARGIN) / step).max(1);
    let rows = ((h + COMPONENT_MARGIN) / step).max(1);
    // per-cell connectors sit ~10% larger than the attachment marker
    // (`connector_size`) so the marker, drawn on top, stays visible.
    let radius = ((connector_size * 11 + 9) / 10).max(3);
    let mut out = Vec::new();
    for c in 0..cols {
        let cx = x + c * step + slot_size / 2;
        out.push(Connector {
            pos: Circle {
                x: cx,
                y,
                r: radius,
            },
            edge: Edge::North,
        });
        out.push(Connector {
            pos: Circle {
                x: cx,
                y: y + h,
                r: radius,
            },
            edge: Edge::South,
        });
    }
    for r in 0..rows {
        let cy = y + r * step + slot_size / 2;
        out.push(Connector {
            pos: Circle {
                x,
                y: cy,
                r: radius,
            },
            edge: Edge::West,
        });
        out.push(Connector {
            pos: Circle {
                x: x + w,
                y: cy,
                r: radius,
            },
            edge: Edge::East,
        });
    }
    out
}

/// integer division through float truncation, matching python `int(a / b)`.
fn idiv(a: i32, b: f64) -> i32 {
    (a as f64 / b) as i32
}

/// the placement origin and per-slot size for a layout with the given slot
/// bounding box on a surface of `surface_size`.
pub fn initial_pos_and_slot_size(
    surface_size: (i32, i32),
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
) -> ((i32, i32), i32) {
    let count_x = (min_x - max_x).abs() + 1;
    let count_y = (min_y - max_y).abs() + 1;
    let margin_x = count_x * COMPONENT_MARGIN + COMPONENT_MARGIN * 3;
    let margin_y = count_y * COMPONENT_MARGIN + COMPONENT_MARGIN * 3;
    let slot_width = idiv(surface_size.0 - margin_x, count_x as f64);
    let slot_height = idiv(surface_size.1 - margin_y, count_y as f64);
    let slot_size = slot_width.min(slot_height);
    let initial_x = slot_width * -min_x + margin_x / 2;
    let initial_y = slot_height * max_y + margin_y / 2;
    ((initial_x, initial_y), slot_size)
}

/// the placed geometry of a single component.
#[derive(Clone, Copy, Debug)]
pub struct ComponentGeometry {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub wide: bool,
    pub body: Rect,
    pub led: Rect,
    pub root_port: Circle,
    pub connector_size: i32,
    pub led_thickness: i32,
    pub slot_size: i32,
}

impl ComponentGeometry {
    pub fn new(
        initial_pos: (i32, i32),
        slot_size: i32,
        slot: Slot,
        orientation: Orientation,
        footprint: (i32, i32),
    ) -> ComponentGeometry {
        let (cells_w, cells_h) = footprint;
        let wide = cells_w > 1;
        let width = cells_w * slot_size + (cells_w - 1) * COMPONENT_MARGIN;
        let height = cells_h * slot_size + (cells_h - 1) * COMPONENT_MARGIN;

        let (x, y, w, h) = orient(initial_pos, slot_size, slot, orientation, width, height);

        let led_thickness = idiv(slot_size, 32.0);
        let connector_size = led_thickness;
        let body = Rect { x, y, w, h };
        let led = Rect {
            x: x + led_thickness,
            y: y + led_thickness,
            w: w - led_thickness * 2,
            h: h - led_thickness * 2,
        };
        let root_port = root_port_position(x, y, w, h, orientation, wide, connector_size);

        ComponentGeometry {
            x,
            y,
            w,
            h,
            wide,
            body,
            led,
            root_port,
            connector_size,
            led_thickness,
            slot_size,
        }
    }

    /// the per-cell magnetic connectors around the placed body.
    pub fn connectors(&self) -> Vec<Connector> {
        perimeter_connectors(
            self.x,
            self.y,
            self.w,
            self.h,
            self.slot_size,
            self.connector_size,
        )
    }
}

fn slot_coordinates(initial_pos: (i32, i32), slot_size: i32, slot: Slot) -> (i32, i32) {
    let step = slot_size + COMPONENT_MARGIN;
    (initial_pos.0 + step * slot.0, initial_pos.1 - step * slot.1)
}

/// rotate the component's natural width/height into placed `(x, y, w, h)`.
fn orient(
    initial_pos: (i32, i32),
    slot_size: i32,
    slot: Slot,
    orientation: Orientation,
    width: i32,
    height: i32,
) -> (i32, i32, i32, i32) {
    let (x, y) = slot_coordinates(initial_pos, slot_size, slot);
    match orientation {
        90 => (x, y, height, width),
        270 => (x, y - (width - slot_size), height, width),
        180 => (x - (width - slot_size), y, width, height),
        _ => (x, y, width, height),
    }
}

fn root_port_position(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    orientation: Orientation,
    wide: bool,
    connector_size: i32,
) -> Circle {
    let narrow_div = if wide { 4.0 } else { 2.0 };
    // the long side of a wide component shifts its root port off-center.
    let long_div = if wide { 1.33 } else { 2.0 };
    let (px, py) = match orientation {
        90 => (x + w, y + idiv(h, narrow_div)),
        180 => (x + idiv(w, long_div), y + h),
        270 => (x, y + idiv(h, long_div)),
        _ => (x + idiv(w, narrow_div), y),
    };
    Circle {
        x: px,
        y: py,
        r: connector_size,
    }
}

/// the body edge of `body` that faces `target`: whichever axis the target lies
/// farther along, and its sign.
pub fn edge_toward(body: Rect, target: Rect) -> Edge {
    let (bcx, bcy) = (body.x + body.w / 2, body.y + body.h / 2);
    let (tcx, tcy) = (target.x + target.w / 2, target.y + target.h / 2);
    if (tcx - bcx).abs() >= (tcy - bcy).abs() {
        if tcx >= bcx {
            Edge::East
        } else {
            Edge::West
        }
    } else if tcy >= bcy {
        Edge::South
    } else {
        Edge::North
    }
}

/// the per-cell connector on `edge` nearest `(rx, ry)`, as an attachment marker of
/// radius `r` centered on it, so the marker snaps onto a real connector rather
/// than the bare edge. `None` when that edge carries no connector.
pub fn connector_on_edge(
    connectors: &[Connector],
    edge: Edge,
    rx: i32,
    ry: i32,
    r: i32,
) -> Option<Circle> {
    connectors
        .iter()
        .filter(|c| c.edge == edge)
        .min_by_key(|c| {
            let (dx, dy) = ((c.pos.x - rx) as i64, (c.pos.y - ry) as i64);
            dx * dx + dy * dy
        })
        .map(|c| Circle {
            x: c.pos.x,
            y: c.pos.y,
            r,
        })
}

/// the body edge a component's male (parent-facing) port sits on, from its
/// orientation: north when upright, turning clockwise.
pub fn male_edge(orientation: Orientation) -> Edge {
    match orientation.rem_euclid(360) {
        90 => Edge::East,
        180 => Edge::South,
        270 => Edge::West,
        _ => Edge::North,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sextant_core::ComponentLayout;
    use std::collections::HashSet;

    // the exact body rectangles drawn for the expertkit layout, rotated 90
    // degrees, on a 2048x2048 surface -- from test_ambit AmbitSimulatorTest.
    const EXPECTED_BODIES: &[(i32, i32, i32, i32)] = &[
        (617, 822, 403, 403),
        (1024, 822, 403, 403),
        (617, 1229, 403, 403),
        (1024, 1229, 403, 403),
        (617, 1636, 810, 403),
        (617, 415, 403, 403),
        (1024, 415, 403, 403),
        (617, 8, 810, 403),
    ];

    const EXPECTED_LEDS: &[(i32, i32, i32, i32)] = &[
        (629, 427, 379, 379),
        (1036, 427, 379, 379),
        (629, 20, 786, 379),
        (1036, 834, 379, 379),
        (629, 834, 379, 379),
        (629, 1241, 379, 379),
        (1036, 1241, 379, 379),
        (629, 1648, 786, 379),
    ];

    const EXPECTED_PORTS: &[(i32, i32)] = &[
        (1427, 1023),
        (1020, 1023),
        (818, 1229),
        (1024, 1430),
        (819, 1636),
        (818, 818),
        (1024, 616),
        (1226, 411),
    ];

    fn expertkit_rotated() -> ComponentLayout {
        // base orientation 90 reproduces a single TAB rotation in the simulator.
        let mut layout = ComponentLayout::new(90);
        layout.update(&sextant_fake::layouts::expertkit());
        layout
    }

    #[test]
    fn render_geometry_matches_pygame_contract() {
        let layout = expertkit_rotated();
        let (min_x, min_y, max_x, max_y) = layout.extrema();
        let (initial_pos, slot_size) =
            initial_pos_and_slot_size((2048, 2048), min_x, min_y, max_x, max_y);
        assert_eq!(403, slot_size);

        let mut bodies = HashSet::new();
        let mut leds = HashSet::new();
        let mut ports = HashSet::new();
        for component in layout.connected() {
            let geometry = ComponentGeometry::new(
                initial_pos,
                slot_size,
                component.slot.unwrap(),
                component.orientation.unwrap(),
                component.footprint,
            );
            bodies.insert((
                geometry.body.x,
                geometry.body.y,
                geometry.body.w,
                geometry.body.h,
            ));
            leds.insert((
                geometry.led.x,
                geometry.led.y,
                geometry.led.w,
                geometry.led.h,
            ));
            ports.insert((geometry.root_port.x, geometry.root_port.y));
        }

        for expected in EXPECTED_BODIES {
            assert!(bodies.contains(expected), "missing body {expected:?}");
        }
        for expected in EXPECTED_LEDS {
            assert!(leds.contains(expected), "missing led {expected:?}");
        }
        for expected in EXPECTED_PORTS {
            assert!(ports.contains(expected), "missing root port {expected:?}");
        }
    }

    fn edge_counts(footprint: (i32, i32)) -> (usize, usize, usize, usize) {
        let geometry = ComponentGeometry::new((0, 0), 100, (0, 0), 0, footprint);
        let count = |e: Edge| geometry.connectors().iter().filter(|c| c.edge == e).count();
        (
            count(Edge::North),
            count(Edge::South),
            count(Edge::West),
            count(Edge::East),
        )
    }

    #[test]
    fn connectors_sit_on_every_perimeter_cell_edge() {
        // (north, south, west, east) per the physical modules.
        // palette button / base 1x1: one connector on each of the four sides.
        assert_eq!((1, 1, 1, 1), edge_counts((1, 1)));
        // core 1x2: two on each long (left/right) side, one top (usb) + one bottom.
        assert_eq!((1, 1, 2, 2), edge_counts((1, 2)));
        // orbiter 2x2: two on every side.
        assert_eq!((2, 2, 2, 2), edge_counts((2, 2)));
        // tri-dial 2x1: two on each long side (top/bottom), one on each short side.
        assert_eq!((2, 2, 1, 1), edge_counts((2, 1)));
    }

    #[test]
    fn edge_toward_picks_the_facing_side() {
        let body = Rect {
            x: 0,
            y: 0,
            w: 100,
            h: 100,
        };
        let at = |x, y| Rect { x, y, w: 20, h: 20 };
        assert_eq!(Edge::East, edge_toward(body, at(200, 40)));
        assert_eq!(Edge::North, edge_toward(body, at(30, -200)));
        assert_eq!(Edge::South, edge_toward(body, at(40, 300)));
        assert_eq!(Edge::West, edge_toward(body, at(-200, 50)));
    }

    #[test]
    fn attachment_snaps_to_the_connector_on_the_edge() {
        // a 2x2 module with a reference point to its east and below (the orbiter
        // toward the core): the marker snaps onto the lower east connector.
        let g = ComponentGeometry::new((200, 200), 100, (0, 0), 0, (2, 2));
        let connectors = g.connectors();
        let (rx, ry) = (g.x + g.w + 90, g.y + g.h);
        let p = connector_on_edge(&connectors, Edge::East, rx, ry, 5).unwrap();
        let east: Vec<&Connector> = connectors.iter().filter(|c| c.edge == Edge::East).collect();
        // it coincides with the lower east connector (largest screen y), radius 5.
        let lowest = east.iter().max_by_key(|c| c.pos.y).unwrap();
        assert_eq!((lowest.pos.x, lowest.pos.y, 5), (p.x, p.y, p.r));
        // an edge with no connector yields none.
        assert!(connector_on_edge(&[], Edge::East, rx, ry, 5).is_none());
    }

    #[test]
    fn multi_cell_footprints_size_the_body() {
        // a 2x2 footprint spans two slots plus the inter-cell margin each way.
        let orbiter = ComponentGeometry::new((0, 0), 100, (0, 0), 0, (2, 2));
        assert_eq!(2 * 100 + COMPONENT_MARGIN, orbiter.w);
        assert_eq!(2 * 100 + COMPONENT_MARGIN, orbiter.h);
        // a 1x2 footprint is one slot wide, two tall.
        let tri = ComponentGeometry::new((0, 0), 100, (0, 0), 0, (1, 2));
        assert_eq!(100, tri.w);
        assert_eq!(2 * 100 + COMPONENT_MARGIN, tri.h);
        // a 2x1 footprint is two wide, one tall.
        let core = ComponentGeometry::new((0, 0), 100, (0, 0), 0, (2, 1));
        assert_eq!(2 * 100 + COMPONENT_MARGIN, core.w);
        assert_eq!(100, core.h);
    }
}
