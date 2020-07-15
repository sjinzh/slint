//! This module contains the basic datastructures that are exposed to the C API

use super::slice::Slice;
use core::pin::Pin;
use std::cell::Cell;
use vtable::*;

#[cfg(feature = "rtti")]
use crate::rtti::{BuiltinItem, FieldInfo, FieldOffset, PropertyInfo, ValueType};
use const_field_offset::FieldOffsets;
use corelib_macro::*;

/// 2D Rectangle
pub type Rect = euclid::default::Rect<f32>;
/// 2D Point
pub type Point = euclid::default::Point2D<f32>;
/// 2D Size
pub type Size = euclid::default::Size2D<f32>;

/// Expand Rect so that cbindgen can see it. ( is in fact euclid::default::Rect<f32>)
#[cfg(cbindgen)]
#[repr(C)]
struct Rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

/// Expand Point so that cbindgen can see it. ( is in fact euclid::default::PointD2<f32>)
#[cfg(cbindgen)]
#[repr(C)]
struct Point {
    x: f32,
    y: f32,
}

/// A Component is representing an unit that is allocated together
#[vtable]
#[repr(C)]
pub struct ComponentVTable {
    /// Visit the children of the item at index `index`.
    /// Note that the root item is at index 0, so passing 0 would visit the item under root (the children of root).
    /// If you want to visit the root item, you need to pass -1 as an index
    pub visit_children_item: extern "C" fn(
        core::pin::Pin<VRef<ComponentVTable>>,
        index: isize,
        visitor: VRefMut<ItemVisitorVTable>,
    ),

    /// Returns the layout info for this component
    pub layout_info: extern "C" fn(core::pin::Pin<VRef<ComponentVTable>>) -> LayoutInfo,

    /// Will compute the layout of
    pub compute_layout: extern "C" fn(core::pin::Pin<VRef<ComponentVTable>>),
}

/// This structure must be present in items that are Rendered and contains information.
/// Used by the backend.
#[derive(Default)]
#[repr(C)]
pub struct CachedRenderingData {
    /// Used and modified by the backend, should be initialized to 0 by the user code
    pub(crate) cache_index: Cell<usize>,
    /// Set to false initially and when changes happen that require updating the cache
    pub(crate) cache_ok: Cell<bool>,
}

impl CachedRenderingData {
    pub(crate) fn low_level_rendering_primitive<
        'a,
        GraphicsBackend: crate::graphics::GraphicsBackend,
    >(
        &self,
        cache: &'a crate::graphics::RenderingCache<GraphicsBackend>,
    ) -> Option<&'a GraphicsBackend::LowLevelRenderingPrimitive> {
        if !self.cache_ok.get() {
            return None;
        }
        Some(cache.entry_at(self.cache_index.get()))
    }
}

/// The item tree is an array of ItemTreeNode representing a static tree of items
/// within a component.
#[repr(u8)]
pub enum ItemTreeNode<T> {
    /// Static item
    Item {
        /// byte offset where we can find the item (from the *ComponentImpl)
        item: vtable::VOffset<T, ItemVTable, vtable::PinnedFlag>,

        /// number of children
        chilren_count: u32,

        /// index of the first children within the item tree
        children_index: u32,
    },
    /// A placeholder for many instance of item in their own component which
    /// are instantiated according to a model.
    DynamicTree {
        /// the undex which is passed in the visit_dynamic callback.
        index: usize,
    },
}

/// Items are the nodes in the render tree.
#[vtable]
#[repr(C)]
pub struct ItemVTable {
    /// Returns the geometry of this item (relative to its parent item)
    pub geometry: extern "C" fn(core::pin::Pin<VRef<ItemVTable>>) -> Rect,

    /// offset in bytes fromthe *const ItemImpl.
    /// isize::MAX  means None
    #[allow(non_upper_case_globals)]
    #[offset(CachedRenderingData)]
    pub cached_rendering_data_offset: usize,

    /// Return the rendering primitive used to display this item.
    pub rendering_primitive: extern "C" fn(core::pin::Pin<VRef<ItemVTable>>) -> RenderingPrimitive,

    /// We would need max/min/preferred size, and all layout info
    pub layouting_info: extern "C" fn(core::pin::Pin<VRef<ItemVTable>>) -> LayoutInfo,

