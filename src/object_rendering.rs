//! Copyright 2024 - The Open-Agriculture Developers
//! SPDX-License-Identifier: GPL-3.0-or-later
//! Authors: Daan Steenbergen

use earcutr::earcut;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::ops::Sub;

use ag_iso_stack::object_pool::object::*;
use ag_iso_stack::object_pool::object_attributes::ButtonState;
use ag_iso_stack::object_pool::object_attributes::FontSize;
use ag_iso_stack::object_pool::object_attributes::FormatType;
use ag_iso_stack::object_pool::object_attributes::GrowDirection;
use ag_iso_stack::object_pool::object_attributes::HorizontalAlignment;
use ag_iso_stack::object_pool::object_attributes::LineDirection;
use ag_iso_stack::object_pool::object_attributes::PictureGraphicFormat;
use ag_iso_stack::object_pool::object_attributes::Point;
use ag_iso_stack::object_pool::object_attributes::VerticalAlignment;
use ag_iso_stack::object_pool::object_attributes::{AxisOrientation, BarGraphType};
use ag_iso_stack::object_pool::vt_version::VtVersion;
use ag_iso_stack::object_pool::Colour;
use ag_iso_stack::object_pool::ObjectId;
use ag_iso_stack::object_pool::ObjectPool;
use ag_iso_stack::object_pool::ObjectRef;
use eframe::egui;
use eframe::egui::Color32;
use eframe::egui::ColorImage;
use eframe::egui::FontId;
use eframe::egui::TextWrapMode;
use eframe::egui::TextureHandle;
use eframe::egui::TextureId;
use eframe::egui::UiBuilder;
use egui::epaint::color;

pub trait RenderableObject {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>);
}

impl RenderableObject for Object {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>) {
        // Make sure text is truncated if it doesn't fit for all object renderings (useful for error labels)
        ui.style_mut().wrap_mode = Some(TextWrapMode::Truncate);

        match self {
            Object::WorkingSet(o) => o.render(ui, pool, position),
            Object::DataMask(o) => o.render(ui, pool, position),
            Object::AlarmMask(o) => o.render(ui, pool, position),
            Object::Container(o) => o.render(ui, pool, position),
            Object::SoftKeyMask(o) => o.render(ui, pool, position),
            Object::Key(o) => o.render(ui, pool, position),
            Object::Button(o) => o.render(ui, pool, position),
            Object::InputBoolean(o) => o.render(ui, pool, position),
            Object::InputString(o) => o.render(ui, pool, position),
            Object::InputNumber(o) => o.render(ui, pool, position),
            Object::InputList(o) => o.render(ui, pool, position),
            Object::OutputString(o) => o.render(ui, pool, position),
            Object::OutputNumber(o) => o.render(ui, pool, position),
            Object::OutputList(o) => o.render(ui, pool, position),
            Object::OutputLine(o) => o.render(ui, pool, position),
            Object::OutputRectangle(o) => o.render(ui, pool, position),
            Object::OutputEllipse(o) => o.render(ui, pool, position),
            Object::OutputPolygon(o) => o.render(ui, pool, position),
            Object::OutputMeter(o) => o.render(ui, pool, position),
            Object::OutputLinearBarGraph(o) => o.render(ui, pool, position),
            Object::OutputArchedBarGraph(o) => o.render(ui, pool, position),
            Object::PictureGraphic(o) => o.render(ui, pool, position),
            Object::NumberVariable(_) => (),
            Object::StringVariable(_) => (),
            Object::FontAttributes(_) => (),
            Object::LineAttributes(_) => (),
            Object::FillAttributes(_) => (),
            Object::InputAttributes(_) => (),
            Object::ObjectPointer(o) => o.render(ui, pool, position),
            Object::Macro(o) => (),
            Object::AuxiliaryFunctionType1(o) => (),
            Object::AuxiliaryInputType1(o) => (),
            Object::AuxiliaryFunctionType2(o) => o.render(ui, pool, position),
            Object::AuxiliaryInputType2(o) => o.render(ui, pool, position),
            Object::AuxiliaryControlDesignatorType2(o) => o.render(ui, pool, position),
            Object::WindowMask(o) => (),
            Object::KeyGroup(o) => (),
            Object::GraphicsContext(o) => (),
            Object::ExtendedInputAttributes(o) => (),
            Object::ColourMap(o) => (),
            Object::ObjectLabelReferenceList(o) => (),
            Object::ExternalObjectDefinition(o) => (),
            Object::ExternalReferenceName(o) => (),
            Object::ExternalObjectPointer(o) => (),
            Object::Animation(o) => (),
            Object::ColourPalette(o) => (),
            Object::GraphicData(o) => (),
            Object::WorkingSetSpecialControls(o) => (),
            Object::ScaledGraphic(o) => (),
        }
    }
}

trait Colorable {
    fn convert(&self) -> egui::Color32;
}

impl Colorable for Colour {
    fn convert(&self) -> egui::Color32 {
        egui::Color32::from_rgb(self.r, self.g, self.b)
    }
}

// Helper function to lighten a color by a certain amount
fn lighten_color(color: egui::Color32, amount: f32) -> egui::Color32 {
    let r = (color.r() as f32 + 255.0 * amount).min(255.0) as u8;
    let g = (color.g() as f32 + 255.0 * amount).min(255.0) as u8;
    let b = (color.b() as f32 + 255.0 * amount).min(255.0) as u8;
    egui::Color32::from_rgb(r, g, b)
}

// Helper function to darken a color by a certain amount
fn darken_color(color: egui::Color32, amount: f32) -> egui::Color32 {
    let r = (color.r() as f32 * (1.0 - amount)).max(0.0) as u8;
    let g = (color.g() as f32 * (1.0 - amount)).max(0.0) as u8;
    let b = (color.b() as f32 * (1.0 - amount)).max(0.0) as u8;
    egui::Color32::from_rgb(r, g, b)
}

fn create_relative_rect(ui: &mut egui::Ui, position: Point<i16>, size: egui::Vec2) -> egui::Rect {
    let width = ui.max_rect().width().sub(position.x as f32).min(size.x);
    let height = ui.max_rect().height().sub(position.y as f32).min(size.y);

    egui::Rect::from_min_size(
        ui.max_rect().min + egui::vec2(position.x as f32, position.y as f32),
        egui::vec2(width, height),
    )
}

fn render_object_refs(ui: &mut egui::Ui, pool: &ObjectPool, object_refs: &Vec<ObjectRef>) {
    for object in object_refs.iter() {
        match pool.object_by_id(object.id) {
            Some(obj) => {
                obj.render(ui, pool, object.offset);
            }
            None => {
                ui.colored_label(Color32::RED, format!("Missing object: {:?}", object));
            }
        }
    }
}

impl RenderableObject for WorkingSet {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, _: Point<i16>) {
        if !self.selectable {
            // The working set is not visible
            return;
        }

        ui.painter().rect_filled(
            ui.available_rect_before_wrap(),
            0.0,
            pool.color_by_index(self.background_colour).convert(),
        );

        render_object_refs(ui, pool, &self.object_refs);
    }
}

impl RenderableObject for DataMask {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, _: Point<i16>) {
        ui.painter().rect_filled(
            ui.available_rect_before_wrap(),
            0.0,
            pool.color_by_index(self.background_colour).convert(),
        );

        render_object_refs(ui, pool, &self.object_refs);
    }
}

impl RenderableObject for AlarmMask {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, _: Point<i16>) {
        ui.painter().rect_filled(
            ui.available_rect_before_wrap(),
            0.0,
            pool.color_by_index(self.background_colour).convert(),
        );

        render_object_refs(ui, pool, &self.object_refs);
    }
}

impl RenderableObject for Container {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>) {
        if self.hidden {
            return;
        }

        let rect = create_relative_rect(
            ui,
            position,
            egui::Vec2::new(self.width() as f32, self.height() as f32),
        );

        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            render_object_refs(ui, pool, &self.object_refs);
        });
    }
}

impl RenderableObject for Button {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>) {
        // Use VT version from settings if available
        let vt_version = ui
            .ctx()
            .data(|data| {
                data.get_temp::<crate::DesignerSettings>(egui::Id::new("designer_settings"))
                    .map(|s| s.vt_version)
            })
            .unwrap_or(VtVersion::Version3);

        let rect = create_relative_rect(
            ui,
            position,
            egui::Vec2::new(self.width() as f32, self.height() as f32),
        );

        let mut no_border = false;
        let mut suppress_border = false;
        let mut transparent_background = false;
        let mut disabled = false;

        if vt_version >= VtVersion::Version4 {
            // The following attributes are only available in VT version 4 and later.
            no_border = self.options.no_border;
            suppress_border = self.options.suppress_border;
            transparent_background = self.options.transparent_background;
            disabled = self.options.disabled;
        }

