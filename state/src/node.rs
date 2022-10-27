use std::fmt::Display;
use std::sync::{Arc, Mutex};

use dioxus::prelude::UseRef;
use dioxus_core::AttributeValue;
use dioxus_native_core::node_ref::{AttributeMask, NodeMask, NodeView};
use dioxus_native_core::state::{NodeDepState, ParentDepState, State};
use dioxus_native_core_macro::{sorted_str_slice, State};
use freya_layout_common::{LayoutMemorizer, NodeReferenceLayout};
use skia_safe::textlayout::TextAlign;
use skia_safe::Color;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CalcType {
    Sub,
    Mul,
    Div,
    Add,
    Percentage(f32),
    Manual(f32),
}

impl Display for CalcType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CalcType::Sub => f.write_str("-"),
            CalcType::Mul => f.write_str("*"),
            CalcType::Div => f.write_str("/"),
            CalcType::Add => f.write_str("+"),
            CalcType::Percentage(p) => f.write_fmt(format_args!("{p}%")),
            CalcType::Manual(s) => f.write_fmt(format_args!("{s}")),
        }
    }
}

#[derive(Default, Clone, Debug, PartialEq)]
pub enum SizeMode {
    #[default]
    Auto,
    Calculation(Vec<CalcType>),
    Percentage(f32),
    Manual(f32),
}

impl Display for SizeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SizeMode::Auto => f.write_str("auto"),
            SizeMode::Manual(s) => f.write_fmt(format_args!("{s}")),
            SizeMode::Calculation(calcs) => f.write_fmt(format_args!(
                "calc({})",
                calcs
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<String>>()
                    .join(" ")
            )),
            SizeMode::Percentage(p) => f.write_fmt(format_args!("{p}%")),
        }
    }
}

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub enum DirectionMode {
    #[default]
    Vertical,
    Horizontal,
    Both,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FontStyle {
    pub color: Color,
    pub font_family: String,
    pub font_size: f32,
    pub line_height: f32, // https://developer.mozilla.org/en-US/docs/Web/CSS/line-height,
    pub align: TextAlign,
    pub max_lines: Option<usize>,
    pub font_style: skia_safe::FontStyle,
}

impl Default for FontStyle {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            font_family: "Fira Sans".to_string(),
            font_size: 16.0,
            line_height: 1.2,
            align: TextAlign::default(),
            max_lines: None,
            font_style: skia_safe::FontStyle::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CursorReference {
    pub positions: Arc<Mutex<Option<(f32, f32)>>>,
    pub agent: UnboundedSender<(usize, usize)>,
    pub id: Arc<Mutex<Option<usize>>>,
}

impl PartialEq for CursorReference {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Default, Clone)]
pub struct References {
    pub node_ref: Option<UnboundedSender<NodeReferenceLayout>>,
    pub cursor_ref: Option<CursorReference>,
}

#[derive(Clone, State, Default)]
pub struct NodeState {
    #[parent_dep_state(cursor_settings)]
    pub cursor_settings: CursorSettings,
    #[parent_dep_state(references)]
    pub references: References,
    #[parent_dep_state(size, Arc<Mutex<LayoutManager>>)]
    pub size: Size,
    #[parent_dep_state(scroll, Arc<Mutex<LayoutManager>>)]
    pub scroll: Scroll,
    #[node_dep_state()]
    pub style: Style,
    #[parent_dep_state(font_style)]
    pub font_style: FontStyle,
}

impl NodeState {
    pub fn set_size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }
}

#[derive(Default, Clone)]
pub struct Scroll {
    pub scroll_y: f32,
    pub scroll_x: f32,
    pub id: usize,
}

#[derive(Default, Clone)]
pub struct Size {
    pub width: SizeMode,
    pub height: SizeMode,
    pub min_height: SizeMode,
    pub min_width: SizeMode,
    pub padding: (f32, f32, f32, f32),
    pub direction: DirectionMode,
    pub id: usize,
}

