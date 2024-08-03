use freya_engine::prelude::*;
use freya_native_core::real_dom::NodeImmutable;
use torin::prelude::LayoutNode;

use super::utils::ElementUtils;
use crate::dom::DioxusNode;
use freya_node_state::{Fill, StyleState};
pub struct SvgElement;

impl ElementUtils for SvgElement {
    fn render(
        self,
        layout_node: &LayoutNode,
        node_ref: &DioxusNode,
        canvas: &Canvas,
        _font_collection: &mut FontCollection,
        font_manager: &FontMgr,
        _default_fonts: &[String],
        scale_factor: f32,
    ) {
        let area = layout_node.visible_area();
        let node_style = &*node_ref.get::<StyleState>().unwrap();
        let area = area.scale(1.0 / scale_factor, 1.0 / scale_factor);
        let x = area.min_x();
        let y = area.min_y();
        if let Some(svg_data) = &node_style.svg_data {
            let svg_dom = svg::Dom::from_bytes(svg_data.as_slice(), font_manager);
            if let Ok(mut svg_dom) = svg_dom {
                canvas.save();

                if scale_factor != 1.0 {
                    canvas.scale((scale_factor, scale_factor));
                }

                canvas.translate((x, y));
                svg_dom.set_container_size((area.width() as i32, area.height() as i32));

                svg_dom.render(canvas);

                if let Some(fill) = node_style.fill.as_ref() {
                    let mut paint = Paint::default();

                    paint.set_anti_alias(true);
                    paint.set_blend_mode(BlendMode::DstOver);

                    match fill {
                        Fill::Color(color) => {
                            paint.set_color(*color);
                            //paint.set_alpha_f(0.1);
                        }
                        Fill::LinearGradient(gradient) => {
                            paint.set_shader(gradient.into_shader(area));
                        }
                    }

                    canvas.draw_paint(&paint);
                }

                canvas.restore();
            }
        }
    }
}