        // Determine if button is latchable and currently latched (pressed).
        let latchable = self.options.latchable;
        let latched = if latchable {
            self.options.state == ButtonState::Latched
        } else {
            false
        };

        // Compute the face rectangle based on border settings
        // According to the standard:
        // - If no_border = true: Face area = entire area (no border space).
        // - If no_border = false: Face is 8 pixels smaller in width and height.
        //
        // The border is a VT proprietary 8-pixel area, but we must reduce face size accordingly.
        // Let's assume a uniform distribution of that 8-pixel shrinkage (4 pixels on each side).
        const BORDER_WIDTH: f32 = 4.0;
        let face_rect = if no_border {
            rect
        } else {
            // Face is area minus 8 pixels in width and height.
            // We'll just evenly shrink by 4 pixels on each side.
            rect.shrink(BORDER_WIDTH)
        };

        let response = ui.interact(
            face_rect,
            ui.id().with(self.id.value()),
            egui::Sense::click(),
        );

        // Determine the current visual state
        // Priority: latched > pressed > hovered > normal
        let is_pressed_state = latched || (response.is_pointer_button_down_on() && !latchable);
        let is_hovered_state = response.hovered();
        // TODO: better visuals for latched states

        let background_color = if transparent_background {
            egui::Color32::TRANSPARENT
        } else {
            let color = pool.color_by_index(self.background_colour).convert();
            if is_pressed_state {
                darken_color(color, 0.2)
            } else if is_hovered_state {
                lighten_color(color, 0.1)
            } else {
                color
            }
        };

        let border_color = if suppress_border {
            egui::Color32::TRANSPARENT
        } else {
            let color = pool.color_by_index(self.border_colour).convert();
            if is_pressed_state {
                lighten_color(color, 0.1)
            } else if is_hovered_state {
                darken_color(color, 0.05)
            } else {
                color
            }
        };

        if !no_border {
            ui.painter().rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(BORDER_WIDTH, border_color),
                egui::StrokeKind::Inside,
            );
        }

        ui.painter().rect_filled(face_rect, 0.0, background_color);

        // Child objects are clipped to the face area
        ui.scope_builder(UiBuilder::new().max_rect(face_rect), |ui| {
            render_object_refs(ui, pool, &self.object_refs);
        });

        // If disabled, we overlay a semi-transparent gray:
        if disabled {
            ui.painter().rect_filled(
                face_rect,
                0.0,
                egui::Color32::from_rgba_premultiplied(128, 128, 128, 100),
            );
        }
    }
}

impl RenderableObject for InputBoolean {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>) {
        let is_true = if let Some(var_id) = self.variable_reference.0 {
            match pool.object_by_id(var_id) {
                Some(Object::NumberVariable(num_var)) => num_var.value > 0,
                _ => self.value,
            }
        } else {
            self.value
        };

        let side = self.width as f32;
        let rect = create_relative_rect(ui, position, egui::Vec2::new(side, side));

        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            let background_color = pool.color_by_index(self.background_colour).convert();
            ui.painter().rect_filled(rect, 0.0, background_color);

            // If the boolean is true, we display a checkmark in the center
            if is_true {
                let fg_color = match pool.object_by_id(self.foreground_colour) {
                    Some(Object::FontAttributes(font_attr)) => {
                        pool.color_by_index(font_attr.font_colour).convert()
                    }
                    // Fall back if missing or the ID is invalid.
                    _ => egui::Color32::BLACK,
                };

                let font_id = egui::FontId::new(side, egui::FontFamily::Proportional);
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "\u{2714}",
                    font_id,
                    fg_color,
                );
            }

            // If disabled, overlay a semi-transparent layer
            if !self.enabled {
                ui.painter().rect_filled(
                    rect,
                    0.0,
                    egui::Color32::from_rgba_premultiplied(128, 128, 128, 100),
                );
            }
        });
    }
}

impl RenderableObject for InputString {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>) {
        let rect = create_relative_rect(
            ui,
            position,
            egui::Vec2::new(self.width() as f32, self.height() as f32),
        );

        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            ui.colored_label(Color32::RED, "InputString not implemented");
        });
    }
}

impl RenderableObject for InputNumber {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>) {
        let rect = create_relative_rect(
            ui,
            position,
            egui::Vec2::new(self.width as f32, self.height as f32),
        );

        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            // Look up the font attributes. If missing, show an error.
            let font_attributes = match pool.object_by_id(self.font_attributes) {
                Some(Object::FontAttributes(fa)) => fa,
                _ => {
                    ui.colored_label(
                        egui::Color32::RED,
                        format!(
                            "Missing FontAttributes for InputNumber ID {:?}",
                            self.id.value()
                        ),
                    );
                    return;
                }
            };

            // Get the background colour from the pool.
            let background_colour = pool.color_by_index(self.background_colour).convert();
            // Fill the background if the NumberOptions do not specify transparency.
            if !self.options.transparent {
                ui.painter().rect_filled(rect, 0.0, background_colour);
            }

            // Determine the “raw” number value to use: if a variable_reference exists, use the referenced
            // NumberVariable’s value; otherwise use our own value.
            let raw_value: u32 = if let Some(var_id) = self.variable_reference.0 {
                match pool.object_by_id(var_id) {
                    Some(Object::NumberVariable(num_var)) => num_var.value,
                    _ => self.value,
                }
            } else {
                self.value
            };

            // Compute the displayed value using double precision:
            //   displayed_value = (raw_value + offset) * scale
            let mut displayed_value = {
                let float_raw = raw_value as f64;
                let float_offset = self.offset as f64;
                let float_scale = self.scale as f64;
                (float_raw + float_offset) * float_scale
            };

            // Use the number of decimals (up to 7) and the "truncate" flag from NumberOptions
            let decimals = self.nr_of_decimals.min(7);
            let power_of_ten = 10f64.powi(decimals as i32);
            if self.options.truncate {
                displayed_value = (displayed_value * power_of_ten).trunc() / power_of_ten;
            } else {
                displayed_value = (displayed_value * power_of_ten).round() / power_of_ten;
            }

            // If the "display_zero_as_blank" option is set and the computed value is exactly zero, show nothing.
            if self.options.display_zero_as_blank && displayed_value == 0.0 {
                return;
            }

            // Format the number to a string. Use exponential formatting if requested.
            let mut number_string = if self.format == FormatType::Exponential {
                format!("{:.*e}", decimals as usize, displayed_value)
            } else {
                format!("{:.*}", decimals as usize, displayed_value)
            };

            // If the "display_leading_zeros" option is set, try to pad the text on the left with zeros
            // so that it fills (or exceeds) the available field width.
            if self.options.display_leading_zeros {
                let fonts = ui.fonts(|f| f.clone());
                let font_height = match font_attributes.font_size {
                    FontSize::NonProportional(size) => size.height() as f32,
                    FontSize::Proportional(height) => height as f32,
                };
                let font_id = egui::FontId::new(font_height, egui::FontFamily::Proportional);
                let mut zero_padded = number_string.clone();
                let max_loop = 1000; // safety to avoid an infinite loop
                for _ in 0..max_loop {
                    let galley = fonts.layout_no_wrap(
                        zero_padded.clone(),
                        font_id.clone(),
                        pool.color_by_index(font_attributes.font_colour).convert(),
                    );
                    if galley.size().x >= rect.width() {
                        number_string = zero_padded;
                        break;
                    } else {
                        zero_padded.insert(0, '0');
                    }
                }
            }

            // Get the font colour.
            let font_colour = pool.color_by_index(font_attributes.font_colour).convert();

            // Choose the font family and height according to the font size:
            let (font_family, font_height) = match font_attributes.font_size {
                FontSize::NonProportional(npsize) => {
                    (egui::FontFamily::Monospace, npsize.height() as f32)
                }
                FontSize::Proportional(h) => (egui::FontFamily::Proportional, h as f32),
            };
            let font_id = egui::FontId::new(font_height, font_family);

            // Lay out the text.
            let fonts = ui.fonts(|f| f.clone());
            let galley = fonts.layout(
                number_string.clone(),
                font_id.clone(),
                font_colour,
                f32::INFINITY,
            );
            let text_size = galley.size();

            // Compute the text’s paint position according to the horizontal and vertical justification.
            let mut paint_pos = rect.min;
            match self.justification.horizontal {
                HorizontalAlignment::Left => {
                    paint_pos.x = rect.min.x;
                }
                HorizontalAlignment::Middle => {
                    paint_pos.x = rect.center().x - text_size.x * 0.5;
                }
                HorizontalAlignment::Right => {
                    paint_pos.x = rect.max.x - text_size.x;
                }
                HorizontalAlignment::Reserved => {
                    ui.colored_label(
                        egui::Color32::RED,
                        "Invalid horizontal alignment for InputNumber",
                    );
                    return;
                }
            }
            match self.justification.vertical {
                VerticalAlignment::Top => {
                    paint_pos.y = rect.min.y;
                }
                VerticalAlignment::Middle => {
                    paint_pos.y = rect.center().y - text_size.y * 0.5;
                }
                VerticalAlignment::Bottom => {
                    paint_pos.y = rect.max.y - text_size.y;
                }
                VerticalAlignment::Reserved => {
                    ui.colored_label(
                        egui::Color32::RED,
                        "Invalid vertical alignment for InputNumber",
                    );
                    return;
                }
            }