    /// input event
    pub input_event: extern "C" fn(core::pin::Pin<VRef<ItemVTable>>, MouseEvent),
}

/// The constraint that applies to an item
#[repr(C)]
#[derive(Clone)]
pub struct LayoutInfo {
    min_width: f32,
    max_width: f32,
    min_height: f32,
    max_height: f32,
}

impl Default for LayoutInfo {
    fn default() -> Self {
        LayoutInfo { min_width: 0., max_width: f32::MAX, min_height: 0., max_height: f32::MAX }
    }
}

/// RGBA color
#[derive(Copy, Clone, PartialEq, Debug, Default)]
#[repr(C)]
pub struct Color {
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

impl Color {
    /// Construct a color from an integer encoded as `0xAARRGGBB`
    pub const fn from_argb_encoded(encoded: u32) -> Color {
        Color {
            red: (encoded >> 16) as u8,
            green: (encoded >> 8) as u8,
            blue: encoded as u8,
            alpha: (encoded >> 24) as u8,
        }
    }

    /// Construct a color from its RGBA components as u8
    pub const fn from_rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Color {
        Color { red, green, blue, alpha }
    }
    /// Construct a color from its RGB components as u8
    pub const fn from_rgb(red: u8, green: u8, blue: u8) -> Color {
        Color::from_rgba(red, green, blue, 0xff)
    }

    /// Returns `(red, green, blue, alpha)` encoded as f32
    pub fn as_rgba_f32(&self) -> (f32, f32, f32, f32) {
        (
            (self.red as f32) / 255.0,
            (self.green as f32) / 255.0,
            (self.blue as f32) / 255.0,
            (self.alpha as f32) / 255.0,
        )
    }

    /// Returns `(red, green, blue, alpha)` encoded as u8
    pub fn as_rgba_u8(&self) -> (u8, u8, u8, u8) {
        (self.red, self.green, self.blue, self.alpha)
    }

    /// Returns `(alpha, red, green, blue)` encoded as u32
    pub fn as_argb_encoded(&self) -> u32 {
        ((self.red as u32) << 16)
            | ((self.green as u32) << 8)
            | (self.blue as u32)
            | ((self.alpha as u32) << 24)
    }

    /// A constant for the black color
    pub const BLACK: Color = Color::from_rgb(0, 0, 0);
    /// A constant for the white color
    pub const WHITE: Color = Color::from_rgb(255, 255, 255);
    /// A constant for the transparent color
    pub const TRANSPARENT: Color = Color::from_rgba(0, 0, 0, 0);
}

impl From<u32> for Color {
    fn from(encoded: u32) -> Self {
        Color::from_argb_encoded(encoded)
    }
}

impl crate::abi::properties::InterpolatedPropertyValue for Color {
    fn interpolate(self, target_value: Self, t: f32) -> Self {
        Self {
            red: self.red.interpolate(target_value.red, t),
            green: self.green.interpolate(target_value.green, t),
            blue: self.blue.interpolate(target_value.blue, t),
            alpha: self.alpha.interpolate(target_value.alpha, t),
        }
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "argb({}, {}, {}, {})", self.alpha, self.red, self.green, self.blue)
    }
}

