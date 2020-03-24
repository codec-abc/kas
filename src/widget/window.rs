// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window widgets

use std::fmt::{self, Debug};

use kas::draw::{DrawHandle, SizeHandle};
use kas::event::{Callback, Manager, VoidMsg};
use kas::layout::{AxisInfo, SizeRules};
use kas::prelude::*;

/// The main instantiation of the [`Window`] trait.
#[handler(generics = <> where W: Widget<Msg = VoidMsg>)]
#[derive(Widget)]
pub struct Window<W: Widget + 'static> {
    #[widget_core]
    core: CoreData,
    enforce_min: bool,
    enforce_max: bool,
    title: CowString,
    #[widget]
    w: W,
    fns: Vec<(Callback, &'static dyn Fn(&mut W, &mut Manager))>,
}

impl<W: Widget> Debug for Window<W> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Window {{ core: {:?}, solver: <omitted>, w: {:?}, fns: [",
            self.core, self.w
        )?;
        let mut iter = self.fns.iter();
        if let Some(first) = iter.next() {
            write!(f, "({:?}, <Fn>)", first.0)?;
            for next in iter {
                write!(f, ", ({:?}, <Fn>)", next.0)?;
            }
        }
        write!(f, "] }}")
    }
}

impl<W: Widget + Clone> Clone for Window<W> {
    fn clone(&self) -> Self {
        Window {
            core: self.core.clone(),
            enforce_min: self.enforce_min,
            enforce_max: self.enforce_max,
            title: self.title.clone(),
            w: self.w.clone(),
            fns: self.fns.clone(),
        }
    }
}

impl<W: Widget> Window<W> {
    /// Create
    pub fn new<T: Into<CowString>>(title: T, w: W) -> Window<W> {
        Window {
            core: Default::default(),
            enforce_min: true,
            enforce_max: false,
            title: title.into(),
            w,
            fns: Vec::new(),
        }
    }

    /// Configure whether min/max dimensions are forced
    ///
    /// By default, the min size is enforced but not the max.
    pub fn set_enforce_size(&mut self, min: bool, max: bool) {
        self.enforce_min = min;
        self.enforce_max = max;
    }

    /// Add a closure to be called, with a reference to self, on the given
    /// condition. The closure must be passed by reference.
    pub fn add_callback(&mut self, condition: Callback, f: &'static dyn Fn(&mut W, &mut Manager)) {
        self.fns.push((condition, f));
    }
}

impl<W: Widget> Layout for Window<W> {
    #[inline]
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        self.w.size_rules(size_handle, axis)
    }

    #[inline]
    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, rect: Rect, align: AlignHints) {
        self.core.rect = rect;
        self.w.set_rect(size_handle, rect, align);
    }

    #[inline]
    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        self.w.find_id(coord)
    }

    #[inline]
    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState) {
        self.w.draw(draw_handle, mgr);
    }
}

impl<W: Widget<Msg = VoidMsg> + 'static> kas::Window for Window<W> {
    fn title(&self) -> &str {
        &self.title
    }

    fn find_size(&mut self, size_handle: &mut dyn SizeHandle) -> (Option<Size>, Size) {
        let (min, ideal) = layout::solve(self, size_handle);
        let min = if self.enforce_min { Some(min) } else { None };
        (min, ideal)
    }

    fn resize(
        &mut self,
        size_handle: &mut dyn SizeHandle,
        size: Size,
    ) -> (Option<Size>, Option<Size>) {
        let (min, ideal) = layout::solve_and_set(self, size_handle, size);
        (
            if self.enforce_min { Some(min) } else { None },
            if self.enforce_max { Some(ideal) } else { None },
        )
    }

    fn callbacks(&self) -> Vec<(usize, Callback)> {
        self.fns.iter().map(|(cond, _)| *cond).enumerate().collect()
    }

    /// Trigger a callback (see `iter_callbacks`).
    fn trigger_callback(&mut self, index: usize, mgr: &mut Manager) {
        let cb = &mut self.fns[index].1;
        cb(&mut self.w, mgr);
    }
}