            // Draw the number string.
            ui.painter().galley(paint_pos, galley, font_colour);

            // If the InputNumber object is not enabled (according to its InputNumberOptions),
            // overlay a semi‐transparent gray rectangle.
            if !self.options2.enabled {
                ui.painter().rect_filled(
                    rect,
                    0.0,
                    egui::Color32::from_rgba_premultiplied(128, 128, 128, 100),
                );
            }
        });
    }
}

impl RenderableObject for InputList {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>) {
        let rect = create_relative_rect(
            ui,
            position,
            egui::Vec2::new(self.width as f32, self.height as f32),
        );

        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            // Draw background (light gray if enabled, darker if disabled)
            let bg_color = if self.options.enabled {
                egui::Color32::from_rgb(240, 240, 240)
            } else {
                egui::Color32::from_rgb(200, 200, 200)
            };
            ui.painter().rect_filled(rect, 0.0, bg_color);

            // Draw border
            ui.painter().rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(1.0, egui::Color32::DARK_GRAY),
                egui::StrokeKind::Inside,
            );

            // Calculate row height
            let row_height = 24.0;
            let max_rows = (rect.height() / row_height).floor() as usize;
            let visible_items = self.list_items.iter().take(max_rows);

            // Determine selected index (from variable_reference or value)
            let selected_index = if let Some(var_id) = self.variable_reference.0 {
                match pool.object_by_id(var_id) {
                    Some(Object::NumberVariable(num_var)) => num_var.value as usize,
                    _ => self.value as usize,
                }
            } else {
                self.value as usize
            };

            // Draw each item
            for (i, item_ref) in visible_items.enumerate() {
                let y = rect.top() + i as f32 * row_height;
                let item_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.left(), y),
                    egui::vec2(rect.width(), row_height),
                );

                // Highlight selected
                if i == selected_index {
                    ui.painter().rect_filled(
                        item_rect,
                        0.0,
                        egui::Color32::from_rgb(180, 210, 255),
                    );
                }

                // Get label for item
                if let Some(item_id) = item_ref.0 {
                    match pool.object_by_id(item_id) {
                        Some(Object::Container(container)) => {
                            // Render the contents of the container in the row
                            let child_rect = item_rect.shrink(2.0);
                            ui.scope_builder(UiBuilder::new().max_rect(child_rect), |ui| {
                                for child_ref in &container.object_refs {
                                    if let Some(child_obj) = pool.object_by_id(child_ref.id) {
                                        child_obj.render(ui, pool, Point { x: 0, y: 0 });
                                    }
                                }
                            });
                        }
                        Some(Object::StringVariable(sv)) => {
                            ui.painter().text(
                                item_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                sv.value.clone(),
                                egui::FontId::proportional(16.0),
                                egui::Color32::BLACK,
                            );
                        }
                        Some(Object::OutputString(os)) => {
                            ui.painter().text(
                                item_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                os.value.clone(),
                                egui::FontId::proportional(16.0),
                                egui::Color32::BLACK,
                            );
                        }
                        Some(Object::OutputNumber(on)) => {
                            ui.painter().text(
                                item_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                on.value.to_string(),
                                egui::FontId::proportional(16.0),
                                egui::Color32::BLACK,
                            );
                        }
                        Some(Object::InputString(is)) => {
                            ui.painter().text(
                                item_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                is.value.clone(),
                                egui::FontId::proportional(16.0),
                                egui::Color32::BLACK,
                            );
                        }
                        Some(Object::InputNumber(inum)) => {
                            ui.painter().text(
                                item_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                inum.value.to_string(),
                                egui::FontId::proportional(16.0),
                                egui::Color32::BLACK,
                            );
                        }
                        Some(obj) => {
                            ui.painter().text(
                                item_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                format!("{:?}", obj),
                                egui::FontId::proportional(16.0),
                                egui::Color32::BLACK,
                            );
                        }
                        None => {
                            ui.painter().text(
                                item_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "<missing>",
                                egui::FontId::proportional(16.0),
                                egui::Color32::BLACK,
                            );
                        }
                    }
                } else {
                    ui.painter().text(
                        item_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "<empty>",
                        egui::FontId::proportional(16.0),
                        egui::Color32::GRAY,
                    );
                }

                // Draw separator
                if i < max_rows - 1 {
                    ui.painter().line_segment(
                        [
                            egui::pos2(rect.left(), y + row_height),
                            egui::pos2(rect.right(), y + row_height),
                        ],
                        egui::Stroke::new(1.0, egui::Color32::LIGHT_GRAY),
                    );
                }
            }

            // Disabled overlay
            if !self.options.enabled {
                ui.painter().rect_filled(
                    rect,
                    0.0,
                    egui::Color32::from_rgba_premultiplied(128, 128, 128, 100),
                );
            }
        });
    }
}

impl RenderableObject for Key {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>) {
        let rect = create_relative_rect(ui, position, egui::Vec2::new(80.0, 80.0));

        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            render_object_refs(ui, pool, &self.object_refs);
        });
    }
}

impl RenderableObject for ObjectPointer {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>) {
        if self.value.0.is_none() {
            // No object selected
            return;
        }

        match pool.object_by_id(self.value.0.unwrap()) {
            Some(obj) => {
                obj.render(ui, pool, position);
            }
            None => {
                ui.colored_label(Color32::RED, format!("Missing object: {:?}", self));
            }
        }
    }
}

impl RenderableObject for OutputString {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>) {
        let rect = create_relative_rect(
            ui,
            position,
            egui::Vec2::new(self.width() as f32, self.height() as f32),
        );

        let font_attributes = match pool.object_by_id(self.font_attributes) {
            Some(Object::FontAttributes(f)) => f,
            _ => {
                ui.colored_label(
                    Color32::RED,
                    format!("Missing font attributes: {:?}", self.font_attributes),
                );
                return;
            }
        };
        let background_colour = pool.color_by_index(self.background_colour).convert();

        let transparent = self.options.transparent;
        let auto_wrap = self.options.auto_wrap;

        // TODO: check if VT version is 4 or later, if so implement wrap_on_hyphen
        // let wrap_on_hyphen = self.options.wrap_on_hyphen;
        // Note: wrap_on_hyphen behavior is complex. For simplicity here, we rely on normal word-wrapping
        // from egui and do not implement special hyphenation logic. A more thorough implementation
        // would detect hyphens and possibly treat them as break opportunities.

        // According to the specification, we need to handle control characters (CR, LF) as line breaks.
        // We'll normalize all line endings to '\n'.
        let mut text_value = if let Some(variable_reference_id) = self.variable_reference.into() {
            match pool.object_by_id(variable_reference_id) {
                Some(Object::StringVariable(s)) => s.value.clone(),
                _ => self.value.clone(),
            }
        } else {
            self.value.clone()
        };
        text_value = text_value
            .replace("\r\n", "\n")
            .replace("\n\r", "\n")
            .replace('\r', "\n")
            .replace('\x0a', "\n");

        // Apply space trimming rules based on horizontal justification:
        // - Left justification: no trimming of leading spaces (for the first line), trailing spaces remain as is.
        // - Middle justification: remove leading and trailing spaces on each line.
        // - Right justification: remove trailing spaces on each line.
        let mut lines: Vec<&str> = text_value.split('\n').collect();
        for (line_number, line) in lines.iter_mut().enumerate() {
            match self.justification.horizontal {
                HorizontalAlignment::Left => {
                    // Per ISO rules, if auto-wrapping is enabled, leading spaces on wrapped lines might be removed.
                    if auto_wrap && line_number > 0 {
                        // Remove leading spaces
                        *line = line.trim_start();
                    }
                }
                HorizontalAlignment::Middle => {
                    // Remove both leading and trailing spaces
                    *line = line.trim();
                }
                HorizontalAlignment::Right => {
                    // Remove trailing spaces only
                    *line = line.trim_end();
                }
                HorizontalAlignment::Reserved => {
                    ui.colored_label(
                        Color32::RED,
                        "Configuration incorrect: horizontal alignment is set to Reserved",
                    );
                    return;
                }
            }
        }

        let processed_text = lines.join("\n");