/// A resource is a reference to binary data, for example images. They can be accessible on the file
/// system or embedded in the resulting binary. Or they might be URLs to a web server and a downloaded
/// is necessary before they can be used.
#[derive(Clone, PartialEq, Debug)]
#[repr(u8)]
pub enum Resource {
    /// A resource that does not represent any data.
    None,
    /// A resource that points to a file in the file system
    AbsoluteFilePath(crate::SharedString),
    /// A resource that is embedded in the program and accessible via pointer
    EmbeddedData(super::slice::Slice<'static, u8>),
}

impl Default for Resource {
    fn default() -> Self {
        Resource::None
    }
}

#[repr(C)]
#[derive(FieldOffsets, Default, BuiltinItem, Clone, Debug, PartialEq)]
#[pin]
/// PathLineTo describes the event of moving the cursor on the path to the specified location
/// along a straight line.
pub struct PathLineTo {
    #[rtti_field]
    /// The x coordinate where the line should go to.
    pub x: f32,
    #[rtti_field]
    /// The y coordinate where the line should go to.
    pub y: f32,
}

#[repr(C)]
#[derive(FieldOffsets, Default, BuiltinItem, Clone, Debug, PartialEq)]
#[pin]
/// PathArcTo describes the event of moving the cursor on the path across an arc to the specified
/// x/y coordinates, with the specified x/y radius and additional properties.
pub struct PathArcTo {
    #[rtti_field]
    /// The x coordinate where the arc should end up.
    pub x: f32,
    #[rtti_field]
    /// The y coordinate where the arc should end up.
    pub y: f32,
    #[rtti_field]
    /// The radius on the x-axis of the arc.
    pub radius_x: f32,
    #[rtti_field]
    /// The radius on the y-axis of the arc.
    pub radius_y: f32,
    #[rtti_field]
    /// The rotation along the x-axis of the arc in degress.
    pub x_rotation: f32,
    #[rtti_field]
    /// large_arc indicates whether to take the long or the shorter path to complete the arc.
    pub large_arc: bool,
    #[rtti_field]
    /// sweep indicates the direction of the arc. If true, a clockwise direction is chosen,
    /// otherwise counter-clockwise.
    pub sweep: bool,
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
/// PathElement describes a single element on a path, such as move-to, line-to, etc.
pub enum PathElement {
    /// The LineTo variant describes a line.
    LineTo(PathLineTo),
    /// The PathArcTo variant describes an arc.
    ArcTo(PathArcTo),
    /// Indicates that the path should be closed now by connecting to the starting point.
    Close,
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
/// PathEvent is a low-level data structure describing the composition of a path. Typically it is
/// generated at compile time from a higher-level description, such as SVG commands.
pub enum PathEvent {
    /// The beginning of the path.
    Begin,
    /// A straight line on the path.
    Line,
    /// A quadratic bezier curve on the path.
    Quadratic,
    /// A cubic bezier curve on the path.
    Cubic,
    /// The end of the path that remains open.
    EndOpen,
    /// The end of a path that is closed.
    EndClosed,
}

struct ToLyonPathEventIterator<'a> {
    events_it: std::slice::Iter<'a, PathEvent>,
    coordinates_it: std::slice::Iter<'a, Point>,
    first: Option<&'a Point>,
    last: Option<&'a Point>,
}

impl<'a> Iterator for ToLyonPathEventIterator<'a> {
    type Item = lyon::path::Event<lyon::math::Point, lyon::math::Point>;
    fn next(&mut self) -> Option<Self::Item> {
        use lyon::path::Event;

        self.events_it.next().map(|event| match event {
            PathEvent::Begin => Event::Begin { at: self.coordinates_it.next().unwrap().clone() },
            PathEvent::Line => Event::Line {
                from: self.coordinates_it.next().unwrap().clone(),
                to: self.coordinates_it.next().unwrap().clone(),
            },
            PathEvent::Quadratic => Event::Quadratic {
                from: self.coordinates_it.next().unwrap().clone(),
                ctrl: self.coordinates_it.next().unwrap().clone(),
                to: self.coordinates_it.next().unwrap().clone(),
            },
            PathEvent::Cubic => Event::Cubic {
                from: self.coordinates_it.next().unwrap().clone(),
                ctrl1: self.coordinates_it.next().unwrap().clone(),
                ctrl2: self.coordinates_it.next().unwrap().clone(),
                to: self.coordinates_it.next().unwrap().clone(),
            },
            PathEvent::EndOpen => Event::End {
                first: self.first.unwrap().clone(),
                last: self.last.unwrap().clone(),
                close: false,
            },
            PathEvent::EndClosed => Event::End {
                first: self.first.unwrap().clone(),
                last: self.last.unwrap().clone(),
                close: true,
            },
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.events_it.size_hint()
    }
}

impl<'a> ExactSizeIterator for ToLyonPathEventIterator<'a> {}

struct TransformedLyonPathIterator<EventIt> {
    it: EventIt,
    transform: lyon::math::Transform,
}

impl<EventIt: Iterator<Item = lyon::path::Event<lyon::math::Point, lyon::math::Point>>> Iterator
    for TransformedLyonPathIterator<EventIt>
{
    type Item = lyon::path::Event<lyon::math::Point, lyon::math::Point>;
    fn next(&mut self) -> Option<Self::Item> {
        self.it.next().map(|ev| ev.transformed(&self.transform))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.it.size_hint()
    }
}

impl<EventIt: Iterator<Item = lyon::path::Event<lyon::math::Point, lyon::math::Point>>>
    ExactSizeIterator for TransformedLyonPathIterator<EventIt>
{
}

/// LyonPathIterator is a data structure that acts as starting point for iterating
/// through the low-level events of a path. If the path was constructed from said
/// events, then it is a very thin abstraction. If the path was created from higher-level
/// elements, then an intermediate lyon path is required/built.
pub struct LyonPathIterator<'a> {
    it: LyonPathIteratorVariant<'a>,
    transform: Option<lyon::math::Transform>,
}

enum LyonPathIteratorVariant<'a> {
    FromPath(lyon::path::Path),
    FromEvents(&'a crate::SharedArray<PathEvent>, &'a crate::SharedArray<Point>),
}

impl<'a> LyonPathIterator<'a> {
    /// Create a new iterator for path traversal.
    pub fn iter(
        &'a self,
    ) -> Box<dyn Iterator<Item = lyon::path::Event<lyon::math::Point, lyon::math::Point>> + 'a>
    {
        match &self.it {
            LyonPathIteratorVariant::FromPath(path) => self.apply_transform(path.iter()),
            LyonPathIteratorVariant::FromEvents(events, coordinates) => {
                Box::new(self.apply_transform(ToLyonPathEventIterator {
                    events_it: events.iter(),
                    coordinates_it: coordinates.iter(),
                    first: coordinates.first(),
                    last: coordinates.last(),
                }))
            }
        }
    }