impl Size {
    pub fn expanded() -> Self {
        Self {
            width: SizeMode::Percentage(100.0),
            height: SizeMode::Percentage(100.0),
            min_height: SizeMode::Manual(0.0),
            min_width: SizeMode::Manual(0.0),
            padding: (0.0, 0.0, 0.0, 0.0),
            direction: DirectionMode::Both,
            id: 0,
        }
    }
}

impl ParentDepState for References {
    type Ctx = ();
    type DepState = Self;

    const NODE_MASK: NodeMask =
        NodeMask::new_with_attrs(AttributeMask::Static(&sorted_str_slice!([
            "reference",
            "cursor_reference"
        ])));

    fn reduce<'a>(
        &mut self,
        node: NodeView,
        parent: Option<&'a Self::DepState>,
        _ctx: &Self::Ctx,
    ) -> bool {
        let mut node_ref = None;
        let mut cursor_ref = if let Some(parent) = parent {
            parent.cursor_ref.clone()
        } else {
            None
        };

        for a in node.attributes() {
            match a.name {
                "reference" => {
                    if let AttributeValue::Any(v) = a.value {
                        let r: &UseRef<UnboundedSender<NodeReferenceLayout>> =
                            v.value.downcast_ref().unwrap();
                        node_ref = Some(r.read().clone())
                    }
                }
                "cursor_reference" => {
                    if let AttributeValue::Any(v) = a.value {
                        let r: &UseRef<CursorReference> = v.value.downcast_ref().unwrap();
                        cursor_ref = Some(r.read().clone())
                    }
                }
                _ => {
                    println!("Unsupported attribute <{}>", a.name);
                }
            }
        }

        let changed = false;
        *self = Self {
            node_ref,
            cursor_ref,
        };
        changed
    }
}

/// Font style are inherited by default if not specified otherwise by some of the supported attributes.
impl ParentDepState for FontStyle {
    type Ctx = ();
    type DepState = Self;

    const NODE_MASK: NodeMask =
        NodeMask::new_with_attrs(AttributeMask::Static(&sorted_str_slice!([
            "color",
            "font_size",
            "font_family",
            "line_height",
            "align",
            "max_lines",
            "font_style"
        ])));

    fn reduce<'a>(
        &mut self,
        node: NodeView,
        parent: Option<&'a Self::DepState>,
        _ctx: &Self::Ctx,
    ) -> bool {
        let mut font_style = parent.cloned().unwrap_or_default();

        for attr in node.attributes() {
            match attr.name {
                "color" => {
                    let new_color = parse_color(&attr.value.to_string());
                    if let Some(new_color) = new_color {
                        font_style.color = new_color;
                    }
                }
                "font_family" => {
                    font_style.font_family = attr.value.to_string();
                }
                "font_size" => {
                    if let Ok(font_size) = attr.value.to_string().parse() {
                        font_style.font_size = font_size;
                    }
                }
                "line_height" => {
                    if let Ok(line_height) = attr.value.to_string().parse() {
                        font_style.line_height = line_height;
                    }
                }
                "align" => {
                    font_style.align = parse_text_align(&attr.value.to_string());
                }
                "max_lines" => {
                    if let Ok(max_lines) = attr.value.to_string().parse() {
                        font_style.max_lines = Some(max_lines);
                    }
                }
                "font_style" => {
                    font_style.font_style = parse_font_style(&attr.value.to_string());
                }
                _ => {}
            }
        }
        let changed = &font_style != self;
        *self = font_style;
        changed
    }
}

impl ParentDepState for Size {
    type Ctx = Arc<Mutex<LayoutMemorizer>>;
    type DepState = Self;

    const NODE_MASK: NodeMask =
        NodeMask::new_with_attrs(AttributeMask::Static(&sorted_str_slice!([
            "width",
            "height",
            "min_height",
            "min_width",
            "padding",
            "direction",
        ])))
        .with_text()
        .with_tag();