        let font_colour = pool.color_by_index(font_attributes.font_colour).convert();
        let fonts = ui.fonts(|fonts| fonts.clone());
        let font_height;
        let font_family;
        match font_attributes.font_size {
            FontSize::NonProportional(size) => {
                font_family = egui::FontFamily::Monospace;

                // We need to calculate the font height based on the width of a letter in the monospace font.
                let font_size = fonts
                    .layout_no_wrap(
                        "a".into(),
                        FontId::new(size.height() as f32, egui::FontFamily::Monospace),
                        font_colour,
                    )
                    .size();

                font_height = size.height() as f32 * (font_size.x / size.width() as f32);
            }
            FontSize::Proportional(height) => {
                font_height = height as f32;
                font_family = egui::FontFamily::Proportional;
            }
        }
        let wrap_width = if auto_wrap {
            self.width() as f32
        } else {
            f32::INFINITY
        };
        let galley = fonts.layout(
            processed_text,
            FontId::new(font_height, font_family.clone()),
            font_colour,
            wrap_width,
        );

        //let fonts = ui.fonts(|fonts| fonts.clone());

        // For non-proportional fonts, implement manual wrapping if auto_wrap is enabled
        //if let FontSize::NonProportional(size) = font_attributes.font_size {
        //    if !transparent {
        //        let painter = ui.painter();
        //        painter.rect_filled(rect, 0.0, background_colour);
        //    }
        //    let char_width = size.width() as f32;
        //    let char_height = size.height() as f32;
        //    let max_chars_per_line = if auto_wrap {
        //        (self.width() as f32 / char_width).floor().max(1.0) as usize
        //    } else {
        //        usize::MAX
        //    };
        //    // Define font_id for monospace font
        //    let font_id = egui::FontId::new(char_height, egui::FontFamily::Monospace);
        //    // Split into lines, then wrap each line if needed
        //    let mut wrapped_lines = Vec::new();
        //    for line in processed_text.split('\n') {
        //        if auto_wrap && line.chars().count() > max_chars_per_line {
        //            let mut current = String::new();
        //            for ch in line.chars() {
        //                current.push(ch);
        //                if current.chars().count() == max_chars_per_line {
        //                    wrapped_lines.push(current.clone());
        //                    current.clear();
        //                }
        //            }
        //            if !current.is_empty() {
        //                wrapped_lines.push(current);
        //            }
        //        } else {
        //            wrapped_lines.push(line.to_string());
        //        }
        //    }
        //    let total_text_height = char_height * wrapped_lines.len() as f32;
        //    let mut y = match self.justification.vertical {
        //        VerticalAlignment::Top => rect.min.y,
        //        VerticalAlignment::Middle => rect.center().y - (total_text_height * 0.5),
        //        VerticalAlignment::Bottom => rect.max.y - total_text_height,
        //        VerticalAlignment::Reserved => {
        //            ui.colored_label(
        //                Color32::RED,
        //                "Configuration incorrect: vertical alignment is set to Reserved",
        //            );
        //            return;
        //        }
        //    };
        //    for line in &wrapped_lines {
        //        let line_len = line.chars().count() as f32;
        //        let line_width = char_width * line_len;
        //        let x = match self.justification.horizontal {
        //            HorizontalAlignment::Left => rect.min.x,
        //            HorizontalAlignment::Middle => rect.center().x - (line_width * 0.5),
        //            HorizontalAlignment::Right => rect.max.x - line_width,
        //            HorizontalAlignment::Reserved => {
        //                ui.colored_label(
        //                    Color32::RED,
        //                    "Configuration incorrect: horizontal alignment is set to Reserved",
        //                );
        //                return;
        //            }
        //        };
        //        let mut cx = x;
        //        for ch in line.chars() {
        //            let galley = fonts.layout_no_wrap(ch.to_string(), font_id.clone(), font_colour);
        //            let pos = egui::pos2(cx, y);
        //            ui.painter().galley(pos, galley.clone(), font_colour);
        //            // Bold
        //            if font_attributes.font_style.bold {
        //                // Bold: draw the character again with a slight offset to simulate boldness
        //                let bold_offset = 1.0;
        //                let bold_pos = egui::pos2(cx + bold_offset, y);
        //                ui.painter().galley(bold_pos, galley.clone(), font_colour);
        //            }
        //            // Crossed out
        //            if font_attributes.font_style.crossed_out {
        //                let cross_y = pos.y + char_height / 2.0;
        //                ui.painter().line_segment(
        //                    [
        //                        egui::pos2(pos.x, cross_y),
        //                        egui::pos2(pos.x + char_width, cross_y),
        //                    ],
        //                    egui::Stroke::new(1.0, font_colour),
        //                );
        //            }
        //            // Underline
        //            if font_attributes.font_style.underlined {
        //                let underline_y = pos.y + char_height - 2.0;
        //                ui.painter().line_segment(
        //                    [
        //                        egui::pos2(pos.x, underline_y),
        //                        egui::pos2(pos.x + char_width, underline_y),
        //                    ],
        //                    egui::Stroke::new(1.0, font_colour),
        //                );
        //            }
        //            //itallic
        //            if font_attributes.font_style.italic {
        //                // Italic: draw the character with a slight skew to simulate italics
        //                let italic_offset = 2.0;
        //                let italic_pos = egui::pos2(cx + italic_offset, y);
        //                ui.painter().galley(italic_pos, galley.clone(), font_colour);
        //            }
        //            //inverted
        //            if font_attributes.font_style.inverted {
        //                let inverted_rect = egui::Rect::from_min_size(
        //                    egui::pos2(cx, y),
        //                    egui::vec2(char_width, char_height),
        //                );
        //                ui.painter().rect_filled(inverted_rect, 0.0, font_colour);
        //                let galley_inv = fonts.layout_no_wrap(
        //                    ch.to_string(),
        //                    font_id.clone(),
        //                    background_colour,
        //                );
        //                ui.painter()
        //                    .galley(egui::pos2(cx, y), galley_inv, background_colour);
        //            }
        //            cx += char_width;
        //        }
        //        y += char_height;
        //    }
        //} else {
        // Proportional font: use egui's normal layout
        //let font_id = egui::FontId::new(font_height, font_family);
        //let galley = fonts.layout(processed_text, font_id.clone(), font_colour, wrap_width);

        let text_size = galley.size();

        let mut paint_pos = rect.min;

        match self.justification.horizontal {
            HorizontalAlignment::Left => {
                paint_pos.x = rect.min.x;
            }
            HorizontalAlignment::Middle => {
                paint_pos.x = rect.center().x - (text_size.x * 0.5);
            }
            HorizontalAlignment::Right => {
                paint_pos.x = rect.max.x - text_size.x;
            }
            HorizontalAlignment::Reserved => {
                ui.colored_label(
                    Color32::RED,
                    "Configuration incorrect: horizontal alignment is set to Reserved",
                );
                return;
            }
        };

        match self.justification.vertical {
            VerticalAlignment::Top => {
                paint_pos.y = rect.min.y;
            }
            VerticalAlignment::Middle => {
                paint_pos.y = rect.center().y - (text_size.y * 0.5);
            }
            VerticalAlignment::Bottom => {
                paint_pos.y = rect.max.y - text_size.y;
            }
            VerticalAlignment::Reserved => {
                ui.colored_label(
                    Color32::RED,
                    "Configuration incorrect: vertical alignment is set to Reserved",
                );
                return;
            }
        };

        if !transparent {
            let painter = ui.painter();
            painter.rect_filled(rect, 0.0, background_colour);
        }

        ui.painter().galley(paint_pos, galley, font_colour);
        //}
    }
}

impl RenderableObject for OutputNumber {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>) {
        let rect = create_relative_rect(
            ui,
            position,
            egui::Vec2::new(self.width() as f32, self.height() as f32),
        );

        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            // 1. Get the font attributes
            let font_attributes = match pool.object_by_id(self.font_attributes) {
                Some(Object::FontAttributes(fa)) => fa,
                _ => {
                    ui.colored_label(
                        Color32::RED,
                        format!(
                            "Missing FontAttributes for OutputNumber ID {:?}",
                            self.id.value()
                        ),
                    );
                    return;
                }
            };

            // 2. Convert the pool color indices to `egui::Color32`
            let background_colour = pool.color_by_index(self.background_colour).convert();
            let font_colour = pool.color_by_index(font_attributes.font_colour).convert();

            // 3. Determine if we need to fill the background or remain transparent
            if !self.options.transparent {
                ui.painter().rect_filled(rect, 0.0, background_colour);
            }

            // 4. Retrieve the raw value (either from variable_reference or this object’s own `value`)
            let raw_value = if let Some(var_id) = self.variable_reference.into() {
                // If we have a referenced NumberVariable, use it
                match pool.object_by_id(var_id) {
                    Some(Object::NumberVariable(num_var)) => num_var.value,
                    _ => self.value,
                }
            } else {
                self.value
            };

            // 5. Compute the displayed value using double precision to reduce rounding errors
            let mut displayed_value = {
                let float_raw = raw_value as f64;
                let float_offset = self.offset as f64;
                let float_scale = self.scale as f64;
                (float_raw + float_offset) * float_scale
            };

