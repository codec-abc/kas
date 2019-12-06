// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! High-level drawing interface
//!
//! A [`Theme`] provides a high-level drawing interface. It may be provided by
//! the toolkit or separately (but dependent on a toolkit's drawing API).
//!
//! A theme is implemented in multiple parts: the [`Theme`] object is shared
//! by all windows and may provide shared resources (e.g. fonts and textures).
//! It is also responsible for creating a per-window [`ThemeWindow`] object and
//! draw handles ([`DrawHandle`]).

use std::any::Any;

use rusttype::Font;

use crate::layout;
use kas::draw::Colour;
use kas::{event, Widget};

/// A *theme* provides widget sizing and drawing implementations.
///
/// The theme is generic over some `Draw` type.
///
/// Objects of this type are copied within each window's data structure. For
/// large resources (e.g. fonts and icons) consider using external storage.
pub trait Theme<Draw> {
    /// The associated [`ThemeWindow`] implementation.
    type Window: Window<Draw> + 'static;

    /// The associated [`DrawHandle`] implementation.
    type DrawHandle: DrawHandle;

    /// Construct per-window storage
    ///
    /// A reference to the draw backend is provided allowing configuration.
    ///
    /// See also documentation on [`ThemeWindow::set_dpi_factor`].
    fn new_window(&self, draw: &mut Draw, dpi_factor: f32) -> Self::Window;

    /// Construct a [`DrawHandle`] object
    ///
    /// The `theme_window` is guaranteed to be one created by a call to
    /// [`Theme::new_window`] on `self`, and the `draw` reference is guaranteed
    /// to be identical to the one passed to [`Theme::new_window`].
    ///
    /// Note: this function is marked **unsafe** because the returned object
    /// requires a lifetime bound not exceeding that of all three pointers
    /// passed in. This ought to be expressible using generic associated types
    /// but currently is not: https://github.com/rust-lang/rust/issues/67089
    unsafe fn draw_handle(
        &self,
        draw: &mut Draw,
        theme_window: &mut Self::Window,
    ) -> Self::DrawHandle;

    /// Get the list of available fonts
    ///
    /// Currently, all fonts used must be specified up front by this method.
    /// (Dynamic addition of fonts may be enabled in the future.)
    ///
    /// This is considered a "getter" rather than a "constructor" method since
    /// the `Font` type is cheap to copy, and each window requires its own copy.
    /// It may also be useful to retain a `Font` handle for access to its
    /// methods.
    ///
    /// Corresponding `FontId`s may be created from the index into this list.
    /// The first font in the list will be the default font.
    ///
    /// TODO: this part of the API is dependent on `rusttype::Font`. We should
    /// build an abstraction over this, or possibly just pass the font bytes
    /// (although this makes re-use of fonts between windows difficult).
    fn get_fonts<'a>(&self) -> Vec<Font<'a>>;

    /// Light source
    ///
    /// This affects shadows on frames, etc. The light source has neutral colour
    /// and intensity such that the colour of flat surfaces is unaffected.
    ///
    /// Return value: `(a, b)` where `0 ≤ a < pi/2` is the angle to the screen
    /// normal (i.e. `a = 0` is straight at the screen) and `b` is the bearing
    /// (from UP, clockwise), both in radians.
    ///
    /// Currently this is not updated after initial set-up.
    fn light_direction(&self) -> (f32, f32);

    /// Background colour
    fn clear_colour(&self) -> Colour;
}

/// Per-window storage for the theme
///
/// Constructed via [`Theme::new_window`].
///
/// The main reason for this separation is to allow proper handling of
/// multi-window applications across screens with differing DPIs.
pub trait Window<Draw> {
    /// The associated [`SizeHandle`] implementation.
    type SizeHandle: SizeHandle;

    /// Construct a [`SizeHandle`] object
    ///
    /// The `draw` reference is guaranteed to be identical to the one used to
    /// construct this object.
    ///
    /// Note: this function is marked **unsafe** because the returned object
    /// requires a lifetime bound not exceeding that of all three pointers
    /// passed in. This ought to be expressible using generic associated types
    /// but currently is not: https://github.com/rust-lang/rust/issues/67089
    unsafe fn size_handle(&mut self, draw: &mut Draw) -> Self::SizeHandle;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Set the DPI factor.
    ///
    /// This method is called when the DPI changes (e.g. via system settings or
    /// when a window is moved to a different screen).
    ///
    /// On "standard" monitors, the factor is 1. High-DPI screens may have a
    /// factor of 2 or higher. The factor may not be an integer; e.g.
    /// `9/8 = 1.125` works well with many 1440p screens. It is recommended to
    /// round dimensions to the nearest integer, and cache the result:
    /// ```notest
    /// self.margin = (MARGIN * factor).round();
    /// ```
    fn set_dpi_factor(&mut self, factor: f32);
}

/// Handle passed to objects during draw and sizing operations
pub trait SizeHandle {
    /// Margin sizes
    ///
    /// May be called multiple times during a resize operation.
    ///
    /// See documentation of [`layout::Margins`].
    fn margins(&mut self, widget: &dyn Widget) -> layout::Margins;

    /// Widget size preferences
    ///
    /// Widgets should expect this to be called at least once for each axis.
    ///
    /// See documentation of [`layout::SizeRules`].
    fn size_rules(&mut self, widget: &dyn Widget, axis: layout::AxisInfo) -> layout::SizeRules;
}

/// Handle passed to objects during draw and sizing operations
pub trait DrawHandle {
    /// Draw a widget
    ///
    /// This method is called to draw each visible widget (and should not
    /// attempt recursion on child widgets).
    fn draw(&mut self, ev_mgr: &event::Manager, widget: &dyn kas::Widget);
}