    fn reduce<'a>(
        &mut self,
        node: NodeView,
        _parent: Option<&'a Self::DepState>,
        ctx: &Self::Ctx,
    ) -> bool {
        let mut width = SizeMode::default();
        let mut height = SizeMode::default();
        let mut min_height = SizeMode::default();
        let mut min_width = SizeMode::default();
        let mut padding = (0.0, 0.0, 0.0, 0.0);
        let mut direction = if let Some("label") = node.tag() {
            DirectionMode::Both
        } else if let Some("paragraph") = node.tag() {
            DirectionMode::Both
        } else if let Some("text") = node.tag() {
            DirectionMode::Both
        } else if node.text().is_some() {
            DirectionMode::Both
        } else {
            DirectionMode::Vertical
        };

        for a in node.attributes() {
            match a.name {
                "width" => {
                    let attr = a.value.to_string();
                    if let Some(new_width) = parse_size(&attr) {
                        width = new_width;
                    }
                }
                "height" => {
                    let attr = a.value.to_string();
                    if let Some(new_height) = parse_size(&attr) {
                        height = new_height;
                    }
                }
                "min_height" => {
                    let attr = a.value.to_string();
                    if let Some(new_min_height) = parse_size(&attr) {
                        min_height = new_min_height;
                    }
                }
                "min_width" => {
                    let attr = a.value.to_string();
                    if let Some(new_min_width) = parse_size(&attr) {
                        min_width = new_min_width;
                    }
                }
                "padding" => {
                    let total_padding: f32 = a.value.to_string().parse().unwrap();
                    let padding_for_side = total_padding / 2.0;
                    padding.0 = padding_for_side;
                    padding.1 = padding_for_side;
                    padding.2 = padding_for_side;
                    padding.3 = padding_for_side;
                }
                "direction" => {
                    direction = if a.value.to_string() == "horizontal" {
                        DirectionMode::Horizontal
                    } else if a.value.to_string() == "both" {
                        DirectionMode::Both
                    } else {
                        DirectionMode::Vertical
                    };
                }
                _ => {
                    println!("Unsupported attribute <{}>", a.name);
                }
            }
        }

        let changed = (width != self.width)
            || (height != self.height)
            || (min_height != self.min_height)
            || (min_width != self.min_width)
            || (padding != self.padding)
            || (direction != self.direction);

        if changed {
            ctx.lock().unwrap().mark_as_dirty(node.id());
        }

        *self = Self {
            width,
            height,
            min_height,
            min_width,
            padding,
            direction,
            id: node.id().0,
        };
        changed
    }
}

// TODO(marc2332) Why use ParentDepState? NodeDepState might make more sense
impl ParentDepState for Scroll {
    type Ctx = Arc<Mutex<LayoutMemorizer>>;
    type DepState = Self;

    const NODE_MASK: NodeMask =
        NodeMask::new_with_attrs(AttributeMask::Static(&sorted_str_slice!([
            "scroll_y", "scroll_x",
        ])))
        .with_text()
        .with_tag();