            // 6. Apply truncation or rounding to the number of decimals
            let decimals = self.nr_of_decimals.min(7); // standard says 0–7 decimals
            let power_of_ten = 10f64.powi(decimals as i32);

            if self.options.truncate {
                // Truncate
                displayed_value = (displayed_value * power_of_ten).trunc() / power_of_ten;
            } else {
                // Round
                displayed_value = (displayed_value * power_of_ten).round() / power_of_ten;
            }

            // 7. If "display_zero_as_blank" and the final number is exactly zero, display blank
            //    We interpret "exactly zero" after the rounding/truncation step
            if self.options.display_zero_as_blank && displayed_value == 0.0 {
                return;
            }

            // 8. Convert the (possibly truncated/rounded) displayed_value to string
            //    Depending on the "format" attribute, use decimal or exponential
            let mut number_string = if self.format == FormatType::Exponential {
                format!("{:.*e}", decimals as usize, displayed_value)
            } else {
                format!("{:.*}", decimals as usize, displayed_value)
            };

            // 9. The standard states that we must always display at least one digit
            //    before the decimal point (i.e., "0.xxxx" if the absolute value < 1)
            //    Normal Rust formatting already ensures e.g. "0.12" for 0.12,
            //    so we usually don't need a special patch here. But we keep the note.
            //
            // 10. If display_leading_zeros is set, we *attempt* to fill the entire width
            //     with zeros to the left before applying alignment. (ISO 11783 says
            //     "fill left to width of field with zeros, then apply justification.")
            //     Below is a best-effort approach: we measure the text in a loop,
            //     and keep prepending '0' until it meets or exceeds the available width.
            //     We also place a reasonable safety limit to avoid infinite loops.
            //
            if self.options.display_leading_zeros {
                let fonts = ui.fonts(|f| f.clone());
                let font_height = match font_attributes.font_size {
                    FontSize::NonProportional(s) => s.height() as f32,
                    FontSize::Proportional(h) => h as f32,
                };
                let font_id = egui::FontId::new(font_height, egui::FontFamily::Proportional);
                let mut zero_padded = number_string.clone();
                let max_loop = 1000; // safety net to avoid infinite loops
                for _ in 0..max_loop {
                    // Measure the current galley
                    let galley = fonts.layout(
                        zero_padded.as_str().to_owned(),
                        font_id.clone(),
                        font_colour,
                        f32::INFINITY, // no wrap
                    );
                    if galley.size().x >= rect.width() {
                        // Enough zeros to fill or exceed the field width
                        number_string = zero_padded;
                        break;
                    } else {
                        zero_padded.insert(0, '0');
                    }
                }
            }

            // 11. We have the final text we want to display in `number_string`.
            //     Next, figure out the font size and alignment. This is similar
            //     to the `OutputString` example.
            let fonts = ui.fonts(|fonts| fonts.clone());
            let (font_family, font_height) = match font_attributes.font_size {
                FontSize::NonProportional(npsize) => {
                    // For simplicity, treat it as monospace
                    (egui::FontFamily::Monospace, npsize.height() as f32)
                }
                FontSize::Proportional(h) => (egui::FontFamily::Proportional, h as f32),
            };

            let font_id = egui::FontId::new(font_height, font_family);
            let galley = fonts.layout(
                number_string.clone(),
                font_id.clone(),
                font_colour,
                f32::INFINITY, // no wrapping
            );
            let text_size = galley.size();

            // 12. Determine text anchor point based on the justification bits
            let mut paint_pos = rect.min;
            match self.justification.horizontal {
                HorizontalAlignment::Left => {
                    paint_pos.x = rect.min.x;
                }
                HorizontalAlignment::Middle => {
                    paint_pos.x = rect.center().x - (text_size.x * 0.5);
                }
                HorizontalAlignment::Right => {
                    paint_pos.x = rect.max.x - text_size.x;
                }
                HorizontalAlignment::Reserved => {
                    ui.colored_label(
                        Color32::RED,
                        "Configuration incorrect: horizontal alignment is set to Reserved",
                    );
                    return;
                }
            }
            match self.justification.vertical {
                VerticalAlignment::Top => {
                    paint_pos.y = rect.min.y;
                }
                VerticalAlignment::Middle => {
                    paint_pos.y = rect.center().y - (text_size.y * 0.5);
                }
                VerticalAlignment::Bottom => {
                    paint_pos.y = rect.max.y - text_size.y;
                }
                VerticalAlignment::Reserved => {
                    ui.colored_label(
                        Color32::RED,
                        "Configuration incorrect: vertical alignment is set to Reserved",
                    );
                    return;
                }
            }

            // 13. Finally, paint the text
            ui.painter().galley(paint_pos, galley, font_colour);
        });
    }
}

impl RenderableObject for OutputList {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>) {
        let rect = create_relative_rect(
            ui,
            position,
            egui::Vec2::new(self.width() as f32, self.height() as f32),
        );

        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            ui.colored_label(Color32::RED, "OutputList not implemented");
        });
    }
}

impl RenderableObject for OutputLine {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>) {
        let rect = create_relative_rect(
            ui,
            position,
            egui::Vec2::new(self.width() as f32, self.height() as f32),
        );

        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            let line_attributes = match pool.object_by_id(self.line_attributes) {
                Some(Object::LineAttributes(attr)) => attr,
                _ => {
                    // If we don't have valid line attributes, just show an error and return
                    ui.colored_label(
                        Color32::RED,
                        format!(
                            "Missing or invalid LineAttributes ID: {:?}",
                            self.line_attributes
                        ),
                    );
                    return;
                }
            };

            if line_attributes.line_width == 0 {
                return;
            }

            let colour = pool.color_by_index(line_attributes.line_colour).convert();
            let stroke_width = line_attributes.line_width as f32;
            let stroke = egui::Stroke::new(stroke_width, colour);
            // TODO: implement line art

            let (start, end) = match self.line_direction {
                LineDirection::TopLeftToBottomRight => {
                    let start = rect.min;
                    let mut end = rect.max - egui::vec2(stroke_width, stroke_width);

                    // Clamp end to start
                    if end.x < start.x {
                        end.x = start.x;
                    }
                    if end.y < start.y {
                        end.y = start.y;
                    }

                    (start, end)
                }
                LineDirection::BottomLeftToTopRight => {
                    let mut start = egui::pos2(rect.left(), rect.bottom() + stroke_width);
                    let mut end = egui::pos2(rect.right() - stroke_width, rect.top());

                    // Clamping start and end
                    if end.x < start.x {
                        end.x = start.x;
                    }
                    if start.y < end.y {
                        start.y = end.y;
                    }

                    (start, end)
                }
            };

            ui.painter().line_segment([start, end], stroke);
        });
    }
}

impl RenderableObject for OutputRectangle {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>) {
        let rect = create_relative_rect(
            ui,
            position,
            egui::Vec2::new(self.width() as f32, self.height() as f32),
        );

        // Paint the border of the rectangle
        let line_attributes = match pool.object_by_id(self.line_attributes) {
            Some(Object::LineAttributes(l)) => l,
            _ => {
                ui.colored_label(
                    Color32::RED,
                    format!("Missing line attributes: {:?}", self.line_attributes),
                );
                return;
            }
        };
        // Paint the fill of the rectangle
        if let Some(fill) = self.fill_attributes.into() {
            let fill_attributes = match pool.object_by_id(fill) {
                Some(Object::FillAttributes(f)) => f,
                _ => {
                    ui.colored_label(Color32::RED, format!("Missing fill attributes: {:?}", fill));
                    return;
                }
            };
            ui.painter().rect_filled(
                rect,
                0.0,
                pool.color_by_index(fill_attributes.fill_colour).convert(),
            );
            // TODO: implement fill type for infill
            // TODO: implement fill pattern for infill
        }

        ui.painter().rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(
                line_attributes.line_width,
                pool.color_by_index(line_attributes.line_colour).convert(),
            ),
            egui::StrokeKind::Inside,
        );
        // TODO: implement line art for border
    }
}

impl RenderableObject for OutputEllipse {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>) {
        let rect = create_relative_rect(
            ui,
            position,
            egui::Vec2::new(self.width() as f32, self.height() as f32),
        );

        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            ui.colored_label(Color32::RED, "OutputEllipse not implemented");
        });
    }
}

impl RenderableObject for OutputPolygon {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>) {
        let rect = create_relative_rect(
            ui,
            position,
            egui::Vec2::new(self.width() as f32, self.height() as f32),
        );

        // Get fill color (if any)
        let fill_color = match self.fill_attributes.into() {
            Some(fill_id) => match pool.object_by_id(fill_id) {
                Some(Object::FillAttributes(fill_attr)) => {
                    Some(pool.color_by_index(fill_attr.fill_colour).convert())
                }
                _ => None,
            },
            None => None,
        };