    /// This function changes the iterator to be one that applies a transformation to make the path fit into the specified bounds.
    pub fn fitted(mut self, width: f32, height: f32) -> LyonPathIterator<'a> {
        if width > 0. || height > 0. {
            let br = lyon::algorithms::aabb::bounding_rect(self.iter());
            self.transform = Some(lyon::algorithms::fit::fit_rectangle(
                &br,
                &Rect::from_size(Size::new(width, height)),
                lyon::algorithms::fit::FitStyle::Min,
            ));
        }

        self
    }

    fn apply_transform(
        &'a self,
        event_it: impl Iterator<Item = lyon::path::Event<lyon::math::Point, lyon::math::Point>> + 'a,
    ) -> Box<dyn Iterator<Item = lyon::path::Event<lyon::math::Point, lyon::math::Point>> + 'a>
    {
        match self.transform {
            Some(transform) => Box::new(TransformedLyonPathIterator { it: event_it, transform }),
            None => Box::new(event_it),
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
/// PathData represents a path described by either high-level elements or low-level
/// events and coordinates.
pub enum PathData {
    /// None is the variant when the path is empty.
    None,
    /// The Elements variant is used to make a Path from shared arrays of elements.
    Elements(crate::SharedArray<PathElement>),
    /// The Events variant describes the path as a series of low-level events and
    /// associated coordinates.
    Events(crate::SharedArray<PathEvent>, crate::SharedArray<Point>),
}

impl Default for PathData {
    fn default() -> Self {
        Self::None
    }
}

impl PathData {
    /// This function returns an iterator that allows traversing the path by means of lyon events.
    pub fn iter(&self) -> LyonPathIterator {
        LyonPathIterator {
            it: match self {
                PathData::None => LyonPathIteratorVariant::FromPath(lyon::path::Path::new()),
                PathData::Elements(elements) => LyonPathIteratorVariant::FromPath(
                    PathData::build_path(elements.as_slice().iter()),
                ),
                PathData::Events(events, coordinates) => {
                    LyonPathIteratorVariant::FromEvents(events, coordinates)
                }
            },
            transform: None,
        }
    }

    fn build_path(element_it: std::slice::Iter<PathElement>) -> lyon::path::Path {
        use lyon::geom::SvgArc;
        use lyon::math::{Angle, Point, Vector};
        use lyon::path::{
            builder::{Build, FlatPathBuilder, SvgBuilder},
            ArcFlags,
        };

        let mut path_builder = lyon::path::Path::builder().with_svg();
        for element in element_it {
            match element {
                PathElement::LineTo(PathLineTo { x, y }) => {
                    path_builder.line_to(Point::new(*x, *y))
                }
                PathElement::ArcTo(PathArcTo {
                    x,
                    y,
                    radius_x,
                    radius_y,
                    x_rotation,
                    large_arc,
                    sweep,
                }) => {
                    let radii = Vector::new(*radius_x, *radius_y);
                    let x_rotation = Angle::degrees(*x_rotation);
                    let flags = ArcFlags { large_arc: *large_arc, sweep: *sweep };
                    let to = Point::new(*x, *y);

                    let svg_arc = SvgArc {
                        from: path_builder.current_position(),
                        radii,
                        x_rotation,
                        flags,
                        to,
                    };

                    if svg_arc.is_straight_line() {
                        path_builder.line_to(to);
                    } else {
                        path_builder.arc_to(radii, x_rotation, flags, to)
                    }
                }
                PathElement::Close => path_builder.close(),
            }
        }

        path_builder.build()
    }
}

#[no_mangle]
/// This function is used for the low-level C++ interface to allocate the backing vector for a shared path element array.
pub unsafe extern "C" fn sixtyfps_new_path_elements(
    out: *mut c_void,
    first_element: *const PathElement,
    count: usize,
) {
    let arr = crate::SharedArray::from(std::slice::from_raw_parts(first_element, count));
    core::ptr::write(out as *mut crate::SharedArray<PathElement>, arr.clone());
}

#[no_mangle]
/// This function is used for the low-level C++ interface to allocate the backing vector for a shared path event array.
pub unsafe extern "C" fn sixtyfps_new_path_events(
    out_events: *mut c_void,
    out_coordinates: *mut c_void,
    first_event: *const PathEvent,
    event_count: usize,
    first_coordinate: *const Point,
    coordinate_count: usize,
) {
    let events = crate::SharedArray::from(std::slice::from_raw_parts(first_event, event_count));
    core::ptr::write(out_events as *mut crate::SharedArray<PathEvent>, events.clone());
    let coordinates =
        crate::SharedArray::from(std::slice::from_raw_parts(first_coordinate, coordinate_count));
    core::ptr::write(out_coordinates as *mut crate::SharedArray<Point>, coordinates.clone());
}

/// Each item return a RenderingPrimitive to the backend with information about what to draw.
#[derive(PartialEq, Debug)]
#[repr(C)]
#[allow(missing_docs)]
pub enum RenderingPrimitive {
    /// There is nothing to draw
    NoContents,
    Rectangle {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    },
    Image {
        x: f32,
        y: f32,
        source: crate::Resource,
    },
    Text {
        x: f32,
        y: f32,
        text: crate::SharedString,
        font_family: crate::SharedString,
        font_pixel_size: f32,
        color: Color,
    },
    Path {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        elements: crate::PathData,
        fill_color: Color,
        stroke_color: Color,
        stroke_width: f32,
    },
}

/// The type of a MouseEvent
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum MouseEventType {
    /// The mouse was pressed
    MousePressed,
    /// The mouse was relased
    MouseReleased,
    /// The mouse position has changed
    MouseMoved,
}

/// Structur representing a mouse event
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MouseEvent {
    /// The position of the cursor
    pub pos: Point,
    /// The action performed (pressed/released/moced)
    pub what: MouseEventType,
}

#[repr(C)]
#[derive(Default)]
/// WindowProperties is used to pass the references to properties of the instantiated
/// component that the run-time will keep up-to-date.
pub struct WindowProperties<'a> {
    /// A reference to the property that is supposed to be kept up-to-date with the width
    /// of the window.
    pub width: Option<&'a crate::Property<f32>>,
    /// A reference to the property that is supposed to be kept up-to-date with the height
    /// of the window.
    pub height: Option<&'a crate::Property<f32>>,

    /// A reference to the property that is supposed to be kept up-to-date with the current
    /// screen dpi
    pub dpi: Option<&'a crate::Property<f32>>,
}

/// The ComponentWindow is the (rust) facing public type that can render the items
/// of components to the screen.
#[repr(C)]
#[derive(Clone)]
pub struct ComponentWindow(std::rc::Rc<dyn crate::eventloop::GenericWindow>);

impl ComponentWindow {
    /// Creates a new instance of a CompomentWindow based on the given window implementation. Only used
    /// internally.
    pub fn new(window_impl: std::rc::Rc<dyn crate::eventloop::GenericWindow>) -> Self {
        Self(window_impl)
    }
    /// Spins an event loop and renders the items of the provided component in this window.
    pub fn run(&self, component: Pin<VRef<ComponentVTable>>, props: &WindowProperties) {
        let event_loop = crate::eventloop::EventLoop::new();
        self.0.clone().map_window(&event_loop);

        event_loop.run(component, &props);
    }
}

#[allow(non_camel_case_types)]
type c_void = ();

/// Same layout as ComponentWindow (fat pointer)
#[repr(C)]
pub struct ComponentWindowOpaque(*const c_void, *const c_void);

/// Releases the reference to the component window held by handle.
#[no_mangle]
pub unsafe extern "C" fn sixtyfps_component_window_drop(handle: *mut ComponentWindowOpaque) {
    assert_eq!(
        core::mem::size_of::<ComponentWindow>(),
        core::mem::size_of::<ComponentWindowOpaque>()
    );
    core::ptr::read(handle as *mut ComponentWindow);
}

/// Spins an event loop and renders the items of the provided component in this window.
#[no_mangle]
pub unsafe extern "C" fn sixtyfps_component_window_run(
    handle: *mut ComponentWindowOpaque,
    component: Pin<VRef<ComponentVTable>>,
    window_props: *mut WindowProperties,
) {
    let window = &*(handle as *const ComponentWindow);
    let window_props = &*(window_props as *const WindowProperties);
    window.run(component, &window_props);
}

#[repr(C)]
#[vtable]
/// Object to be passed in visit_item_children method of the Component.
pub struct ItemVisitorVTable {
    /// Called for each children of the visited item
    ///
    /// The `component` parameter is the component in which the item live which might not be the same
    /// as the parent's component.
    /// `index` is to be used again in the visit_item_children function of the Component (the one passed as parameter)
    /// and `item` is a reference to the item itself
    visit_item: fn(
        VRefMut<ItemVisitorVTable>,
        component: Pin<VRef<ComponentVTable>>,
        index: isize,
        item: Pin<VRef<ItemVTable>>,
    ),
    /// Destructor
    drop: fn(VRefMut<ItemVisitorVTable>),
}

impl<T: FnMut(crate::ComponentRefPin, isize, Pin<ItemRef>)> ItemVisitor for T {
    fn visit_item(&mut self, component: crate::ComponentRefPin, index: isize, item: Pin<ItemRef>) {
        self(component, index, item)
    }
}

/// Expose `crate::item_tree::visit_item_tree` to C++
///
/// Safety: Assume a correct implementation of the item_tree array
#[no_mangle]
pub unsafe extern "C" fn sixtyfps_visit_item_tree(
    component: Pin<VRef<ComponentVTable>>,
    item_tree: Slice<ItemTreeNode<u8>>,
    index: isize,
    visitor: VRefMut<ItemVisitorVTable>,
    visit_dynamic: extern "C" fn(
        base: &u8,
        visitor: vtable::VRefMut<ItemVisitorVTable>,
        dyn_index: usize,
    ),
) {
    crate::item_tree::visit_item_tree(
        Pin::new_unchecked(&*(component.as_ptr() as *const u8)),
        component,
        item_tree.as_slice(),
        index,
        visitor,
        |a, b, c| visit_dynamic(a.get_ref(), b, c),
    )
}

// This is here because for some reason (rust bug?) the ItemVTable_static is not accessible in the other modules

ItemVTable_static! {
    /// The VTable for `Image`
    #[no_mangle]
    pub static ImageVTable for crate::abi::primitives::Image
}
ItemVTable_static! {
    /// The VTable for `Rectangle`
    #[no_mangle]
    pub static RectangleVTable for crate::abi::primitives::Rectangle
}
ItemVTable_static! {
    /// The VTable for `Text`
    #[no_mangle]
    pub static TextVTable for crate::abi::primitives::Text
}
ItemVTable_static! {
    /// The VTable for `TouchArea`
    #[no_mangle]
    pub static TouchAreaVTable for crate::abi::primitives::TouchArea
}
ItemVTable_static! {
    /// The VTable for `Path`
    #[no_mangle]
    pub static PathVTable for crate::abi::primitives::Path
}