    fn reduce<'a>(
        &mut self,
        node: NodeView,
        _parent: Option<&'a Self::DepState>,
        ctx: &Self::Ctx,
    ) -> bool {
        let mut scroll_y = 0.0;
        let mut scroll_x = 0.0;

        for attr in node.attributes() {
            match attr.name {
                "scroll_y" => {
                    let scroll: f32 = attr.value.to_string().parse().unwrap();
                    scroll_y = scroll;
                }
                "scroll_x" => {
                    let scroll: f32 = attr.value.to_string().parse().unwrap();
                    scroll_x = scroll;
                }
                _ => {
                    println!("Unsupported attribute <{}>", attr.name);
                }
            }
        }

        let changed = (scroll_x != self.scroll_x) || (scroll_y != self.scroll_y);

        if changed {
            ctx.lock().unwrap().mark_as_dirty(node.id());
        }

        *self = Self {
            scroll_y,
            scroll_x,
            id: node.id().0,
        };
        changed
    }
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct ShadowSettings {
    pub x: f32,
    pub y: f32,
    pub intensity: u8,
    pub size: f32,
    pub color: Color,
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub enum DisplayMode {
    #[default]
    Normal,
    Center,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CursorMode {
    None,
    Editable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CursorSettings {
    pub position: Option<i32>,
    pub color: Color,
    pub mode: CursorMode,
    pub id: Option<usize>,
}

impl Default for CursorSettings {
    fn default() -> Self {
        Self {
            position: None,
            color: Color::WHITE,
            mode: CursorMode::None,
            id: None,
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct Style {
    pub background: Color,
    pub relative_layer: i16,
    pub shadow: ShadowSettings,
    pub radius: f32,
    pub image_data: Option<Vec<u8>>,
    pub svg_data: Option<Vec<u8>>,
    pub display: DisplayMode,
}

impl NodeDepState<()> for Style {
    type Ctx = ();

    const NODE_MASK: NodeMask =
        NodeMask::new_with_attrs(AttributeMask::Static(&sorted_str_slice!([
            "background",
            "layer",
            "shadow",
            "radius",
            "image_data",
            "svg_data",
            "svg_content",
            "display",
        ])));

    fn reduce<'a>(&mut self, node: NodeView, _sibling: (), _ctx: &Self::Ctx) -> bool {
        let mut background = Color::TRANSPARENT;
        let mut relative_layer = 0;
        let mut shadow = ShadowSettings::default();
        let mut radius = 0.0;
        let mut image_data = None;
        let mut svg_data = None;
        let mut display = DisplayMode::Normal;

        for attr in node.attributes() {
            match attr.name {
                "display" => display = parse_display(&attr.value.to_string()),
                "background" => {
                    let new_back = parse_color(&attr.value.to_string());
                    if let Some(new_back) = new_back {
                        background = new_back;
                    }
                }
                "layer" => {
                    let new_relative_layer: Option<i16> = attr.value.to_string().parse().ok();
                    if let Some(new_relative_layer) = new_relative_layer {
                        relative_layer = new_relative_layer;
                    }
                }
                "shadow" => {
                    let new_shadow = parse_shadow(&attr.value.to_string());

                    if let Some(new_shadow) = new_shadow {
                        shadow = new_shadow;
                    }
                }
                "radius" => {
                    let new_radius: Option<f32> = attr.value.to_string().parse().ok();

                    if let Some(new_radius) = new_radius {
                        radius = new_radius;
                    }
                }
                "image_data" => {
                    let bytes = attr.value.as_bytes();
                    image_data = bytes.map(|v| v.to_vec());
                }
                "svg_data" => {
                    let bytes = attr.value.as_bytes();
                    svg_data = bytes.map(|v| v.to_vec());
                }
                "svg_content" => {
                    let text = attr.value.as_text();
                    svg_data = text.map(|v| v.as_bytes().to_vec());
                }
                _ => {
                    println!("Unsupported attribute <{}>", attr.name);
                }
            }
        }

        let changed = (background != self.background)
            || (relative_layer != self.relative_layer)
            || (shadow != self.shadow)
            || (radius != self.radius)
            || (image_data != self.image_data);

        *self = Self {
            background,
            relative_layer,
            shadow,
            radius,
            image_data,
            svg_data,
            display,
        };
        changed
    }
}

impl ParentDepState for CursorSettings {
    type Ctx = ();
    type DepState = Self;

    const NODE_MASK: NodeMask =
        NodeMask::new_with_attrs(AttributeMask::Static(&sorted_str_slice!([
            "cursor_index",
            "cursor_color",
            "cursor_mode",
            "cursor_id",
        ])));

    fn reduce<'a>(
        &mut self,
        node: NodeView,
        parent: Option<&'a Self::DepState>,
        _ctx: &Self::Ctx,
    ) -> bool {
        let mut cursor = parent.cloned().unwrap_or_default();

        for attr in node.attributes() {
            match attr.name {
                "cursor_index" => {
                    let text = attr.value.as_text().unwrap();
                    if text != "none" {
                        let new_cursor_index = text.parse().unwrap();
                        cursor.position = Some(new_cursor_index);
                    }
                }
                "cursor_color" => {
                    let new_cursor_color = parse_color(&attr.value.to_string());
                    if let Some(new_cursor_color) = new_cursor_color {
                        cursor.color = new_cursor_color;
                    }
                }
                "cursor_mode" => {
                    cursor.mode = parse_cursor(&attr.value.to_string());
                }
                "cursor_id" => {
                    let new_cursor_id = attr.value.to_string().parse();
                    if let Ok(new_cursor_id) = new_cursor_id {
                        cursor.id = Some(new_cursor_id);
                    }
                }
                _ => {}
            }
        }
        let changed = &cursor != self;
        *self = cursor;
        changed
    }
}

pub fn parse_display(value: &str) -> DisplayMode {
    match value {
        "center" => DisplayMode::Center,
        _ => DisplayMode::Normal,
    }
}

pub fn parse_shadow(value: &str) -> Option<ShadowSettings> {
    let value = value.to_string();
    let mut shadow_values = value.split_ascii_whitespace();
    Some(ShadowSettings {
        x: shadow_values.next()?.parse().ok()?,
        y: shadow_values.next()?.parse().ok()?,
        intensity: shadow_values.next()?.parse().ok()?,
        size: shadow_values.next()?.parse().ok()?,
        color: parse_color(shadow_values.next()?)?,
    })
}

pub fn parse_rgb(color: &str) -> Option<Color> {
    let color = color.replace("rgb(", "").replace(')', "");
    let mut colors = color.split(',');

    let r = colors.next()?.trim().parse().ok()?;
    let g = colors.next()?.trim().parse().ok()?;
    let b = colors.next()?.trim().parse().ok()?;
    Some(Color::from_rgb(r, g, b))
}

pub fn parse_color(color: &str) -> Option<Color> {
    match color {
        "red" => Some(Color::RED),
        "green" => Some(Color::GREEN),
        "blue" => Some(Color::BLUE),
        "yellow" => Some(Color::YELLOW),
        "black" => Some(Color::BLACK),
        "gray" => Some(Color::GRAY),
        "white" => Some(Color::WHITE),
        _ => parse_rgb(color),
    }
}

pub fn parse_text_align(align: &str) -> TextAlign {
    match align {
        "center" => TextAlign::Center,
        "end" => TextAlign::End,
        "justify" => TextAlign::Justify,
        "left" => TextAlign::Left,
        "right" => TextAlign::Right,
        "start" => TextAlign::Start,
        _ => TextAlign::Left,
    }
}

pub fn parse_size(size: &str) -> Option<SizeMode> {
    if size == "stretch" {
        Some(SizeMode::Percentage(100.0))
    } else if size == "auto" {
        Some(SizeMode::Auto)
    } else if size.contains("calc") {
        Some(SizeMode::Calculation(parse_calc(size)?))
    } else if size.contains('%') {
        Some(SizeMode::Percentage(size.replace('%', "").parse().ok()?))
    } else if size.contains("calc") {
        Some(SizeMode::Calculation(parse_calc(size)?))
    } else {
        Some(SizeMode::Manual(size.parse().ok()?))
    }
}

pub fn parse_calc(mut size: &str) -> Option<Vec<CalcType>> {
    let mut calcs = Vec::new();

    size = size.strip_prefix("calc(")?;
    size = size.strip_suffix(')')?;

    let vals = size.split_whitespace();

    for val in vals {
        if val.contains('%') {
            calcs.push(CalcType::Percentage(val.replace('%', "").parse().ok()?));
        } else if val == "+" {
            calcs.push(CalcType::Add);
        } else if val == "-" {
            calcs.push(CalcType::Sub);
        } else if val == "/" {
            calcs.push(CalcType::Div);
        } else if val == "*" {
            calcs.push(CalcType::Mul);
        } else {
            calcs.push(CalcType::Manual(val.parse().ok()?));
        }
    }

    Some(calcs)
}

fn parse_cursor(cursor: &str) -> CursorMode {
    match cursor {
        "editable" => CursorMode::Editable,
        _ => CursorMode::None,
    }
}

fn parse_font_style(style: &str) -> skia_safe::FontStyle {
    match style {
        "italic" => skia_safe::FontStyle::italic(),
        "bold" => skia_safe::FontStyle::bold(),
        "bold-italic" => skia_safe::FontStyle::bold_italic(),
        _ => skia_safe::FontStyle::default(),
    }
}