        // Get line color and width
        let (line_color, line_width) = pool
            .line_attributes_object_by_id(self.line_attributes)
            .map(|line_attr| {
                (
                    pool.color_by_index(line_attr.line_colour).convert(),
                    line_attr.line_width as f32,
                )
            })
            .unwrap_or((Color32::BLACK, 1.0));

        // Get the painter before using it
        let painter = ui.painter();

        // Transform points to egui coordinates within rect, preserving aspect ratio, and apply rotation
        let width = self.width.max(1) as f32;
        let height = self.height.max(1) as f32;
        let rect_width = rect.width();
        let rect_height = rect.height();
        let scale_x = rect_width / width;
        let scale_y = rect_height / height;
        let center_x = rect.left() + rect_width / 2.0;
        let center_y = rect.top() + rect_height / 2.0;
        let egui_points: Vec<egui::Pos2> = self
            .points
            .iter()
            .map(|pt| {
                // Center relative
                let px = (pt.x as f32 + 0.5) * scale_x - rect_width / 2.0;
                let py = (pt.y as f32 + 0.5) * scale_y - rect_height / 2.0;
                // No rotation, just transform
                egui::pos2(center_x + px, center_y + py)
            })
            .collect();

        if let Some(mut fill) = fill_color {
            // Force fill color to be fully opaque to avoid visible triangle edges
            fill = egui::Color32::from_rgba_premultiplied(fill.r(), fill.g(), fill.b(), 255);
            if egui_points.len() >= 3 {
                // Simple convexity check: for all triplets, the cross product sign should be the same
                let mut is_convex = true;
                let n = egui_points.len();
                let mut prev = 0.0;
                for i in 0..n {
                    let a = egui_points[i];
                    let b = egui_points[(i + 1) % n];
                    let c = egui_points[(i + 2) % n];
                    let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
                    if i == 0 {
                        prev = cross;
                    } else if cross.signum() != prev.signum() && cross.abs() > f32::EPSILON {
                        is_convex = false;
                        break;
                    }
                }
                if is_convex {
                    painter.add(egui::Shape::convex_polygon(
                        egui_points.clone(),
                        fill,
                        egui::Stroke::NONE,
                    ));
                } else {
                    // Use triangulation for concave polygons
                    let mut coords: Vec<f64> = Vec::with_capacity(egui_points.len() * 2);
                    for pt in &egui_points {
                        coords.push(pt.x as f64);
                        coords.push(pt.y as f64);
                    }
                    if let Ok(indices) = earcut(&coords, &[], 2) {
                        for tri in indices.chunks(3) {
                            if tri.len() == 3 {
                                let a = egui_points[tri[0]];
                                let b = egui_points[tri[1]];
                                let c = egui_points[tri[2]];
                                // Draw triangle edges in fill color to hide seams
                                painter.line_segment([a, b], egui::Stroke::new(1.0, fill));
                                painter.line_segment([b, c], egui::Stroke::new(1.0, fill));
                                painter.line_segment([c, a], egui::Stroke::new(1.0, fill));
                                painter.add(egui::Shape::convex_polygon(
                                    vec![a, b, c],
                                    fill,
                                    egui::Stroke::NONE,
                                ));
                            }
                        }
                    }
                }
            }
        }

        // Draw polygon outline
        if egui_points.len() >= 2 {
            painter.add(egui::Shape::closed_line(
                egui_points.clone(),
                egui::Stroke::new(line_width, line_color),
            ));
        }

        // Debug render: draw a small magenta circle at each polygon point if local UI toggle is enabled
        let debug_points_id = egui::Id::new(format!("polygon_debug_points_{}", self.id.value()));
        let debug_points = ui
            .ctx()
            .data(|data| data.get_temp::<bool>(debug_points_id))
            .unwrap_or(false);
        if debug_points {
            let debug_color = egui::Color32::from_rgb(255, 0, 255);
            let debug_radius = 4.0;
            for (i, pt) in egui_points.iter().enumerate() {
                painter.add(egui::Shape::circle_filled(*pt, debug_radius, debug_color));
                painter.text(
                    *pt,
                    egui::Align2::CENTER_CENTER,
                    format!("{}", i),
                    egui::FontId::monospace(12.0),
                    egui::Color32::WHITE,
                );
            }
        }
    }
}

impl RenderableObject for OutputMeter {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>) {
        let rect = create_relative_rect(
            ui,
            position,
            egui::Vec2::new(self.width() as f32, self.height() as f32),
        );

        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            ui.colored_label(Color32::RED, "OutputMeter not implemented");
        });
    }
}

impl RenderableObject for OutputLinearBarGraph {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>) {
        let rect = create_relative_rect(
            ui,
            position,
            egui::Vec2::new(self.width() as f32, self.height() as f32),
        );

        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            // Draw border if enabled
            if self.options.draw_border {
                let bar_colour = pool.color_by_index(self.colour).convert();
                ui.painter().rect_stroke(
                    rect,
                    0.0,
                    egui::Stroke::new(2.0, bar_colour),
                    egui::StrokeKind::Inside,
                );
            }

            // Calculate bar fill

            // Support dynamic value via variable_reference
            let value = if let Some(var_id) = self.variable_reference.into() {
                match pool.object_by_id(var_id) {
                    Some(Object::NumberVariable(num_var)) => num_var.value as f32,
                    _ => self.value as f32,
                }
            } else {
                self.value as f32
            };
            let min = self.min_value as f32;
            let max = self.max_value as f32;
            let percent: f32 = if max > min {
                ((value - min) / (max - min)).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let bar_colour = pool.color_by_index(self.colour).convert();
            match self.options.axis_orientation {
                AxisOrientation::Horizontal => {
                    let width = rect.width() * percent;
                    let bar_rect = match self.options.grow_direction {
                        GrowDirection::GrowLeftDown => egui::Rect::from_min_size(
                            egui::pos2(rect.right() - width, rect.top()),
                            egui::vec2(width, rect.height()),
                        ),
                        GrowDirection::GrowRightUp => egui::Rect::from_min_size(
                            egui::pos2(rect.left(), rect.top()),
                            egui::vec2(width, rect.height()),
                        ),
                    };
                    match self.options.bar_graph_type {
                        BarGraphType::Filled => {
                            ui.painter().rect_filled(bar_rect, 0.0, bar_colour);
                        }
                        BarGraphType::NotFilled => {
                            ui.painter().rect_stroke(
                                bar_rect,
                                0.0,
                                egui::Stroke::new(2.0, bar_colour),
                                egui::StrokeKind::Inside,
                            );
                        }
                    }
                }
                AxisOrientation::Vertical => {
                    let height = rect.height() * percent;
                    let bar_rect = match self.options.grow_direction {
                        GrowDirection::GrowLeftDown => egui::Rect::from_min_size(
                            egui::pos2(rect.left(), rect.top()),
                            egui::vec2(rect.width(), height),
                        ),
                        GrowDirection::GrowRightUp => egui::Rect::from_min_size(
                            egui::pos2(rect.left(), rect.bottom() - height),
                            egui::vec2(rect.width(), height),
                        ),
                    };
                    match self.options.bar_graph_type {
                        BarGraphType::Filled => {
                            ui.painter().rect_filled(bar_rect, 0.0, bar_colour);
                        }
                        BarGraphType::NotFilled => {
                            ui.painter().rect_stroke(
                                bar_rect,
                                0.0,
                                egui::Stroke::new(2.0, bar_colour),
                                egui::StrokeKind::Inside,
                            );
                        }
                    }
                }
            }

            // Draw ticks if enabled
            if self.options.draw_ticks && self.nr_of_ticks > 0 {
                let tick_colour = pool.color_by_index(self.colour).convert();
                let tick_length = 5.0;
                let painter = ui.painter();

                // Distribute ticks evenly across the range
                let tick_count = self.nr_of_ticks as usize;
                for i in 0..tick_count {
                    let t = (i as f32 + 0.5) / tick_count as f32;

                    match (self.options.axis_orientation, self.options.grow_direction) {
                        (AxisOrientation::Horizontal, GrowDirection::GrowRightUp) => {
                            // Left to right, ticks on top and bottom
                            let x = rect.left() + t * rect.width();
                            painter.line_segment(
                                [
                                    egui::pos2(x, rect.top()),
                                    egui::pos2(x, rect.top() + tick_length),
                                ],
                                egui::Stroke::new(1.0, tick_colour),
                            );
                            painter.line_segment(
                                [
                                    egui::pos2(x, rect.bottom()),
                                    egui::pos2(x, rect.bottom() - tick_length),
                                ],
                                egui::Stroke::new(1.0, tick_colour),
                            );
                        }
                        (AxisOrientation::Horizontal, GrowDirection::GrowLeftDown) => {
                            // Right to left, ticks on top and bottom
                            let x = rect.right() - t * rect.width();
                            painter.line_segment(
                                [
                                    egui::pos2(x, rect.top()),
                                    egui::pos2(x, rect.top() + tick_length),
                                ],
                                egui::Stroke::new(1.0, tick_colour),
                            );
                            painter.line_segment(
                                [
                                    egui::pos2(x, rect.bottom()),
                                    egui::pos2(x, rect.bottom() - tick_length),
                                ],
                                egui::Stroke::new(1.0, tick_colour),
                            );
                        }
                        (AxisOrientation::Vertical, GrowDirection::GrowRightUp) => {
                            // Bottom to top, ticks on left and right
                            let y = rect.bottom() - t * rect.height();
                            painter.line_segment(
                                [
                                    egui::pos2(rect.left(), y),
                                    egui::pos2(rect.left() + tick_length, y),
                                ],
                                egui::Stroke::new(1.0, tick_colour),
                            );
                            painter.line_segment(
                                [
                                    egui::pos2(rect.right(), y),
                                    egui::pos2(rect.right() - tick_length, y),
                                ],
                                egui::Stroke::new(1.0, tick_colour),
                            );
                        }
                        (AxisOrientation::Vertical, GrowDirection::GrowLeftDown) => {
                            // Top to bottom, ticks on left and right
                            let y = rect.top() + t * rect.height();
                            painter.line_segment(
                                [
                                    egui::pos2(rect.left(), y),
                                    egui::pos2(rect.left() + tick_length, y),
                                ],
                                egui::Stroke::new(1.0, tick_colour),
                            );
                            painter.line_segment(
                                [
                                    egui::pos2(rect.right(), y),
                                    egui::pos2(rect.right() - tick_length, y),
                                ],
                                egui::Stroke::new(1.0, tick_colour),
                            );
                        }
                    }
                }
            }

            // Draw target line if enabled
            if self.options.draw_target_line {
                let target_line_colour = pool.color_by_index(self.target_line_colour).convert();
                let target_val = if let Some(var_id) = self.target_value_variable_reference.into() {
                    match pool.object_by_id(var_id) {
                        Some(Object::NumberVariable(num_var)) => num_var.value as f32,
                        _ => self.target_value as f32,
                    }
                } else {
                    self.target_value as f32
                };
                let t = if max > min {
                    ((target_val - min) / (max - min)).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let painter = ui.painter();
                match (self.options.axis_orientation, self.options.grow_direction) {
                    (AxisOrientation::Horizontal, GrowDirection::GrowRightUp) => {
                        // Left to right
                        let x = rect.left() + t * rect.width();
                        painter.line_segment(
                            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                            egui::Stroke::new(2.0, target_line_colour),
                        );
                    }
                    (AxisOrientation::Horizontal, GrowDirection::GrowLeftDown) => {
                        // Right to left
                        let x = rect.right() - t * rect.width();
                        painter.line_segment(
                            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                            egui::Stroke::new(2.0, target_line_colour),
                        );
                    }
                    (AxisOrientation::Vertical, GrowDirection::GrowRightUp) => {
                        // Bottom to top
                        let y = rect.bottom() - t * rect.height();
                        painter.line_segment(
                            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                            egui::Stroke::new(2.0, target_line_colour),
                        );
                    }
                    (AxisOrientation::Vertical, GrowDirection::GrowLeftDown) => {
                        // Top to bottom
                        let y = rect.top() + t * rect.height();
                        painter.line_segment(
                            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                            egui::Stroke::new(2.0, target_line_colour),
                        );
                    }
                }
            }
        });
    }
}

