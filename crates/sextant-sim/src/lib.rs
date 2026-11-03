//! sextant-sim: the graphical simulator, built on `macroquad`.
//!
//! it renders the component layout (bodies, leds, labels, screen icons, values),
//! ticks at the same redraw / layout-poll rates as the original, and maps the
//! keyboard onto `sextant-fake` input injectors (digit keys select a component,
//! arrows drive dials/sliders, space presses buttons, tab rotates the layout).
//! this gives device-free development and visual verification of the demos.
//!
//! the [`geometry`] module is pure (no gpu) and pins the on-screen placement
//! against the upstream pygame renderer; [`input`] maps keys to simulator
//! actions. the macroquad render loop is in [`display`].

pub mod display;
pub mod geometry;
pub mod input;