impl RenderableObject for OutputArchedBarGraph {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>) {
        let rect = create_relative_rect(
            ui,
            position,
            egui::Vec2::new(self.width() as f32, self.height() as f32),
        );

        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            ui.colored_label(Color32::RED, "OutputArchedBarGraph not implemented");
        });
    }
}

impl RenderableObject for PictureGraphic {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>) {
        let rect = create_relative_rect(
            ui,
            position,
            egui::Vec2::new(self.width() as f32, self.height() as f32),
        );

        let mut hasher = DefaultHasher::new();
        Object::PictureGraphic(self.clone())
            .write()
            .hash(&mut hasher);
        let hash = hasher.finish();

        let changed: bool = ui.data_mut(|data| {
            let old_hash: Option<u64> =
                data.get_temp(format!("picturegraphic_{}_image", self.id.value()).into());
            if old_hash.is_none() || old_hash.unwrap() != hash {
                data.insert_temp(
                    format!("picturegraphic_{}_image", self.id.value()).into(),
                    hash,
                );
                true
            } else {
                false
            }
        });

        let texture_id: Option<TextureId>;
        if changed {
            let mut x = 0;
            let mut y = 0;

            let mut image = ColorImage::filled(
                [self.actual_width.into(), self.actual_height.into()],
                Color32::TRANSPARENT,
            );

            for raw in self.data_as_raw_encoded() {
                let mut colors: Vec<Color32> = vec![];
                match self.format {
                    PictureGraphicFormat::Monochrome => {
                        for bit in 0..8 {
                            colors.push(pool.color_by_index((raw >> (7 - bit)) & 0x01).convert());
                        }
                    }
                    PictureGraphicFormat::FourBit => {
                        for segment in 0..2 {
                            let shift = 4 - (segment * 4);
                            colors.push(pool.color_by_index((raw >> shift) & 0x0F).convert());
                        }
                    }
                    PictureGraphicFormat::EightBit => {
                        colors.push(pool.color_by_index(raw).convert());
                    }
                }

                for color in colors {
                    let idx = y as usize * self.actual_width as usize + x as usize;
                    if idx >= image.pixels.len() {
                        break;
                    }
                    if !(self.options.transparent
                        && color == pool.color_by_index(self.transparency_colour).convert())
                    {
                        image.pixels[idx] = color;
                    }

                    x += 1;
                    if x >= self.actual_width {
                        x = 0;
                        y += 1;
                        // If we go onto the next row, then we discard the rest of the bits
                        break;
                    }
                }
            }

            let new_texture = ui.ctx().load_texture(
                format!("picturegraphic_{}_texture", self.id.value()).as_str(),
                image,
                Default::default(),
            );
            texture_id = Some(new_texture.id());
            ui.data_mut(|data| {
                println!("Saving texture - {:?}", self.id.value());
                data.insert_temp(
                    format!("picturegraphic_{}_texture", self.id.value()).into(),
                    new_texture,
                );
            });
        } else {
            texture_id = ui.data(|data| {
                data.get_temp::<TextureHandle>(
                    format!("picturegraphic_{}_texture", self.id.value()).into(),
                )
                .map(|t| t.id())
            });
        }

        // Use image dimensions, but clip to the available rect
        let image_size = egui::Vec2::new(self.width as f32, self.height() as f32);
        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            if let Some(texture_id) = texture_id {
                ui.image((texture_id, image_size));
            } else {
                ui.colored_label(Color32::RED, "Failed to load image");
            }
        });
    }
}

impl RenderableObject for AuxiliaryFunctionType2 {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>) {
        // Draw a simple filled rectangle with the background colour
        let rect = create_relative_rect(
            ui,
            position,
            egui::Vec2::new(60.0, 60.0), // Default size for AUX2, adjust as needed
        );
        let palette = pool.get_colour_palette();
        let colour = palette[self.background_colour as usize];
        ui.painter().rect_filled(rect, 4.0, colour.convert());

        // Show function_attributes as debug text

        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.label(format!("AUX2\n{:?}", self.function_attributes));
            });
        });

        // Optionally render child object refs (icons, etc.)
        render_object_refs(ui, pool, &self.object_refs);
    }
}

impl RenderableObject for AuxiliaryInputType2 {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>) {
        let rect = create_relative_rect(
            ui,
            position,
            egui::Vec2::new(60.0, 60.0), // Default size for AUX2 Input, adjust as needed
        );
        let palette = pool.get_colour_palette();
        let colour = palette[self.background_colour as usize];
        ui.painter().rect_filled(rect, 4.0, colour.convert());

        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.label(format!("AUX2 In\n{:?}", self.function_attributes));
            });
        });

        render_object_refs(ui, pool, &self.object_refs);
    }
}

impl RenderableObject for AuxiliaryControlDesignatorType2 {
    fn render(&self, ui: &mut egui::Ui, _pool: &ObjectPool, _position: Point<i16>) {
        ui.colored_label(
            Color32::RED,
            "AuxiliaryControlDesignatorType2 not implemented",
        );
    }
}

impl RenderableObject for SoftKeyMask {
    fn render(&self, ui: &mut egui::Ui, pool: &ObjectPool, position: Point<i16>) {
        // Get orientation and key_order from settings if available, else use defaults
        let (orientation, key_order) = ui.ctx().data(|data| {
            if let Some(settings) =
                data.get_temp::<crate::DesignerSettings>(egui::Id::new("designer_settings"))
            {
                (
                    settings.softkey_mask_orientation,
                    settings.softkey_mask_key_order,
                )
            } else {
                (SoftKeyMaskOrientation::RightRight, SoftKeyOrder::TopRight)
            }
        });

        let (key_width, key_height) = ui.ctx().data(|data| {
            if let Some(settings) =
                data.get_temp::<crate::DesignerSettings>(egui::Id::new("designer_settings"))
            {
                (
                    settings.softkey_key_width as f32,
                    settings.softkey_key_height as f32,
                )
            } else {
                (80.0, 80.0)
            }
        });
        let (columns, rows, slot_count) = match orientation {
            SoftKeyMaskOrientation::RightRight => (2, 6, 12),
            SoftKeyMaskOrientation::LeftLeft => (2, 6, 12),
            SoftKeyMaskOrientation::TopTop => (6, 2, 12),
            SoftKeyMaskOrientation::BottomBottom => (6, 2, 12),
            _ => (1, 6, 6),
        };
        let width = columns as f32 * key_width;
        let height = rows as f32 * key_height;
        let bg_color = pool.color_by_index(self.background_colour).convert();
        let desired_size = egui::vec2(width, height);
        let rect = create_relative_rect(ui, position, desired_size);

        // Add spacing between keys (horizontal and vertical)
        let h_spacing: f32 = 0.0;
        let v_spacing: f32 = 0.0;

        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            ui.painter().rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(4.0, Color32::PURPLE),
                egui::StrokeKind::Inside,
            );
        });

        // Helper: get slot (col, row) for a given key index, respecting key_order
        fn key_slot_for_index(
            idx: usize,
            orientation: &SoftKeyMaskOrientation,
            key_order: &SoftKeyOrder,
        ) -> (usize, usize) {
            match orientation {
                SoftKeyMaskOrientation::RightRight | SoftKeyMaskOrientation::LeftLeft => {
                    match key_order {
                        SoftKeyOrder::TopRight => {
                            if idx < 6 {
                                (1, idx) // right column, top to bottom
                            } else {
                                (0, idx - 6) // left column, top to bottom
                            }
                        }
                        SoftKeyOrder::BottomRight => {
                            if idx < 6 {
                                (1, 5 - idx) // right column, bottom to top
                            } else {
                                (0, 5 - (idx - 6)) // left column, bottom to top
                            }
                        }
                        SoftKeyOrder::TopLeft => {
                            if idx < 6 {
                                (0, idx) // left column, top to bottom
                            } else {
                                (1, idx - 6) // right column, top to bottom
                            }
                        }
                        SoftKeyOrder::BottomLeft => {
                            if idx < 6 {
                                (0, 5 - idx) // left column, bottom to top
                            } else {
                                (1, 5 - (idx - 6)) // right column, bottom to top
                            }
                        }
                    }
                }
                SoftKeyMaskOrientation::TopTop | SoftKeyMaskOrientation::BottomBottom => {
                    match key_order {
                        SoftKeyOrder::TopRight => (idx % 6, idx / 6),
                        SoftKeyOrder::BottomRight => (idx % 6, 1 - (idx / 6)),
                        SoftKeyOrder::TopLeft => (5 - (idx % 6), idx / 6),
                        SoftKeyOrder::BottomLeft => (5 - (idx % 6), 1 - (idx / 6)),
                    }
                }
                _ => (0, idx),
            }
        }

        // Draw slot grid and render keys
        for vis_idx in 0..slot_count {
            let obj_id = self.objects.get(vis_idx).cloned().unwrap_or_default();
            let (col, row) = key_slot_for_index(vis_idx, &orientation, &key_order);
            let slot_x = rect.left() as f32 + (col as f32 * (key_width + h_spacing));
            let slot_y = rect.top() as f32 + (row as f32 * (key_height + v_spacing));
            let slot_rect = egui::Rect::from_min_size(
                egui::pos2(slot_x, slot_y),
                egui::vec2(key_width, key_height),
            );
            // Draw slot border
            ui.painter().rect_stroke(
                slot_rect,
                0.0,
                egui::Stroke::new(2.0, Color32::BLUE),
                egui::StrokeKind::Inside,
            );

            // Render key or show empty/missing
            if let Some(id) = obj_id.0 {
                if let Some(obj) = pool.object_by_id(id) {
                    // Use slot-local scope so child is clipped to slot
                    ui.scope_builder(UiBuilder::new().max_rect(slot_rect), |ui| {
                        obj.render(
                            ui,
                            pool,
                            Point { x: 0, y: 0 }, // relative to slot
                        );
                    });
                } else {
                    ui.painter().text(
                        slot_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("Missing key: {:?}", obj_id),
                        egui::FontId::default(),
                        Color32::RED,
                    );
                }
            } else {
                ui.painter().text(
                    slot_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("Empty slot {}", vis_idx + 1),
                    egui::FontId::default(),
                    Color32::GRAY,
                );
            }
        }
    }
}

impl RenderableObject for WindowMask {
    fn render(&self, ui: &mut egui::Ui, _pool: &ObjectPool, _position: Point<i16>) {
        ui.colored_label(Color32::RED, "WindowMask not implemented");
    }
}

impl RenderableObject for KeyGroup {
    fn render(&self, ui: &mut egui::Ui, _pool: &ObjectPool, _position: Point<i16>) {
        ui.colored_label(Color32::RED, "KeyGroup not implemented");
    }
}

impl RenderableObject for GraphicsContext {
    fn render(&self, ui: &mut egui::Ui, _pool: &ObjectPool, _position: Point<i16>) {
        ui.colored_label(Color32::RED, "GraphicsContext not implemented");
    }
}

impl RenderableObject for ExtendedInputAttributes {
    fn render(&self, ui: &mut egui::Ui, _pool: &ObjectPool, _position: Point<i16>) {
        ui.colored_label(Color32::RED, "ExtendedInputAttributes not implemented");
    }
}

impl RenderableObject for ColourMap {
    fn render(&self, ui: &mut egui::Ui, _pool: &ObjectPool, _position: Point<i16>) {
        ui.colored_label(Color32::RED, "ColourMap not implemented");
    }
}

impl RenderableObject for ObjectLabelReferenceList {
    fn render(&self, ui: &mut egui::Ui, _pool: &ObjectPool, _position: Point<i16>) {
        ui.colored_label(Color32::RED, "ObjectLabelReferenceList not implemented");
    }
}

impl RenderableObject for ExternalObjectDefinition {
    fn render(&self, ui: &mut egui::Ui, _pool: &ObjectPool, _position: Point<i16>) {
        ui.colored_label(Color32::RED, "ExternalObjectDefinition not implemented");
    }
}

impl RenderableObject for ExternalReferenceName {
    fn render(&self, ui: &mut egui::Ui, _pool: &ObjectPool, _position: Point<i16>) {
        ui.colored_label(Color32::RED, "ExternalReferenceName not implemented");
    }
}

impl RenderableObject for ExternalObjectPointer {
    fn render(&self, ui: &mut egui::Ui, _pool: &ObjectPool, _position: Point<i16>) {
        ui.colored_label(Color32::RED, "ExternalObjectPointer not implemented");
    }
}

impl RenderableObject for Animation {
    fn render(&self, ui: &mut egui::Ui, _pool: &ObjectPool, _position: Point<i16>) {
        ui.colored_label(Color32::RED, "Animation not implemented");
    }
}

impl RenderableObject for ColourPalette {
    fn render(&self, ui: &mut egui::Ui, _pool: &ObjectPool, _position: Point<i16>) {
        ui.colored_label(Color32::RED, "ColourPalette not implemented");
    }
}

impl RenderableObject for GraphicData {
    fn render(&self, ui: &mut egui::Ui, _pool: &ObjectPool, _position: Point<i16>) {
        ui.colored_label(Color32::RED, "GraphicData not implemented");
    }
}

impl RenderableObject for WorkingSetSpecialControls {
    fn render(&self, ui: &mut egui::Ui, _pool: &ObjectPool, _position: Point<i16>) {
        ui.colored_label(Color32::RED, "WorkingSetSpecialControls not implemented");
    }
}

impl RenderableObject for ScaledGraphic {
    fn render(&self, ui: &mut egui::Ui, _pool: &ObjectPool, _position: Point<i16>) {
        ui.colored_label(Color32::RED, "ScaledGraphic not implemented");
    }
}

impl RenderableObject for AuxiliaryFunctionType1 {
    fn render(&self, ui: &mut egui::Ui, _pool: &ObjectPool, _position: Point<i16>) {
        ui.colored_label(Color32::RED, "AuxiliaryFunctionType1 not implemented");
    }
}

impl RenderableObject for AuxiliaryInputType1 {
    fn render(&self, ui: &mut egui::Ui, _pool: &ObjectPool, _position: Point<i16>) {
        ui.colored_label(Color32::RED, "AuxiliaryInputType1 not implemented");
    }
}
