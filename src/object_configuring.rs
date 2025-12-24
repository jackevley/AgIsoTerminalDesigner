//! Copyright 2024 - The Open-Agriculture Developers
//! SPDX-License-Identifier: GPL-3.0-or-later
//! Authors: Daan Steenbergen

use crate::allowed_object_relationships::get_allowed_child_refs;
use crate::allowed_object_relationships::AllowedChildRefs;
use crate::possible_events::PossibleEvents;
use crate::DesignerSettings;
use crate::EditorProject;

use ag_iso_stack::object_pool::object::*;
use ag_iso_stack::object_pool::object_attributes::*;
use ag_iso_stack::object_pool::vt_version::VtVersion;
use ag_iso_stack::object_pool::Colour;
use ag_iso_stack::object_pool::NullableObjectId;
use ag_iso_stack::object_pool::ObjectId;
use ag_iso_stack::object_pool::ObjectPool;
use ag_iso_stack::object_pool::ObjectRef;
use ag_iso_stack::object_pool::ObjectType;
use eframe::egui;
use eframe::egui::TextWrapMode;
use std::collections::HashSet;

/// Check if adding a reference from `parent_id` to `child_id` would create a circular reference
/// Returns true if it would create a cycle (and should be blocked)
fn would_create_circular_reference(
    pool: &ObjectPool,
    parent_id: ObjectId,
    child_id: ObjectId,
) -> bool {
    // If parent and child are the same, it's definitely circular
    if parent_id == child_id {
        return true;
    }

    // Check if child already references parent (directly or indirectly)
    // Use depth-first search to check all descendants of child_id
    let mut visited = HashSet::new();
    let mut stack = vec![child_id];

    while let Some(current_id) = stack.pop() {
        // If we've already visited this object, skip it (prevents infinite loops)
        if !visited.insert(current_id) {
            continue;
        }

        // If we find the parent in the descendants of child, it's circular
        if current_id == parent_id {
            return true;
        }

        // Add all children of current object to the stack
        if let Some(obj) = pool.object_by_id(current_id) {
            for ref_id in obj.referenced_objects() {
                stack.push(ref_id);
            }
        }
    }

    false
}

/// Show a button with the current color; clicking it pops up a palette for selection.
fn color_swatch_selector(
    ui: &mut egui::Ui,
    color_index: &mut u8,
    palette: &[ag_iso_stack::object_pool::colour::Colour; 256],
    label: &str,
) -> bool {
    use egui::Color32;

    let mut changed = false;
    let id = ui.make_persistent_id(format!("color_swatch_selector_{}", label));
    let c = &palette[*color_index as usize];
    let color32 = Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a);
    // Track popup open state manually
    let mut popup_open = ui
        .memory(|mem| mem.data.get_temp::<bool>(id))
        .unwrap_or(false);
    ui.horizontal(|ui| {
        ui.label(label);
        let button = egui::Button::new("")
            .fill(color32)
            .stroke(egui::Stroke::new(2.0, Color32::GRAY))
            .min_size(egui::Vec2::splat(24.0));
        let resp = ui.add(button);
        if resp.clicked() {
            popup_open = !popup_open;
            ui.memory_mut(|mem| mem.data.insert_temp(id, popup_open));
        }
        // Render the popup palette manually
        let mut palette_rect = None;
        if popup_open {
            let pos = resp.rect.left_bottom();
            egui::Area::new(id.with("palette_popup"))
                .fixed_pos(pos)
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        let swatch_size = egui::Vec2::splat(20.0);
                        let swatches_per_row = 16;
                        for row in 0..(palette.len() / swatches_per_row) {
                            ui.horizontal(|ui| {
                                for col in 0..swatches_per_row {
                                    let idx = row * swatches_per_row + col;
                                    let c = &palette[idx];
                                    let color32 =
                                        Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a);
                                    let mut button =
                                        egui::Button::new("").fill(color32).min_size(swatch_size);
                                    if *color_index == idx as u8 {
                                        button =
                                            button.stroke(egui::Stroke::new(2.0, Color32::YELLOW));
                                    }
                                    if ui.add(button).clicked() {
                                        *color_index = idx as u8;
                                        changed = true;
                                        popup_open = false;
                                        ui.memory_mut(|mem| mem.data.insert_temp(id, popup_open));
                                    }
                                }
                            });
                        }
                        // Save the palette rect for outside click detection
                        palette_rect = Some(ui.min_rect());
                    });
                });
        }
        // Close popup if user clicks outside both the button and the palette
        if popup_open && ui.input(|i| i.pointer.any_click()) {
            let pointer_pos = ui.input(|i| i.pointer.interact_pos());
            let button_rect = resp.rect;
            let palette_rect = palette_rect.unwrap_or(egui::Rect::NOTHING);
            if let Some(pos) = pointer_pos {
                if !button_rect.contains(pos) && !palette_rect.contains(pos) {
                    popup_open = false;
                    ui.memory_mut(|mem| mem.data.insert_temp(id, popup_open));
                }
            }
        }
    });
    changed
}

pub trait ConfigurableObject {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        settings: &DesignerSettings,
    );
}

impl ConfigurableObject for Object {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        settings: &DesignerSettings,
    ) {
        // Specific UI settings that are applied to all configuration screens

        // The combination below makes the comboboxes used throughout the configuration UI have minimal width, yet still be able to show the full text
        ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
        ui.style_mut().spacing.combo_width = 0.0;

        match self {
            Object::WorkingSet(o) => o.render_parameters(ui, design, settings),
            Object::DataMask(o) => o.render_parameters(ui, design, settings),
            Object::AlarmMask(o) => o.render_parameters(ui, design, settings),
            Object::Container(o) => o.render_parameters(ui, design, settings),
            Object::SoftKeyMask(o) => o.render_parameters(ui, design, settings),
            Object::Key(o) => o.render_parameters(ui, design, settings),
            Object::Button(o) => o.render_parameters(ui, design, settings),
            Object::InputBoolean(o) => o.render_parameters(ui, design, settings),
            Object::InputString(o) => o.render_parameters(ui, design, settings),
            Object::InputNumber(o) => o.render_parameters(ui, design, settings),
            Object::InputList(o) => o.render_parameters(ui, design, settings),
            Object::OutputString(o) => o.render_parameters(ui, design, settings),
            Object::OutputNumber(o) => o.render_parameters(ui, design, settings),
            Object::OutputList(o) => o.render_parameters(ui, design, settings),
            Object::OutputLine(o) => o.render_parameters(ui, design, settings),
            Object::OutputRectangle(o) => o.render_parameters(ui, design, settings),
            Object::OutputEllipse(o) => o.render_parameters(ui, design, settings),
            Object::OutputPolygon(o) => o.render_parameters(ui, design, settings),
            Object::OutputMeter(o) => o.render_parameters(ui, design, settings),
            Object::OutputLinearBarGraph(o) => o.render_parameters(ui, design, settings),
            Object::OutputArchedBarGraph(o) => o.render_parameters(ui, design, settings),
            Object::PictureGraphic(o) => o.render_parameters(ui, design, settings),
            Object::NumberVariable(o) => o.render_parameters(ui, design, settings),
            Object::StringVariable(o) => o.render_parameters(ui, design, settings),
            Object::FontAttributes(o) => o.render_parameters(ui, design, settings),
            Object::LineAttributes(o) => o.render_parameters(ui, design, settings),
            Object::FillAttributes(o) => o.render_parameters(ui, design, settings),
            Object::InputAttributes(o) => o.render_parameters(ui, design, settings),
            Object::ObjectPointer(o) => o.render_parameters(ui, design, settings),
            Object::Macro(o) => o.render_parameters(ui, design, settings),
            Object::AuxiliaryFunctionType1(_) => (),
            Object::AuxiliaryInputType1(_) => (),
            Object::AuxiliaryFunctionType2(o) => o.render_parameters(ui, design, settings),
            Object::AuxiliaryInputType2(o) => o.render_parameters(ui, design, settings),
            Object::AuxiliaryControlDesignatorType2(o) => o.render_parameters(ui, design, settings),
            Object::WindowMask(_) => (),
            Object::KeyGroup(_) => (),
            Object::GraphicsContext(_) => (),
            Object::ExtendedInputAttributes(_) => (),
            Object::ColourMap(_) => (),
            Object::ObjectLabelReferenceList(_) => (),
            Object::ExternalObjectDefinition(_) => (),
            Object::ExternalReferenceName(_) => (),
            Object::ExternalObjectPointer(_) => (),
            Object::Animation(_) => (),
            Object::ColourPalette(_) => (),
            Object::GraphicData(_) => (),
            Object::WorkingSetSpecialControls(_) => (),
            Object::ScaledGraphic(_) => (),
        }
    }
}

fn render_object_id(ui: &mut egui::Ui, id: &mut ObjectId, design: &EditorProject) {
    let mut current_id = u16::from(*id);

    ui.horizontal(|ui| {
        ui.label("Object ID:");

        let widget = egui::DragValue::new(&mut current_id)
            .speed(1.0)
            .range(0..=65534);
        let resp = ui.add(widget);

        let new_id = ObjectId::new(current_id).unwrap();

        // Check if the new ID is already used by another object (excluding the current object)
        let conflict = design.get_pool().object_by_id(new_id).is_some() && new_id != *id;

        let conflict_storage = ui.id().with("conflict");
        let was_conflict = ui.data(|data| data.get_temp::<u16>(conflict_storage));

        if conflict || was_conflict.is_some_and(|id| id == current_id) {
            ui.colored_label(egui::Color32::RED, "ID already in use!");

            // Save the conflict in storage so it is still displayed next frame
            ui.data_mut(|data| {
                data.insert_temp(conflict_storage, u16::from(*id));
            });
        } else if resp.changed() || was_conflict.is_some_and(|id| id != current_id) {
            // Remove the conflict from storage if we are actively changing the ID,
            // or if the ID has changed (most likely another object is selected)
            ui.data_mut(|data| {
                data.remove_temp::<u16>(conflict_storage);
            });
        }

        if !conflict && resp.changed() {
            design.update_object_id_for_info(*id, new_id);
            *id = new_id;
            design.get_mut_selected().borrow_mut().0 = Some(*id);
        }

        // Add the object type display
        if let Some(obj) = design.get_pool().object_by_id(*id) {
            ui.separator();
            ui.label("Type:");
            ui.label(format!("{:?}", obj.object_type()));
            ui.separator();
            ui.label("Referenced In: ");
            // If get_references_to does not exist, use a fallback or implement it.
            // For example, if you want to find all objects that reference this id:
            egui::ScrollArea::horizontal().show(ui, |ui| {
                let referencing_objects: Vec<&Object> = design
                    .get_pool()
                    .objects()
                    .iter()
                    .filter(|obj| {
                        // Replace this with the actual logic for your object reference
                        // For example, if objects have a method `references_id(id: ObjectId) -> bool`
                        obj.referenced_objects().contains(id)
                    })
                    .collect();
                for ref_obj in referencing_objects {
                    ui.label(format!(
                        "{} ({:?})",
                        design.get_object_info(ref_obj).get_name(ref_obj),
                        ref_obj.object_type()
                    ));
                }
            });
        }
    });
}
/// Like render_object_id_selector, but excludes a specific ObjectId (e.g., the parent container's id)
fn render_object_id_selector_exclude(
    ui: &mut egui::Ui,
    idx: usize,
    design: &EditorProject,
    object_id: &mut ObjectId,
    allowed_child_objects: &[ObjectType],
    exclude_id: ObjectId,
) {
    // Use the existing render_object_id_selector, but filter out exclude_id after selection
    let prev_id = *object_id;
    render_object_id_selector(ui, idx, design, object_id, allowed_child_objects, Some(exclude_id));
    if *object_id == exclude_id {
        *object_id = prev_id; // Prevent selecting the excluded id
    }
}

fn render_object_id_selector(
    ui: &mut egui::Ui,
    idx: usize,
    design: &EditorProject,
    object_id: &mut ObjectId,
    allowed_child_objects: &[ObjectType],
    current_object_id: Option<ObjectId>,
) {
    let pool = design.get_pool();
    // If this is being used for a container, prevent self-selection
    // Try to get the current container's id from object_id (the parent id)
    // This assumes object_id is the id of the container being edited
    let parent_id = *object_id;
    egui::ComboBox::from_id_salt(format!("object_id_selector_{}", idx))
        .selected_text(format!("{:?}", object_id.value()))
        .show_ui(ui, |ui| {
            for potential_child in pool.objects_by_types(allowed_child_objects) {
                let child_id = potential_child.id();

                // Check if this would create a circular reference
                let would_be_circular = if let Some(parent_id) = current_object_id {
                    would_create_circular_reference(pool, parent_id, child_id)
                } else {
                    false
                };

                let object_info = design.get_object_info(potential_child);
                let name = object_info.get_name(potential_child);
                let label = format!(
                    "{:?}: {:?} - {}{}",
                    u16::from(child_id),
                    potential_child.object_type(),
                    name,
                    if would_be_circular {
                        " ⚠ (circular)"
                    } else {
                        ""
                    }
                );

                // Disable selection if it would create a circular reference
                ui.add_enabled_ui(!would_be_circular, |ui| {
                    ui.selectable_value(object_id, child_id, label);
                });
            }
        });
}

fn render_nullable_object_id_selector(
    ui: &mut egui::Ui,
    idx: usize,
    design: &EditorProject,
    object_id: &mut NullableObjectId,
    allowed_child_objects: &[ObjectType],
    current_object_id: Option<ObjectId>,
) {
    let pool = design.get_pool();
    egui::ComboBox::from_id_salt(format!("nullable_object_id_selector_{}", idx))
        .selected_text(
            object_id
                .0
                .map_or("None".to_string(), |id| format!("{:?}", id.value())),
        )
        .show_ui(ui, |ui| {
            ui.selectable_value(object_id, NullableObjectId::NULL, "None");
            for potential_child in pool.objects_by_types(allowed_child_objects) {
                let child_id = potential_child.id();

                // Check if this would create a circular reference
                let would_be_circular = if let Some(parent_id) = current_object_id {
                    would_create_circular_reference(pool, parent_id, child_id)
                } else {
                    false
                };

                let object_info = design.get_object_info(potential_child);
                let name = object_info.get_name(potential_child);
                let label = format!(
                    "{:?}: {:?} - {}{}",
                    u16::from(child_id),
                    potential_child.object_type(),
                    name,
                    if would_be_circular {
                        " ⚠ (circular)"
                    } else {
                        ""
                    }
                );

                // Disable selection if it would create a circular reference
                ui.add_enabled_ui(!would_be_circular, |ui| {
                    ui.selectable_value(object_id, child_id.into(), label);
                });
            }
        });
}

fn render_index_modifiers<T>(ui: &mut egui::Ui, idx: usize, list: &mut Vec<T>) {
    if ui
        .add_enabled(idx > 0, egui::Button::new("\u{23F6}"))
        .on_hover_text("Move up")
        .clicked()
    {
        list.swap(idx, idx - 1);
    }
    if ui
        .add_enabled(idx < list.len() - 1, egui::Button::new("\u{23F7}"))
        .on_hover_text("Move down")
        .clicked()
    {
        list.swap(idx, idx + 1);
    }
    if ui.button("\u{1F5D9}").on_hover_text("Remove").clicked() {
        list.remove(idx);
    }
}

fn render_object_references_list(
    ui: &mut egui::Ui,
    design: &EditorProject,
    width: u16,
    height: u16,
    object_refs: &mut Vec<ObjectRef>,
    allowed_child_objects: &[ObjectType],
    current_object_id: ObjectId,
) {
    egui::Grid::new("object_ref_grid")
        .striped(true)
        .min_col_width(0.0)
        .show(ui, |ui| {
            let mut idx = 0;
            while idx < object_refs.len() {
                let obj_ref = &mut object_refs[idx];
                let obj = design.get_pool().object_by_id(obj_ref.id);

                ui.label(" - ");
                render_object_id_selector(
                    ui,
                    idx,
                    design,
                    &mut obj_ref.id,
                    allowed_child_objects,
                    Some(current_object_id),
                );

                if let Some(obj) = obj {
                    let mut max_x = width as i16;
                    let mut max_y = height as i16;
                    if let Some(sized_obj) = obj.as_sized_object() {
                        max_x -= sized_obj.width() as i16;
                        max_y -= sized_obj.height() as i16;
                    }
                    if ui.link(format!("{:?}", obj.object_type())).clicked() {
                        *design.get_mut_selected().borrow_mut() = obj.id().into();
                    }

                    // Add name column
                    let object_info = design.get_object_info(obj);
                    ui.label(object_info.get_name(obj));

                    ui.add(
                        egui::Slider::new(&mut obj_ref.offset.x, 0..=max_x)
                            .text("X")
                            .drag_value_speed(1.0),
                    );
                    ui.add(
                        egui::Slider::new(&mut obj_ref.offset.y, 0..=max_y)
                            .text("Y")
                            .drag_value_speed(1.0),
                    );
                } else {
                    ui.colored_label(egui::Color32::RED, "Missing object");
                }

                render_index_modifiers(ui, idx, object_refs);

                idx += 1;
                ui.end_row();
            }
        });

    let (new_object_id, _) = render_add_object_id(
        ui,
        design,
        allowed_child_objects,
        false,
        Some(current_object_id),
    );
    if let Some(id) = new_object_id {
        object_refs.push(ObjectRef {
            id,
            offset: Point::default(),
        });
    }
}

fn render_object_id_list(
    ui: &mut egui::Ui,
    design: &EditorProject,
    object_ids: &mut Vec<NullableObjectId>,
    allowed_child_objects: &[ObjectType],
    current_object_id: ObjectId,
) {
    // For SoftKeyMask, always show 64 slots, but page through 12 at a time
    // Don't auto-fill with null values - only show slots as needed
    let slot_count = 64;
    let page_size = 12;

    // Find or create a null ObjectPointer for empty slots
    let null_pointer_id = {
        // First, look for an existing ObjectPointer that points to null
        let existing = design
            .get_pool()
            .objects_by_type(ObjectType::ObjectPointer)
            .iter()
            .find(|op| {
                if let Object::ObjectPointer(obj_ptr) = op {
                    obj_ptr.value.0.is_none()
                } else {
                    false
                }
            })
            .map(|op| op.id());

        if let Some(id) = existing {
            Some(id)
        } else {
            // If no null pointer exists, we need to create one
            // For now, we'll just use the UI to guide the user
            None
        }
    };

    // Use a persistent id for the page state
    let page_id = ui.make_persistent_id("softkey_page");
    let mut page = ui.ctx().data(|d| d.get_temp::<usize>(page_id)).unwrap_or(0);
    let max_page = (slot_count + page_size - 1) / page_size - 1;
    ui.horizontal(|ui| {
        if ui.button("< Prev").clicked() {
            if page > 0 {
                page -= 1;
            }
        }
        ui.label(format!("Softkey Page {}/{}", page + 1, max_page + 1));
        if ui.button("Next >").clicked() {
            if page < max_page {
                page += 1;
            }
        }
    });

    if null_pointer_id.is_none() {
        ui.colored_label(
            egui::Color32::YELLOW,
            "⚠ No null ObjectPointer found. Create one with ID pointing to null to use as empty slot.",
        );
    }

    ui.ctx().data_mut(|d| d.insert_temp(page_id, page));
    let start = page * page_size;
    let end = ((page + 1) * page_size).min(slot_count);
    egui::Grid::new("object_id_grid")
        .striped(true)
        .min_col_width(0.0)
        .show(ui, |ui| {
            for idx in start..end {
                ui.label(format!("Slot {}", idx + 1));

                // Check if this slot exists in the vector
                if idx < object_ids.len() {
                    let mut current_id = object_ids[idx];
                    let obj: Option<&Object> = if let Some(id) = current_id.0 {
                        design.get_pool().object_by_id(id)
                    } else {
                        None
                    };

                    // ComboBox for selecting a key or null ObjectPointer
                    let selected_label = match current_id.0 {
                        None => "Select...".to_string(),
                        Some(id) => {
                            // Check if this is a null ObjectPointer
                            if let Some(ptr_id) = null_pointer_id {
                                if id == ptr_id {
                                    "(Empty - Null Pointer)".to_string()
                                } else {
                                    format!("{:?}", id.value())
                                }
                            } else {
                                format!("{:?}", id.value())
                            }
                        }
                    };
                    egui::ComboBox::from_id_salt(format!("softkey_slot_{}", idx))
                        .selected_text(selected_label)
                        .show_ui(ui, |ui| {
                            // Show "Clear" option
                            if ui
                                .selectable_label(current_id.0.is_none(), "Clear")
                                .clicked()
                            {
                                current_id = NullableObjectId(None);
                            }

                            // Show null ObjectPointer if available
                            if let Some(ptr_id) = null_pointer_id {
                                if ui
                                    .selectable_label(
                                        current_id.0.map_or(false, |id| id == ptr_id),
                                        "(Empty - Null Pointer)",
                                    )
                                    .clicked()
                                {
                                    current_id = NullableObjectId(Some(ptr_id));
                                }
                            }

                            for potential_child in
                                design.get_pool().objects_by_types(allowed_child_objects)
                            {
                                let id_val = potential_child.id().value();
                                let name = design
                                    .get_object_info(potential_child)
                                    .get_name(potential_child);
                                if ui
                                    .selectable_label(
                                        current_id.0.map_or(false, |id| id.value() == id_val),
                                        format!("{:?}: {}", id_val, name),
                                    )
                                    .clicked()
                                {
                                    current_id = NullableObjectId(Some(potential_child.id()));
                                }
                            }
                        });

                    // Update the vector after the closure
                    object_ids[idx] = current_id;

                    if let Some(obj) = obj {
                        if current_id.0.is_some() {
                            if ui.link(format!("{:?}", obj.object_type())).clicked() {
                                *design.get_mut_selected().borrow_mut() = obj.id().into();
                            }
                        } else {
                            ui.label("");
                        }
                        let object_info = design.get_object_info(obj);
                        ui.label(object_info.get_name(obj));
                    } else {
                        ui.colored_label(egui::Color32::GRAY, "Empty");
                        ui.label("");
                    }
                } else {
                    // Allow selecting from this empty slot
                    egui::ComboBox::from_id_salt(format!("softkey_slot_{}", idx))
                        .selected_text("Select...")
                        .show_ui(ui, |ui| {
                            // Show null ObjectPointer if available
                            if let Some(ptr_id) = null_pointer_id {
                                if ui
                                    .selectable_label(false, "(Empty - Null Pointer)")
                                    .clicked()
                                {
                                    // Fill gaps with None (not NULL ObjectPointer)
                                    while object_ids.len() <= idx {
                                        object_ids.push(NullableObjectId(None));
                                    }
                                    object_ids[idx] = NullableObjectId(Some(ptr_id));
                                }
                            }

                            for potential_child in
                                design.get_pool().objects_by_types(allowed_child_objects)
                            {
                                let id_val = potential_child.id().value();
                                let name = design
                                    .get_object_info(potential_child)
                                    .get_name(potential_child);
                                if ui
                                    .selectable_label(false, format!("{:?}: {}", id_val, name))
                                    .clicked()
                                {
                                    // Auto-expand vector when slot is filled (fill gaps with None)
                                    while object_ids.len() <= idx {
                                        object_ids.push(NullableObjectId(None));
                                    }
                                    object_ids[idx] = NullableObjectId(Some(potential_child.id()));
                                }
                            }
                        });

                    ui.colored_label(egui::Color32::GRAY, "Empty");
                    ui.label("");
                }
                ui.end_row();
            }
        });
}

fn render_nullable_object_id_list(
    ui: &mut egui::Ui,
    design: &EditorProject,
    nullable_object_ids: &mut Vec<NullableObjectId>,
    allowed_child_objects: &[ObjectType],
    current_object_id: ObjectId,
) {
    egui::Grid::new("object_id_grid")
        .striped(true)
        .min_col_width(0.0)
        .show(ui, |ui| {
            let mut idx = 0;
            while idx < nullable_object_ids.len() {
                ui.label(" - ");
                render_nullable_object_id_selector(
                    ui,
                    idx,
                    design,
                    &mut nullable_object_ids[idx],
                    allowed_child_objects,
                    Some(current_object_id),
                );
                if let Some(object_id) = &mut nullable_object_ids[idx].0 {
                    let obj: Option<&Object> = design.get_pool().object_by_id(*object_id);

                    if let Some(obj) = obj {
                        if ui.link(format!("{:?}", obj.object_type())).clicked() {
                            *design.get_mut_selected().borrow_mut() = obj.id().into();
                        }

                        // Add name column
                        let object_info = design.get_object_info(obj);
                        ui.label(object_info.get_name(obj));
                    } else {
                        ui.colored_label(egui::Color32::RED, "Missing object");
                        ui.label(""); // Empty cell for name column
                    }
                } else {
                    ui.label(""); // Empty cell for type
                    ui.label(""); // Empty cell for name
                }
                render_index_modifiers(ui, idx, nullable_object_ids);
                idx += 1;
                ui.end_row();
            }
        });

    let (new_object_id, success) = render_add_object_id(
        ui,
        design,
        allowed_child_objects,
        true,
        Some(current_object_id),
    );
    if success {
        nullable_object_ids.push(NullableObjectId(new_object_id));
    }
}

fn render_add_object_id(
    ui: &mut egui::Ui,
    design: &EditorProject,
    allowed_child_objects: &[ObjectType],
    allow_none: bool,
    current_object_id: Option<ObjectId>,
) -> (Option<ObjectId>, bool) {
    let pool = design.get_pool();
    let mut result = (None, false);
    ui.horizontal(|ui| {
        ui.label("Add object:");
        egui::ComboBox::from_id_salt("New Object Type")
            .selected_text("Select existing object")
            .show_ui(ui, |ui| {
                if allow_none {
                    if ui.selectable_label(false, "None").clicked() {
                        result = (None, true);
                    }
                }
                for potential_child in pool.objects_by_types(allowed_child_objects) {
                    let child_id = potential_child.id();

                    // Check if this would create a circular reference
                    let would_be_circular = if let Some(parent_id) = current_object_id {
                        would_create_circular_reference(pool, parent_id, child_id)
                    } else {
                        false
                    };

                    let object_info = design.get_object_info(potential_child);
                    let name = object_info.get_name(potential_child);
                    let label = format!(
                        "{:?}: {:?} - {}{}",
                        u16::from(child_id),
                        potential_child.object_type(),
                        name,
                        if would_be_circular {
                            " ⚠ (circular)"
                        } else {
                            ""
                        }
                    );

                    // Only allow clicking if it wouldn't create a circular reference
                    ui.add_enabled_ui(!would_be_circular, |ui| {
                        if ui.selectable_label(false, label).clicked() {
                            result = (Some(child_id), true);
                        }
                    });
                }
            });
    });
    result
}

fn render_macro_references(
    ui: &mut egui::Ui,
    design: &EditorProject,
    macro_refs: &mut Vec<MacroRef>,
    possible_events: &[Event],
) {
    egui::Grid::new("macro_grid")
        .striped(true)
        .min_col_width(0.0)
        .show(ui, |ui| {
            let mut idx = 0;
            while idx < macro_refs.len() {
                let macro_ref = &mut macro_refs[idx];

                if let Some(macro_obj) = design
                    .get_pool()
                    .objects_by_type(ObjectType::Macro)
                    .iter()
                    .find(|o| u16::from(o.id()) == macro_ref.macro_id as u16)
                {
                    ui.label(" - ");
                    ui.push_id(idx, |ui| {
                        egui::ComboBox::from_id_salt("event_id")
                            .selected_text(format!("{:?}", macro_ref.event_id))
                            .show_ui(ui, |ui| {
                                for event in possible_events {
                                    ui.selectable_value(
                                        &mut macro_ref.event_id,
                                        *event,
                                        format!("{:?}", event),
                                    );
                                }
                            });

                        if ui.link(" Macro ").clicked() {
                            *design.get_mut_selected().borrow_mut() = macro_obj.id().into();
                        }

                        egui::ComboBox::from_id_salt("macro_id")
                            .selected_text(format!("{:?}", macro_ref.macro_id))
                            .show_ui(ui, |ui| {
                                for potential_macro in
                                    design.get_pool().objects_by_type(ObjectType::Macro)
                                {
                                    ui.selectable_value(
                                        &mut macro_ref.macro_id,
                                        u16::from(potential_macro.id()) as u8,
                                        format!("{:?}", u16::from(potential_macro.id())),
                                    );
                                }
                            });
                    });
                } else {
                    ui.label(format!(
                        "- {:?}: Missing macro object {:?}",
                        macro_ref.event_id, macro_ref.macro_id
                    ));
                }

                render_index_modifiers(ui, idx, macro_refs);
                idx += 1;
                ui.end_row();
            }
        });

    render_add_macro_reference(ui, design.get_pool(), macro_refs, possible_events);
}

fn render_add_macro_reference(
    ui: &mut egui::Ui,
    pool: &ObjectPool,
    macro_refs: &mut Vec<MacroRef>,
    possible_events: &[Event],
) {
    ui.horizontal(|ui| {
        ui.label("Add macro:");
        ui.horizontal(|ui| {
            let mut selected_event = ui.data_mut(|data| {
                data.get_temp(egui::Id::new("selected_event"))
                    .unwrap_or(Event::Reserved)
            });
            egui::ComboBox::from_id_salt("New Event Type")
                .selected_text(if selected_event == Event::Reserved {
                    "Select event".to_string()
                } else {
                    format!("{:?}", selected_event)
                })
                .show_ui(ui, |ui| {
                    for event in possible_events {
                        if ui
                            .selectable_value(&mut selected_event, *event, format!("{:?}", event))
                            .changed()
                        {
                            ui.data_mut(|data| {
                                data.insert_temp(egui::Id::new("selected_event"), selected_event);
                            });
                        }
                    }
                });

            if selected_event != Event::Reserved {
                egui::ComboBox::from_id_salt("New Macro")
                    .selected_text("Select macro")
                    .show_ui(ui, |ui| {
                        for potential_macro in pool.objects_by_type(ObjectType::Macro) {
                            if ui
                                .selectable_label(
                                    false,
                                    format!("{:?}", u16::from(potential_macro.id())),
                                )
                                .clicked()
                            {
                                macro_refs.push(MacroRef {
                                    event_id: selected_event,
                                    macro_id: u16::from(potential_macro.id()) as u8,
                                });
                            }
                        }
                    });
            }
        });
    });
}

impl ConfigurableObject for WorkingSet {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);
        color_swatch_selector(
            ui,
            &mut self.background_colour,
            design.get_pool().get_colour_palette(),
            "Background Colour",
        );
        ui.checkbox(&mut self.selectable, "Selectable");
        ui.horizontal(|ui| {
            let masks = design
                .get_pool()
                .objects_by_types(&[ObjectType::DataMask, ObjectType::AlarmMask]);
            egui::ComboBox::from_label("Active Mask")
                .selected_text(format!("{:?}", u16::from(self.active_mask)))
                .show_ui(ui, |ui| {
                    for object in masks {
                        ui.selectable_value(
                            &mut self.active_mask,
                            object.id(),
                            format!("{:?}", u16::from(object.id())),
                        );
                    }
                });
            if ui.link("(view)").clicked() {
                *design.get_mut_selected().borrow_mut() = self.active_mask.into();
            }
        });
        ui.separator();
        ui.label("Objects:");
        render_object_references_list(
            ui,
            design,
            design.mask_size,
            design.mask_size,
            &mut self.object_refs,
            &Self::get_allowed_child_refs(_settings.vt_version),
            self.id,
        );

        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for DataMask {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);
        color_swatch_selector(
            ui,
            &mut self.background_colour,
            design.get_pool().get_colour_palette(),
            "Background Colour",
        );
        ui.horizontal(|ui| {
            egui::ComboBox::from_label("Soft Key Mask")
                .selected_text(
                    self.soft_key_mask
                        .0
                        .map_or("None".to_string(), |id| format!("{:?}", u16::from(id))),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.soft_key_mask,
                        NullableObjectId(None),
                        "None".to_string(),
                    );
                    for object in design.get_pool().objects_by_type(ObjectType::SoftKeyMask) {
                        ui.selectable_value(
                            &mut self.soft_key_mask,
                            NullableObjectId(Some(object.id())),
                            format!("{:?}", u16::from(object.id())),
                        );
                    }
                });
            if let Some(mask) = self.soft_key_mask.0 {
                if ui.link("(view)").clicked() {
                    *design.get_mut_selected().borrow_mut() = mask.into();
                }
            }
        });
        ui.separator();
        ui.label("Objects:");
        render_object_references_list(
            ui,
            design,
            design.mask_size,
            design.mask_size,
            &mut self.object_refs,
            &Self::get_allowed_child_refs(_settings.vt_version),
            self.id,
        );

        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for AlarmMask {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);
        color_swatch_selector(
            ui,
            &mut self.background_colour,
            design.get_pool().get_colour_palette(),
            "Background Colour",
        );
        ui.horizontal(|ui| {
            egui::ComboBox::from_label("Soft Key Mask")
                .selected_text(
                    self.soft_key_mask
                        .0
                        .map_or("None".to_string(), |id| format!("{:?}", u16::from(id))),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.soft_key_mask,
                        NullableObjectId(None),
                        "None".to_string(),
                    );
                    for object in design.get_pool().objects_by_type(ObjectType::SoftKeyMask) {
                        ui.selectable_value(
                            &mut self.soft_key_mask,
                            NullableObjectId(Some(object.id())),
                            format!("{:?}", u16::from(object.id())),
                        );
                    }
                });
            if let Some(mask) = self.soft_key_mask.0 {
                if ui.link("(view)").clicked() {
                    *design.get_mut_selected().borrow_mut() = mask.into();
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Priority:");
            ui.radio_value(&mut self.priority, 2, "Low");
            ui.radio_value(&mut self.priority, 1, "Medium");
            ui.radio_value(&mut self.priority, 0, "High");
        });
        ui.horizontal(|ui| {
            ui.label("Acoustic signal:");
            ui.radio_value(&mut self.acoustic_signal, 3, "None");
            ui.radio_value(&mut self.acoustic_signal, 2, "Lowest");
            ui.radio_value(&mut self.acoustic_signal, 1, "Medium");
            ui.radio_value(&mut self.acoustic_signal, 0, "Highest");
        });
        ui.separator();
        ui.label("Objects:");
        render_object_references_list(
            ui,
            design,
            design.mask_size,
            design.mask_size,
            &mut self.object_refs,
            &Self::get_allowed_child_refs(_settings.vt_version),
            self.id,
        );

        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for Container {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);
        ui.checkbox(&mut self.hidden, "Hidden");
        ui.add(
            egui::Slider::new(&mut self.width, 0..=design.mask_size)
                .text("Width")
                .drag_value_speed(1.0),
        );
        ui.add(
            egui::Slider::new(&mut self.height, 0..=design.mask_size)
                .text("Height")
                .drag_value_speed(1.0),
        );
        ui.separator();
        ui.label("Objects:");
        render_object_references_list(
            ui,
            design,
            self.width,
            self.height,
            &mut self.object_refs,
            &Self::get_allowed_child_refs(_settings.vt_version),
            self.id,
        );

        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for SoftKeyMask {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);
        color_swatch_selector(
            ui,
            &mut self.background_colour,
            design.get_pool().get_colour_palette(),
            "Background Colour",
        );
        ui.horizontal(|ui| {
            ui.label("Key Width:");
            ui.label(format!("{}", settings.softkey_key_width));
            ui.label("Key Height:");
            ui.label(format!("{}", settings.softkey_key_height));
        });
        ui.separator();
        ui.label("Objects:");
        render_object_id_list(
            ui,
            design,
            &mut self.objects,
            &Self::get_allowed_child_refs(settings.vt_version),
            self.id,
        );

        // Trim trailing None values to avoid saving empty slots
        while self.objects.last().map_or(false, |id| id.0.is_none()) {
            self.objects.pop();
        }

        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for Key {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);
        color_swatch_selector(
            ui,
            &mut self.background_colour,
            design.get_pool().get_colour_palette(),
            "Background Colour",
        );
        ui.horizontal(|ui| {
            ui.label("Key code:");
            ui.radio_value(&mut self.key_code, 0, "ACK");
            ui.add(egui::DragValue::new(&mut self.key_code).speed(1));
        });
        ui.horizontal(|ui| {
            ui.label("Width:");
            ui.label(format!("{}", settings.softkey_key_width));
            ui.label("Height:");
            ui.label(format!("{}", settings.softkey_key_height));
        });
        ui.separator();
        ui.label("Objects:");
        render_object_references_list(
            ui,
            design,
            design.mask_size,
            design.mask_size,
            &mut self.object_refs,
            &Self::get_allowed_child_refs(settings.vt_version),
            self.id,
        );

        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for Button {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);

        ui.add(
            egui::Slider::new(&mut self.width, 0..=design.mask_size)
                .text("Width")
                .drag_value_speed(1.0),
        );
        ui.add(
            egui::Slider::new(&mut self.height, 0..=design.mask_size)
                .text("Height")
                .drag_value_speed(1.0),
        );

        color_swatch_selector(
            ui,
            &mut self.background_colour,
            design.get_pool().get_colour_palette(),
            "Background Colour",
        );
        color_swatch_selector(
            ui,
            &mut self.border_colour,
            design.get_pool().get_colour_palette(),
            "Border Colour",
        );

        ui.horizontal(|ui| {
            ui.label("Key code:");
            ui.add(egui::DragValue::new(&mut self.key_code).speed(1.0));
        });

        ui.separator();
        ui.checkbox(&mut self.options.latchable, "Latchable");
        if self.options.latchable {
            ui.horizontal(|ui| {
                ui.label("Initial State:");
                ui.radio_value(&mut self.options.state, ButtonState::Released, "Released");
                ui.radio_value(&mut self.options.state, ButtonState::Latched, "Latched");
            });
        }
        ui.checkbox(&mut self.options.suppress_border, "Suppress Border");
        ui.checkbox(
            &mut self.options.transparent_background,
            "Transparent Background",
        );
        ui.checkbox(&mut self.options.disabled, "Disabled");
        ui.checkbox(&mut self.options.no_border, "No Border");

        ui.separator();
        ui.label("Objects:");
        render_object_references_list(
            ui,
            design,
            self.width,
            self.height,
            &mut self.object_refs,
            &Self::get_allowed_child_refs(_settings.vt_version),
            self.id,
        );

        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for InputBoolean {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);
        color_swatch_selector(
            ui,
            &mut self.background_colour,
            design.get_pool().get_colour_palette(),
            "Background Colour",
        );
        ui.add(
            egui::Slider::new(&mut self.width, 0..=design.mask_size)
                .text("Width")
                .drag_value_speed(1.0),
        );
        egui::ComboBox::from_id_salt("foreground_colour")
            .selected_text(format!("{:?}", u16::from(self.foreground_colour)))
            .show_ui(ui, |ui| {
                for potential_child in design
                    .get_pool()
                    .objects_by_type(ObjectType::FontAttributes)
                {
                    ui.selectable_value(
                        &mut self.foreground_colour,
                        potential_child.id(),
                        format!(
                            "{:?}: {:?}",
                            u16::from(potential_child.id()),
                            potential_child.object_type()
                        ),
                    );
                }
            });
        ui.horizontal(|ui| {
            ui.label("Variable reference:");
            egui::ComboBox::from_id_salt("variable_reference")
                .selected_text(format!("{:?}", u16::from(self.variable_reference)))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.variable_reference,
                        NullableObjectId::NULL,
                        "None",
                    );
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::NumberVariable)
                    {
                        ui.selectable_value(
                            &mut self.variable_reference,
                            potential_child.id().into(),
                            format!(
                                "{:?}: {:?}",
                                u16::from(potential_child.id()),
                                potential_child.object_type()
                            ),
                        );
                    }
                });
        });
        if self.variable_reference.0.is_none() {
            ui.label("Initial value:");
            egui::ComboBox::from_id_salt("initial_value")
                .selected_text(format!("{:?}", self.value))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.value, false, "False");
                    ui.selectable_value(&mut self.value, true, "True");
                });
        }
        ui.checkbox(&mut self.enabled, "Enabled");
        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for InputString {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);
        ui.add(
            egui::Slider::new(&mut self.width, 0..=design.mask_size)
                .text("Width")
                .drag_value_speed(1.0),
        );
        ui.add(
            egui::Slider::new(&mut self.height, 0..=design.mask_size)
                .text("Height")
                .drag_value_speed(1.0),
        );
        color_swatch_selector(
            ui,
            &mut self.background_colour,
            design.get_pool().get_colour_palette(),
            "Background Colour",
        );
        ui.horizontal(|ui| {
            ui.label("Font attributes:");
            egui::ComboBox::from_id_salt("font_attributes")
                .selected_text(format!("{:?}", u16::from(self.font_attributes)))
                .show_ui(ui, |ui| {
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::FontAttributes)
                    {
                        ui.selectable_value(
                            &mut self.font_attributes,
                            potential_child.id(),
                            format!("{:?}", u16::from(potential_child.id())),
                        );
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.label("Input attributes:");
            egui::ComboBox::from_id_salt("input_attributes")
                .selected_text(format!("{:?}", u16::from(self.input_attributes)))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.input_attributes, NullableObjectId::NULL, "None");
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::InputAttributes)
                    {
                        ui.selectable_value(
                            &mut self.input_attributes,
                            potential_child.id().into(),
                            format!("{:?}", u16::from(potential_child.id())),
                        );
                    }
                });
        });
        ui.checkbox(&mut self.options.transparent, "Transparent Background");
        ui.checkbox(&mut self.options.auto_wrap, "Auto Wrap");
        // TODO: check if we have VT version 4 or later
        // if self.options.auto_wrap {
        //     ui.checkbox(&mut self.options.wrap_on_hyphen, "Wrap on Hyphen");
        // }
        ui.horizontal(|ui| {
            ui.label("Variable reference:");
            egui::ComboBox::from_id_salt("variable_reference")
                .selected_text(format!("{:?}", u16::from(self.variable_reference)))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.variable_reference,
                        NullableObjectId::NULL,
                        "None",
                    );
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::StringVariable)
                    {
                        ui.selectable_value(
                            &mut self.variable_reference,
                            potential_child.id().into(),
                            format!(
                                "{:?}: {:?}",
                                u16::from(potential_child.id()),
                                potential_child.object_type()
                            ),
                        );
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.label("Horizontal Justification:");
            let before = self.justification.horizontal;
            ui.radio_value(
                &mut self.justification.horizontal,
                HorizontalAlignment::Left,
                "Left",
            );
            ui.radio_value(
                &mut self.justification.horizontal,
                HorizontalAlignment::Middle,
                "Middle",
            );
            ui.radio_value(
                &mut self.justification.horizontal,
                HorizontalAlignment::Right,
                "Right",
            );
        });
        // TODO: check if we have VT version 4 or later
        if _settings.vt_version >= VtVersion::Version4 {
            ui.horizontal(|ui| {
                ui.label("Vertical Justification:");
                ui.radio_value(
                    &mut self.justification.vertical,
                    VerticalAlignment::Top,
                    "Top",
                );
                ui.radio_value(
                    &mut self.justification.vertical,
                    VerticalAlignment::Middle,
                    "Middle",
                );
                ui.radio_value(
                    &mut self.justification.vertical,
                    VerticalAlignment::Bottom,
                    "Bottom",
                );
            });
        }
        if self.variable_reference.0.is_none() {
            ui.label("Initial value:");
            ui.text_edit_singleline(&mut self.value);
        }
        ui.checkbox(&mut self.enabled, "Enabled");
        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for InputNumber {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);
        ui.add(
            egui::Slider::new(&mut self.width, 0..=design.mask_size)
                .text("Width")
                .drag_value_speed(1.0),
        );
        ui.add(
            egui::Slider::new(&mut self.height, 0..=design.mask_size)
                .text("Height")
                .drag_value_speed(1.0),
        );
        color_swatch_selector(
            ui,
            &mut self.background_colour,
            design.get_pool().get_colour_palette(),
            "Background Colour",
        );
        ui.horizontal(|ui| {
            ui.label("Font attributes:");
            egui::ComboBox::from_id_salt("font_attributes")
                .selected_text(format!("{:?}", u16::from(self.font_attributes)))
                .show_ui(ui, |ui| {
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::FontAttributes)
                    {
                        ui.selectable_value(
                            &mut self.font_attributes,
                            potential_child.id(),
                            format!("{:?}", u16::from(potential_child.id())),
                        );
                    }
                });
        });
        ui.checkbox(&mut self.options.transparent, "Transparent Background");
        ui.checkbox(
            &mut self.options.display_leading_zeros,
            "Display Leading Zeros",
        );
        ui.checkbox(
            &mut self.options.display_zero_as_blank,
            "Display Zero as Blank",
        );
        // TODO: check if we have VT version 4 or later
        // ui.checkbox(&mut self.options.truncate, "Truncate");
        ui.horizontal(|ui| {
            ui.label("Variable reference:");
            egui::ComboBox::from_id_salt("variable_reference")
                .selected_text(format!("{:?}", u16::from(self.variable_reference)))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.variable_reference,
                        NullableObjectId::NULL,
                        "None",
                    );
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::NumberVariable)
                    {
                        ui.selectable_value(
                            &mut self.variable_reference,
                            potential_child.id().into(),
                            format!(
                                "{:?}: {:?}",
                                u16::from(potential_child.id()),
                                potential_child.object_type()
                            ),
                        );
                    }
                });
        });
        if self.variable_reference.0.is_none() {
            ui.label("Initial value:");
            ui.add(egui::DragValue::new(&mut self.value).speed(1.0));
        }
        ui.add(
            egui::DragValue::new(&mut self.min_value)
                .speed(1.0)
                .prefix("Min: "),
        );
        ui.add(
            egui::DragValue::new(&mut self.max_value)
                .speed(1.0)
                .prefix("Max: "),
        );
        ui.add(
            egui::DragValue::new(&mut self.offset)
                .speed(1.0)
                .prefix("Offset: "),
        );
        ui.add(egui::DragValue::new(&mut self.scale).prefix("Scale: "));
        ui.add(
            egui::DragValue::new(&mut self.nr_of_decimals)
                .speed(1.0)
                .prefix("Number of Decimals: "),
        );
        ui.horizontal(|ui| {
            ui.label("Format:");
            ui.radio_value(&mut self.format, FormatType::Decimal, "Decimal");
            ui.radio_value(&mut self.format, FormatType::Exponential, "Exponential");
        });

        ui.horizontal(|ui| {
            ui.label("Horizontal Justification:");
            ui.radio_value(
                &mut self.justification.horizontal,
                HorizontalAlignment::Left,
                "Left",
            );
            ui.radio_value(
                &mut self.justification.horizontal,
                HorizontalAlignment::Middle,
                "Middle",
            );
            ui.radio_value(
                &mut self.justification.horizontal,
                HorizontalAlignment::Right,
                "Right",
            );
        });
        // TODO: check if we have VT version 4 or later
        if _settings.vt_version >= VtVersion::Version4 {
            ui.horizontal(|ui| {
                ui.label("Vertical Justification:");
                ui.radio_value(
                    &mut self.justification.vertical,
                    VerticalAlignment::Top,
                    "Top",
                );
                ui.radio_value(
                    &mut self.justification.vertical,
                    VerticalAlignment::Middle,
                    "Middle",
                );
                ui.radio_value(
                    &mut self.justification.vertical,
                    VerticalAlignment::Bottom,
                    "Bottom",
                );
            });
        }

        ui.checkbox(&mut self.options2.enabled, "Enabled");
        // TODO: check if we have VT version 4 or later
        // ui.checkbox(&mut self.options2.real_time_editing, "Real Time Editing");

        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for InputList {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);
        ui.add(
            egui::Slider::new(&mut self.width, 0..=design.mask_size)
                .text("Width")
                .drag_value_speed(1.0),
        );
        ui.add(
            egui::Slider::new(&mut self.height, 0..=design.mask_size)
                .text("Height")
                .drag_value_speed(1.0),
        );
        ui.horizontal(|ui| {
            ui.label("Variable reference:");
            egui::ComboBox::from_id_salt("variable_reference")
                .selected_text(format!("{:?}", u16::from(self.variable_reference)))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.variable_reference,
                        NullableObjectId::NULL,
                        "None",
                    );
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::NumberVariable)
                    {
                        ui.selectable_value(
                            &mut self.variable_reference,
                            potential_child.id().into(),
                            format!(
                                "{:?}: {:?}",
                                u16::from(potential_child.id()),
                                potential_child.object_type()
                            ),
                        );
                    }
                });
        });
        if self.variable_reference.0.is_none() {
            ui.label("Initial value:");
            ui.add(egui::DragValue::new(&mut self.value).speed(1.0));
        }

        ui.checkbox(&mut self.options.enabled, "Enabled");
        // TODO: check if we have VT version 4 or later
        // ui.checkbox(&mut self.options.real_time_editing, "Real Time Editing");

        ui.separator();
        ui.label("List items:");
        render_nullable_object_id_list(
            ui,
            design,
            &mut self.list_items,
            &Self::get_allowed_child_refs(_settings.vt_version),
            self.id,
        );

        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for OutputString {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);
        ui.add(
            egui::Slider::new(&mut self.width, 0..=design.mask_size)
                .text("Width")
                .drag_value_speed(1.0),
        );
        ui.add(
            egui::Slider::new(&mut self.height, 0..=design.mask_size)
                .text("Height")
                .drag_value_speed(1.0),
        );
        color_swatch_selector(
            ui,
            &mut self.background_colour,
            design.get_pool().get_colour_palette(),
            "Background Colour",
        );
        ui.horizontal(|ui| {
            ui.label("Font attributes:");
            egui::ComboBox::from_id_salt("font_attributes")
                .selected_text(format!("{:?}", u16::from(self.font_attributes)))
                .show_ui(ui, |ui| {
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::FontAttributes)
                    {
                        ui.selectable_value(
                            &mut self.font_attributes,
                            potential_child.id(),
                            format!("{:?}", u16::from(potential_child.id())),
                        );
                    }
                });
        });
        ui.checkbox(&mut self.options.transparent, "Transparent Background");
        ui.checkbox(&mut self.options.auto_wrap, "Auto Wrap");
        // TODO: check if we have VT version 4 or later
        // if self.options.auto_wrap {
        //     ui.checkbox(&mut self.options.wrap_on_hyphen, "Wrap on Hyphen");
        // }
        ui.horizontal(|ui| {
            ui.label("Variable reference:");
            egui::ComboBox::from_id_salt("variable_reference")
                .selected_text(format!("{:?}", u16::from(self.variable_reference)))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.variable_reference,
                        NullableObjectId::NULL,
                        "None",
                    );
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::StringVariable)
                    {
                        ui.selectable_value(
                            &mut self.variable_reference,
                            potential_child.id().into(),
                            format!(
                                "{:?}: {:?}",
                                u16::from(potential_child.id()),
                                potential_child.object_type()
                            ),
                        );
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.label("Horizontal Justification:");
            ui.radio_value(
                &mut self.justification.horizontal,
                HorizontalAlignment::Left,
                "Left",
            );
            ui.radio_value(
                &mut self.justification.horizontal,
                HorizontalAlignment::Middle,
                "Middle",
            );
            ui.radio_value(
                &mut self.justification.horizontal,
                HorizontalAlignment::Right,
                "Right",
            );
        });
        // TODO: check if we have VT version 4 or later
        if _settings.vt_version >= VtVersion::Version4 {
            ui.horizontal(|ui| {
                ui.label("Vertical Justification:");
                ui.radio_value(
                    &mut self.justification.vertical,
                    VerticalAlignment::Top,
                    "Top",
                );
                ui.radio_value(
                    &mut self.justification.vertical,
                    VerticalAlignment::Middle,
                    "Middle",
                );
                ui.radio_value(
                    &mut self.justification.vertical,
                    VerticalAlignment::Bottom,
                    "Bottom",
                );
            });
        }
        let string_len = if self.variable_reference.0.is_none() {
            ui.label("Initial value:");
            ui.text_edit_singleline(&mut self.value);
            self.value.len()
        } else {
            if let Some(Object::StringVariable(sv)) = design
                .get_pool()
                .object_by_id(self.variable_reference.0.unwrap())
            {
                sv.value.len()
            } else {
                0
            }
        };
        ui.label(format!("String length: {}", string_len));

        // Calculate min width/height based on font size
        let font_attrs = design.get_pool().object_by_id(self.font_attributes);
        if let Some(Object::FontAttributes(fa)) = font_attrs {
            match fa.font_size {
                FontSize::NonProportional(size) => {
                    let min_width = size.width() as usize * string_len;
                    let min_height = size.height() as usize;
                    ui.label(format!(
                        "Min width: {} px, Min height: {} px ({}x{})",
                        min_width,
                        min_height,
                        size.width(),
                        size.height()
                    ));
                }
                FontSize::Proportional(height) => {
                    let min_width = string_len * height as usize; // crude estimate
                    let min_height = height as usize;
                    ui.label(format!(
                        "Min width: ~{} px, Min height: {} px (proportional)",
                        min_width, min_height
                    ));
                }
            }
        } else {
            ui.label("(Font attributes not found)");
        }
        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for OutputNumber {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);
        ui.add(
            egui::Slider::new(&mut self.width, 0..=design.mask_size)
                .text("Width")
                .drag_value_speed(1.0),
        );
        ui.add(
            egui::Slider::new(&mut self.height, 0..=design.mask_size)
                .text("Height")
                .drag_value_speed(1.0),
        );
        color_swatch_selector(
            ui,
            &mut self.background_colour,
            design.get_pool().get_colour_palette(),
            "Background Colour",
        );
        ui.horizontal(|ui| {
            ui.label("Font attributes:");
            egui::ComboBox::from_id_salt("font_attributes")
                .selected_text(format!("{:?}", u16::from(self.font_attributes)))
                .show_ui(ui, |ui| {
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::FontAttributes)
                    {
                        ui.selectable_value(
                            &mut self.font_attributes,
                            potential_child.id(),
                            format!("{:?}", u16::from(potential_child.id())),
                        );
                    }
                });
        });
        ui.checkbox(&mut self.options.transparent, "Transparent Background");
        ui.checkbox(
            &mut self.options.display_leading_zeros,
            "Display Leading Zeros",
        );
        ui.checkbox(
            &mut self.options.display_zero_as_blank,
            "Display Zero as Blank",
        );
        // TODO: check if we have VT version 4 or later
        // ui.checkbox(&mut self.options.truncate, "Truncate");
        ui.horizontal(|ui| {
            ui.label("Variable reference:");
            egui::ComboBox::from_id_salt("variable_reference")
                .selected_text(format!("{:?}", u16::from(self.variable_reference)))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.variable_reference,
                        NullableObjectId::NULL,
                        "None",
                    );
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::NumberVariable)
                    {
                        ui.selectable_value(
                            &mut self.variable_reference,
                            potential_child.id().into(),
                            format!(
                                "{:?}: {:?}",
                                u16::from(potential_child.id()),
                                potential_child.object_type()
                            ),
                        );
                    }
                });
        });
        if self.variable_reference.0.is_none() {
            ui.label("Initial value:");
            ui.add(egui::DragValue::new(&mut self.value).speed(1.0));
        }
        ui.horizontal(|ui| {
            ui.label("Offset:");
            ui.add(egui::DragValue::new(&mut self.offset).speed(1.0));
        });
        ui.horizontal(|ui| {
            ui.label("Scale:");
            ui.add(egui::DragValue::new(&mut self.scale).speed(1.0));
        });
        ui.horizontal(|ui| {
            ui.label("Number of Decimals:");
            ui.add(egui::DragValue::new(&mut self.nr_of_decimals).speed(1.0));
        });
        ui.horizontal(|ui| {
            ui.label("Format:");
            ui.radio_value(&mut self.format, FormatType::Decimal, "Decimal");
            ui.radio_value(&mut self.format, FormatType::Exponential, "Exponential");
        });

        ui.horizontal(|ui| {
            ui.label("Horizontal Justification:");
            ui.radio_value(
                &mut self.justification.horizontal,
                HorizontalAlignment::Left,
                "Left",
            );
            ui.radio_value(
                &mut self.justification.horizontal,
                HorizontalAlignment::Middle,
                "Middle",
            );
            ui.radio_value(
                &mut self.justification.horizontal,
                HorizontalAlignment::Right,
                "Right",
            );
        });
        // TODO: check if we have VT version 4 or later
        if _settings.vt_version >= VtVersion::Version4 {
            ui.horizontal(|ui| {
                ui.label("Vertical Justification:");
                ui.radio_value(
                    &mut self.justification.vertical,
                    VerticalAlignment::Top,
                    "Top",
                );
                ui.radio_value(
                    &mut self.justification.vertical,
                    VerticalAlignment::Middle,
                    "Middle",
                );
                ui.radio_value(
                    &mut self.justification.vertical,
                    VerticalAlignment::Bottom,
                    "Bottom",
                );
            });
        }
        // Calculate number of digits for OutputNumber
        // If variable reference is set, try to get the NumberVariable's max digits, else use current value
        let num_digits = if self.variable_reference.0.is_none() {
            // Use the current value, offset, scale, decimals, and format to estimate the number of digits
            let val = (self.value as f32) * self.scale + self.offset as f32;
            match self.format {
                FormatType::Decimal => {
                    // Count digits before decimal, plus decimals, plus sign if negative, plus decimal point
                    let int_part = val.abs().trunc() as u64;
                    let mut digits = int_part.to_string().len();
                    if val < 0.0 {
                        digits += 1;
                    }
                    if self.nr_of_decimals > 0 {
                        digits += 1 + self.nr_of_decimals as usize;
                    }
                    digits
                }
                FormatType::Exponential => {
                    // e.g. -1.23e+04: sign, 1, dot, decimals, 'e', sign, exp digits
                    let mut digits = 1; // first digit
                    if val < 0.0 {
                        digits += 1;
                    }
                    if self.nr_of_decimals > 0 {
                        digits += 1 + self.nr_of_decimals as usize;
                    } // dot + decimals
                    digits += 2; // 'e' and exp sign
                    digits += 2; // at least two exp digits
                    digits
                }
            }
        } else {
            // Try to get the NumberVariable's max digits if available
            if let Some(Object::NumberVariable(nv)) = design
                .get_pool()
                .object_by_id(self.variable_reference.0.unwrap())
            {
                let val = (nv.value as f32) * self.scale + self.offset as f32;
                match self.format {
                    FormatType::Decimal => {
                        let int_part = val.abs().trunc() as u64;
                        let mut digits = int_part.to_string().len();
                        if val < 0.0 {
                            digits += 1;
                        }
                        if self.nr_of_decimals > 0 {
                            digits += 1 + self.nr_of_decimals as usize;
                        }
                        digits
                    }
                    FormatType::Exponential => {
                        let mut digits = 1;
                        if val < 0.0 {
                            digits += 1;
                        }
                        if self.nr_of_decimals > 0 {
                            digits += 1 + self.nr_of_decimals as usize;
                        }
                        digits += 2; // 'e' and exp sign
                        digits += 2; // at least two exp digits
                        digits
                    }
                }
            } else {
                0
            }
        };
        ui.label(format!("Number length: {}", num_digits));

        // Calculate min width/height based on font size
        let font_attrs = design.get_pool().object_by_id(self.font_attributes);
        if let Some(Object::FontAttributes(fa)) = font_attrs {
            match fa.font_size {
                FontSize::NonProportional(size) => {
                    let min_width = size.width() as usize * num_digits;
                    let min_height = size.height() as usize;
                    ui.label(format!(
                        "Min width: {} px, Min height: {} px ({}x{})",
                        min_width,
                        min_height,
                        size.width(),
                        size.height()
                    ));
                }
                FontSize::Proportional(height) => {
                    let min_width = num_digits * height as usize; // crude estimate
                    let min_height = height as usize;
                    ui.label(format!(
                        "Min width: ~{} px, Min height: {} px (proportional)",
                        min_width, min_height
                    ));
                }
            }
        } else {
            ui.label("(Font attributes not found)");
        }

        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for OutputList {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);

        ui.add(
            egui::Slider::new(&mut self.width, 0..=design.mask_size)
                .text("Width")
                .drag_value_speed(1.0),
        );
        ui.add(
            egui::Slider::new(&mut self.height, 0..=design.mask_size)
                .text("Height")
                .drag_value_speed(1.0),
        );

        ui.horizontal(|ui| {
            ui.label("Variable reference:");
            egui::ComboBox::from_id_salt("variable_reference")
                .selected_text(format!("{:?}", u16::from(self.variable_reference)))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.variable_reference,
                        NullableObjectId::NULL,
                        "None",
                    );
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::NumberVariable)
                    {
                        ui.selectable_value(
                            &mut self.variable_reference,
                            potential_child.id().into(),
                            format!(
                                "{:?}: {:?}",
                                u16::from(potential_child.id()),
                                potential_child.object_type()
                            ),
                        );
                    }
                });
        });

        if self.variable_reference.0.is_none() {
            ui.label("Initial value:");
            ui.add(egui::DragValue::new(&mut self.value).speed(1.0));
        }

        ui.separator();
        ui.label("List items:");
        render_nullable_object_id_list(
            ui,
            design,
            &mut self.list_items,
            &Self::get_allowed_child_refs(_settings.vt_version),
            self.id,
        );

        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for OutputLine {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);

        ui.horizontal(|ui| {
            ui.label("Line Attributes:");
            egui::ComboBox::from_id_salt("line_attributes")
                .selected_text(format!("{:?}", u16::from(self.line_attributes)))
                .show_ui(ui, |ui| {
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::LineAttributes)
                    {
                        ui.selectable_value(
                            &mut self.line_attributes,
                            potential_child.id(),
                            format!(
                                "{:?}: {:?}",
                                u16::from(potential_child.id()),
                                potential_child.object_type()
                            ),
                        );
                    }
                });

            // If a valid line_attributes object is selected, provide a link to navigate there
            if let Some(obj) = design.get_pool().object_by_id(self.line_attributes) {
                if ui.link("(view)").clicked() {
                    *design.get_mut_selected().borrow_mut() = self.line_attributes.into();
                }
            } else {
                ui.colored_label(egui::Color32::RED, "Missing object");
            }
        });

        ui.add(
            egui::Slider::new(&mut self.width, 0..=design.mask_size)
                .text("Width")
                .drag_value_speed(1.0),
        );
        ui.add(
            egui::Slider::new(&mut self.height, 0..=design.mask_size)
                .text("Height")
                .drag_value_speed(1.0),
        );

        ui.horizontal(|ui| {
            ui.label("Line Direction:");
            ui.radio_value(
                &mut self.line_direction,
                LineDirection::TopLeftToBottomRight,
                "Top-left to bottom-right",
            );
            ui.radio_value(
                &mut self.line_direction,
                LineDirection::BottomLeftToTopRight,
                "Bottom-left to top-right",
            );
        });

        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for OutputRectangle {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);

        ui.horizontal(|ui| {
            ui.label("Line Attributes:");
            egui::ComboBox::from_id_salt("line_attributes_selector")
                .selected_text(format!("{:?}", u16::from(self.line_attributes)))
                .show_ui(ui, |ui| {
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::LineAttributes)
                    {
                        ui.selectable_value(
                            &mut self.line_attributes,
                            potential_child.id(),
                            format!(
                                "{:?}: {:?}",
                                u16::from(potential_child.id()),
                                potential_child.object_type()
                            ),
                        );
                    }
                });

            // Link to view the selected line attributes object
            if let Some(obj) = design.get_pool().object_by_id(self.line_attributes) {
                if ui.link("(view)").clicked() {
                    *design.get_mut_selected().borrow_mut() = self.line_attributes.into();
                }
            } else {
                ui.colored_label(egui::Color32::RED, "Missing object");
            }
        });

        ui.add(
            egui::Slider::new(&mut self.width, 0..=design.mask_size)
                .text("Width")
                .drag_value_speed(1.0),
        );
        ui.add(
            egui::Slider::new(&mut self.height, 0..=design.mask_size)
                .text("Height")
                .drag_value_speed(1.0),
        );

        ui.horizontal(|ui| {
            ui.label("Line Suppression:");
            ui.add(egui::DragValue::new(&mut self.line_suppression).speed(1.0));
        });

        // Fill Attributes Selection
        ui.horizontal(|ui| {
            ui.label("Fill Attributes:");
            egui::ComboBox::from_id_salt("fill_attributes_selector")
                .selected_text(
                    self.fill_attributes
                        .0
                        .map_or("None".to_string(), |id| format!("{:?}", u16::from(id))),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.fill_attributes, NullableObjectId::NULL, "None");
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::FillAttributes)
                    {
                        ui.selectable_value(
                            &mut self.fill_attributes,
                            potential_child.id().into(),
                            format!(
                                "{:?}: {:?}",
                                u16::from(potential_child.id()),
                                potential_child.object_type()
                            ),
                        );
                    }
                });

            // Link to view the selected fill attributes object, if present
            if let Some(id) = self.fill_attributes.into() {
                if let Some(obj) = design.get_pool().object_by_id(id) {
                    if ui.link("(view)").clicked() {
                        *design.get_mut_selected().borrow_mut() = id.into();
                    }
                } else {
                    ui.colored_label(egui::Color32::RED, "Missing object");
                }
            }
        });

        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for OutputEllipse {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);

        ui.horizontal(|ui| {
            ui.label("Line Attributes:");
            egui::ComboBox::from_id_salt("line_attributes_selector")
                .selected_text(format!("{:?}", u16::from(self.line_attributes)))
                .show_ui(ui, |ui| {
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::LineAttributes)
                    {
                        ui.selectable_value(
                            &mut self.line_attributes,
                            potential_child.id(),
                            format!(
                                "{:?}: {:?}",
                                u16::from(potential_child.id()),
                                potential_child.object_type()
                            ),
                        );
                    }
                });

            // Link to navigate to the chosen line attributes object
            if let Some(obj) = design.get_pool().object_by_id(self.line_attributes) {
                if ui.link("(view)").clicked() {
                    *design.get_mut_selected().borrow_mut() = self.line_attributes.into();
                }
            } else {
                ui.colored_label(egui::Color32::RED, "Missing object");
            }
        });

        ui.add(
            egui::Slider::new(&mut self.width, 0..=design.mask_size)
                .text("Width")
                .drag_value_speed(1.0),
        );
        ui.add(
            egui::Slider::new(&mut self.height, 0..=design.mask_size)
                .text("Height")
                .drag_value_speed(1.0),
        );

        ui.label("Ellipse Type:");
        ui.radio_value(&mut self.ellipse_type, 0, "Closed Ellipse");
        ui.radio_value(&mut self.ellipse_type, 1, "Open Ellipse");
        ui.radio_value(&mut self.ellipse_type, 2, "Closed Ellipse Segment");
        ui.radio_value(&mut self.ellipse_type, 3, "Closed Ellipse Section");

        ui.horizontal(|ui| {
            ui.label("Start Angle:");
            ui.add(
                egui::DragValue::new(&mut self.start_angle)
                    .speed(1.0)
                    .range(0..=180),
            );
            ui.label("End Angle:");
            ui.add(
                egui::DragValue::new(&mut self.end_angle)
                    .speed(1.0)
                    .range(0..=180),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Fill Attributes:");
            egui::ComboBox::from_id_salt("fill_attributes_selector")
                .selected_text(
                    self.fill_attributes
                        .0
                        .map_or("None".to_string(), |id| format!("{:?}", u16::from(id))),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.fill_attributes, NullableObjectId::NULL, "None");
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::FillAttributes)
                    {
                        ui.selectable_value(
                            &mut self.fill_attributes,
                            potential_child.id().into(),
                            format!(
                                "{:?}: {:?}",
                                u16::from(potential_child.id()),
                                potential_child.object_type()
                            ),
                        );
                    }
                });

            // Link to view the chosen fill attributes object, if any
            if let Some(id) = self.fill_attributes.into() {
                if let Some(obj) = design.get_pool().object_by_id(id) {
                    if ui.link("(view)").clicked() {
                        *design.get_mut_selected().borrow_mut() = id.into();
                    }
                } else {
                    ui.colored_label(egui::Color32::RED, "Missing object");
                }
            }
        });

        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for OutputPolygon {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);
        ui.add(
            egui::Slider::new(&mut self.width, 0..=design.mask_size)
                .text("Width")
                .drag_value_speed(1.0),
        );
        ui.add(
            egui::Slider::new(&mut self.height, 0..=design.mask_size)
                .text("Height")
                .drag_value_speed(1.0),
        );
        ui.horizontal(|ui| {
            ui.label("Line Attributes:");
            egui::ComboBox::from_id_salt("line_attributes_selector")
                .selected_text(format!("{:?}", u16::from(self.line_attributes)))
                .show_ui(ui, |ui| {
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::LineAttributes)
                    {
                        ui.selectable_value(
                            &mut self.line_attributes,
                            potential_child.id(),
                            format!(
                                "{:?}: {:?}",
                                u16::from(potential_child.id()),
                                potential_child.object_type()
                            ),
                        );
                    }
                });
            // Link to navigate to the chosen line attributes object
            if let Some(obj) = design.get_pool().object_by_id(self.line_attributes) {
                if ui.link("(view)").clicked() {
                    *design.get_mut_selected().borrow_mut() = self.line_attributes.into();
                }
            } else {
                ui.colored_label(egui::Color32::RED, "Missing object");
            }
        });
        // Fill Attributes selector
        ui.horizontal(|ui| {
            ui.label("Fill Attributes:");
            egui::ComboBox::from_id_salt("fill_attributes_selector")
                .selected_text(
                    self.fill_attributes
                        .0
                        .map_or("None".to_string(), |id| format!("{:?}", u16::from(id))),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.fill_attributes, NullableObjectId::NULL, "None");
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::FillAttributes)
                    {
                        ui.selectable_value(
                            &mut self.fill_attributes,
                            potential_child.id().into(),
                            format!(
                                "{:?}: {:?}",
                                u16::from(potential_child.id()),
                                potential_child.object_type()
                            ),
                        );
                    }
                });
            // Link to view the selected fill attributes object, if present
            if let Some(id) = self.fill_attributes.into() {
                if let Some(obj) = design.get_pool().object_by_id(id) {
                    if ui.link("(view)").clicked() {
                        *design.get_mut_selected().borrow_mut() = id.into();
                    }
                } else {
                    ui.colored_label(egui::Color32::RED, "Missing object");
                }
            }
        });

        // One-shot rotation: user enters angle, presses Rotate, points are rotated by that angle about centroid
        ui.separator();
        ui.label("Rotate All Points:");
        let angle_id = ui.make_persistent_id("rotate_angle");
        let mut rotate_angle = ui
            .ctx()
            .memory_mut(|mem| mem.data.get_temp::<f32>(angle_id).unwrap_or(0.0));
        ui.horizontal(|ui| {
            let changed = ui
                .add(
                    egui::DragValue::new(&mut rotate_angle)
                        .speed(1.0)
                        .suffix(" deg")
                        .range(-360.0..=360.0),
                )
                .changed();
            if changed {
                // Clamp manually in case user pastes a value
                if rotate_angle > 360.0 {
                    rotate_angle = 360.0;
                }
                if rotate_angle < -360.0 {
                    rotate_angle = -360.0;
                }
                ui.ctx()
                    .memory_mut(|mem| mem.data.insert_temp(angle_id, rotate_angle));
            }
            if ui
                .button("Rotate")
                .on_hover_text("Rotate all points by this angle about centroid")
                .clicked()
            {
                if !self.points.is_empty() && rotate_angle.abs() > f32::EPSILON {
                    let angle_rad = rotate_angle.to_radians();
                    let (sum_x, sum_y) = self
                        .points
                        .iter()
                        .fold((0.0, 0.0), |(sx, sy), p| (sx + p.x as f32, sy + p.y as f32));
                    let n = self.points.len() as f32;
                    let (cx, cy) = (sum_x / n, sum_y / n);
                    for pt in &mut self.points {
                        let x = pt.x as f32 - cx;
                        let y = pt.y as f32 - cy;
                        let new_x = x * angle_rad.cos() - y * angle_rad.sin();
                        let new_y = x * angle_rad.sin() + y * angle_rad.cos();
                        pt.x = ((new_x + cx).round().max(0.0)) as u16;
                        pt.y = ((new_y + cy).round().max(0.0)) as u16;
                    }
                    // Reset angle after rotation
                    rotate_angle = 0.0;
                    ui.ctx()
                        .memory_mut(|mem| mem.data.insert_temp(angle_id, rotate_angle));
                }
            }
        });

        // --- Polygon Type Detection Helpers ---
        #[derive(Copy, Clone, Debug, PartialEq, Eq)]
        enum PolygonTypeAuto {
            Convex = 0,
            NonConvex = 1,
            Complex = 2,
            Open = 3,
        }

        fn detect_polygon_type(points: &[Point<u16>]) -> PolygonTypeAuto {
            if points.len() < 3 {
                return PolygonTypeAuto::Open;
            }
            // Check for self-intersection (complex)
            if is_self_intersecting(points) {
                return PolygonTypeAuto::Complex;
            }
            // Check convexity
            if is_convex(points) {
                PolygonTypeAuto::Convex
            } else {
                PolygonTypeAuto::NonConvex
            }
        }

        fn is_self_intersecting(points: &[Point<u16>]) -> bool {
            // Simple O(n^2) check for edge intersection (excluding adjacent edges)
            let n = points.len();
            for i in 0..n {
                let a1 = &points[i];
                let a2 = &points[(i + 1) % n];
                for j in (i + 1)..n {
                    // Skip adjacent edges
                    if (i + 1) % n == j || i == (j + 1) % n {
                        continue;
                    }
                    let b1 = &points[j];
                    let b2 = &points[(j + 1) % n];
                    if segments_intersect(a1, a2, b1, b2) {
                        return true;
                    }
                }
            }
            false
        }

        fn segments_intersect(
            a1: &Point<u16>,
            a2: &Point<u16>,
            b1: &Point<u16>,
            b2: &Point<u16>,
        ) -> bool {
            fn ccw(p1: &Point<u16>, p2: &Point<u16>, p3: &Point<u16>) -> bool {
                (p3.y as i32 - p1.y as i32) * (p2.x as i32 - p1.x as i32)
                    > (p2.y as i32 - p1.y as i32) * (p3.x as i32 - p1.x as i32)
            }
            (ccw(a1, b1, b2) != ccw(a2, b1, b2)) && (ccw(a1, a2, b1) != ccw(a1, a2, b2))
        }

        fn is_convex(points: &[Point<u16>]) -> bool {
            let n = points.len();
            if n < 4 {
                return true; // triangle is always convex
            }
            let mut sign = 0;
            for i in 0..n {
                let dx1 = points[(i + 2) % n].x as i32 - points[(i + 1) % n].x as i32;
                let dy1 = points[(i + 2) % n].y as i32 - points[(i + 1) % n].y as i32;
                let dx2 = points[i].x as i32 - points[(i + 1) % n].x as i32;
                let dy2 = points[i].y as i32 - points[(i + 1) % n].y as i32;
                let zcrossproduct = dx1 * dy2 - dy1 * dx2;
                let new_sign = zcrossproduct.signum();
                if new_sign != 0 {
                    if sign != 0 && new_sign != sign {
                        return false;
                    }
                    sign = new_sign;
                }
            }
            true
        }

        // --- Automatic Polygon Type Detection ---
        let detected_type = detect_polygon_type(&self.points);
        let type_str = match detected_type {
            PolygonTypeAuto::Open => "Open (not closed)",
            PolygonTypeAuto::Complex => "Complex (self-intersecting)",
            PolygonTypeAuto::NonConvex => "Non-Convex",
            PolygonTypeAuto::Convex => "Convex",
        };
        ui.label(format!("Polygon Type: {} (auto-detected)", type_str));
        self.polygon_type = detected_type as u8;
        ui.separator();
        // Move polygon buttons
        ui.horizontal(|ui| {
            if ui.button("Up").on_hover_text("Move Up").clicked() {
                for pt in &mut self.points {
                    if pt.y > 0 {
                        pt.y -= 1;
                    }
                }
            }
            if ui.button("Down").on_hover_text("Move Down").clicked() {
                for pt in &mut self.points {
                    pt.y = pt.y.saturating_add(1);
                }
            }
            if ui.button("Left").on_hover_text("Move Left").clicked() {
                for pt in &mut self.points {
                    if pt.x > 0 {
                        pt.x -= 1;
                    }
                }
            }
            if ui.button("Right").on_hover_text("Move Right").clicked() {
                for pt in &mut self.points {
                    pt.x = pt.x.saturating_add(1);
                }
            }
        });

        ui.separator();
        // Local UI toggle for debug points (not stored in object)
        let debug_points_id = egui::Id::new(format!("polygon_debug_points_{}", self.id.value()));
        let mut debug_points = ui
            .ctx()
            .memory_mut(|mem| mem.data.get_temp::<bool>(debug_points_id).unwrap_or(false));
        if ui
            .checkbox(&mut debug_points, "Show Debug Points")
            .changed()
        {
            ui.ctx()
                .memory_mut(|mem| mem.data.insert_temp(debug_points_id, debug_points));
        }
        ui.label("Points:");
        egui::Grid::new("points_grid")
            .striped(true)
            .min_col_width(0.0)
            .show(ui, |ui| {
                let mut idx = 0;
                while idx < self.points.len() {
                    ui.label(format!("Point {}", idx));
                    ui.add(egui::DragValue::new(&mut self.points[idx].x).speed(1.0));
                    ui.add(egui::DragValue::new(&mut self.points[idx].y).speed(1.0));

                    if ui
                        .add_enabled(idx > 0, egui::Button::new("\u{23F6}"))
                        .on_hover_text("Move Up")
                        .clicked()
                    {
                        self.points.swap(idx, idx - 1);
                    }

                    if ui
                        .add_enabled(idx < self.points.len() - 1, egui::Button::new("\u{23F7}"))
                        .on_hover_text("Move Down")
                        .clicked()
                    {
                        self.points.swap(idx, idx + 1);
                    }
                    if self.points.len() > 3 {
                        if ui
                            .add(egui::Button::new("\u{1F5D9}"))
                            .on_hover_text("Remove")
                            .clicked()
                        {
                            self.points.remove(idx);
                            continue; // Skip incrementing idx since we removed this item
                        }
                    }

                    idx += 1;
                    ui.end_row();
                }
            });

        if ui.button("Add Point").clicked() {
            self.points.push(Point { x: 0, y: 0 });
        }

        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for OutputMeter {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);

        ui.add(
            egui::Slider::new(&mut self.width, 0..=design.mask_size)
                .text("Width")
                .drag_value_speed(1.0),
        );

        ui.add(
            egui::Slider::new(&mut self.needle_colour, 0..=255)
                .text("Needle Colour")
                .drag_value_speed(1.0),
        );

        ui.add(
            egui::Slider::new(&mut self.border_colour, 0..=255)
                .text("Border Colour")
                .drag_value_speed(1.0),
        );

        ui.add(
            egui::Slider::new(&mut self.arc_and_tick_colour, 0..=255)
                .text("Arc & Tick Colour")
                .drag_value_speed(1.0),
        );

        ui.checkbox(&mut self.options.draw_arc, "Draw Arc");
        ui.checkbox(&mut self.options.draw_border, "Draw Border");
        ui.checkbox(&mut self.options.draw_ticks, "Draw Ticks");

        ui.horizontal(|ui| {
            ui.label("Deflection Direction:");
            ui.radio_value(
                &mut self.options.deflection_direction,
                DeflectionDirection::AntiClockwise,
                "Anti-clockwise",
            );
            ui.radio_value(
                &mut self.options.deflection_direction,
                DeflectionDirection::Clockwise,
                "Clockwise",
            );

            // --- Polygon Type Detection Helpers ---
            #[derive(Copy, Clone, Debug, PartialEq, Eq)]
            enum PolygonTypeAuto {
                Convex = 0,
                NonConvex = 1,
                Complex = 2,
                Open = 3,
            }

            fn detect_polygon_type(points: &[Point<u16>]) -> PolygonTypeAuto {
                if points.len() < 3 {
                    return PolygonTypeAuto::Open;
                }
                // Check if open (first != last)
                if points.first() != points.last() {
                    // If the first and last points are not the same, treat as open
                    return PolygonTypeAuto::Open;
                }
                // Check for self-intersection (complex)
                if is_self_intersecting(points) {
                    return PolygonTypeAuto::Complex;
                }
                // Check convexity
                if is_convex(points) {
                    PolygonTypeAuto::Convex
                } else {
                    PolygonTypeAuto::NonConvex
                }
            }

            fn is_self_intersecting(points: &[Point<u16>]) -> bool {
                // Simple O(n^2) check for edge intersection (excluding adjacent edges)
                let n = points.len();
                for i in 0..n {
                    let a1 = &points[i];
                    let a2 = &points[(i + 1) % n];
                    for j in (i + 1)..n {
                        // Skip adjacent edges
                        if (i + 1) % n == j || i == (j + 1) % n {
                            continue;
                        }
                        let b1 = &points[j];
                        let b2 = &points[(j + 1) % n];
                        if segments_intersect(a1, a2, b1, b2) {
                            return true;
                        }
                    }
                }
                false
            }

            fn segments_intersect(
                a1: &Point<u16>,
                a2: &Point<u16>,
                b1: &Point<u16>,
                b2: &Point<u16>,
            ) -> bool {
                fn ccw(p1: &Point<u16>, p2: &Point<u16>, p3: &Point<u16>) -> bool {
                    (p3.y as i32 - p1.y as i32) * (p2.x as i32 - p1.x as i32)
                        > (p2.y as i32 - p1.y as i32) * (p3.x as i32 - p1.x as i32)
                }
                (ccw(a1, b1, b2) != ccw(a2, b1, b2)) && (ccw(a1, a2, b1) != ccw(a1, a2, b2))
            }

            fn is_convex(points: &[Point<u16>]) -> bool {
                let n = points.len();
                if n < 4 {
                    return true; // triangle is always convex
                }
                let mut sign = 0;
                for i in 0..n {
                    let dx1 = points[(i + 2) % n].x as i32 - points[(i + 1) % n].x as i32;
                    let dy1 = points[(i + 2) % n].y as i32 - points[(i + 1) % n].y as i32;
                    let dx2 = points[i].x as i32 - points[(i + 1) % n].x as i32;
                    let dy2 = points[i].y as i32 - points[(i + 1) % n].y as i32;
                    let zcrossproduct = dx1 * dy2 - dy1 * dx2;
                    let new_sign = zcrossproduct.signum();
                    if new_sign != 0 {
                        if sign != 0 && new_sign != sign {
                            return false;
                        }
                        sign = new_sign;
                    }
                }
                true
            }
        });

        ui.add(
            egui::DragValue::new(&mut self.nr_of_ticks)
                .speed(1.0)
                .prefix("Number of Ticks: "),
        );
        ui.add(
            egui::DragValue::new(&mut self.start_angle)
                .speed(1.0)
                .prefix("Start Angle: ")
                .range(0..=180),
        );
        ui.add(
            egui::DragValue::new(&mut self.end_angle)
                .speed(1.0)
                .prefix("End Angle: ")
                .range(0..=180),
        );
        ui.add(
            egui::DragValue::new(&mut self.min_value)
                .speed(1.0)
                .prefix("Min Value: "),
        );
        ui.add(
            egui::DragValue::new(&mut self.max_value)
                .speed(1.0)
                .prefix("Max Value: "),
        );

        ui.horizontal(|ui| {
            ui.label("Variable Reference:");
            egui::ComboBox::from_id_salt("variable_reference")
                .selected_text(
                    self.variable_reference
                        .0
                        .map_or("None".to_string(), |id| format!("{:?}", u16::from(id))),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.variable_reference,
                        NullableObjectId::NULL,
                        "None",
                    );
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::NumberVariable)
                    {
                        ui.selectable_value(
                            &mut self.variable_reference,
                            potential_child.id().into(),
                            format!(
                                "{:?}: {:?}",
                                u16::from(potential_child.id()),
                                potential_child.object_type()
                            ),
                        );
                    }
                });
        });

        // If there's no variable reference, allow editing the initial value
        if self.variable_reference.0.is_none() {
            ui.label("Initial value:");
            ui.add(egui::DragValue::new(&mut self.value).speed(1.0));
        }

        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for OutputLinearBarGraph {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);

        ui.add(
            egui::Slider::new(&mut self.width, 0..=design.mask_size)
                .text("Width")
                .drag_value_speed(1.0),
        );
        ui.add(
            egui::Slider::new(&mut self.height, 0..=design.mask_size)
                .text("Height")
                .drag_value_speed(1.0),
        );

        color_swatch_selector(
            ui,
            &mut self.colour,
            design.get_pool().get_colour_palette(),
            "Bar Colour",
        );
        if self.options.draw_target_line {
            ui.add(
                egui::Slider::new(&mut self.target_line_colour, 0..=255)
                    .text("Target Line Colour")
                    .drag_value_speed(1.0),
            );
        }

        ui.checkbox(&mut self.options.draw_border, "Draw Border");
        ui.checkbox(&mut self.options.draw_target_line, "Draw Target Line");
        ui.checkbox(&mut self.options.draw_ticks, "Draw Ticks");
        ui.horizontal(|ui| {
            ui.label("Bar Graph Type:");
            ui.radio_value(
                &mut self.options.bar_graph_type,
                BarGraphType::Filled,
                "Filled",
            );
            ui.radio_value(
                &mut self.options.bar_graph_type,
                BarGraphType::NotFilled,
                "Not Filled",
            );
        });

        ui.horizontal(|ui| {
            ui.label("Axis Orientation:");
            ui.radio_value(
                &mut self.options.axis_orientation,
                AxisOrientation::Vertical,
                "Vertical",
            );
            ui.radio_value(
                &mut self.options.axis_orientation,
                AxisOrientation::Horizontal,
                "Horizontal",
            );
        });

        ui.horizontal(|ui| {
            ui.label("Grow Direction:");
            ui.radio_value(
                &mut self.options.grow_direction,
                GrowDirection::GrowLeftDown,
                "Left/Down",
            );
            ui.radio_value(
                &mut self.options.grow_direction,
                GrowDirection::GrowRightUp,
                "Right/Up",
            );
        });

        if self.options.draw_ticks {
            ui.add(
                egui::DragValue::new(&mut self.nr_of_ticks)
                    .speed(1.0)
                    .prefix("Number of Ticks: "),
            );
        }
        ui.add(
            egui::DragValue::new(&mut self.min_value)
                .speed(1.0)
                .prefix("Min Value: "),
        );
        ui.add(
            egui::DragValue::new(&mut self.max_value)
                .speed(1.0)
                .prefix("Max Value: "),
        );

        ui.horizontal(|ui| {
            ui.label("Variable Reference:");
            egui::ComboBox::from_id_salt("variable_reference")
                .selected_text(
                    self.variable_reference
                        .0
                        .map_or("None".to_string(), |id| format!("{:?}", u16::from(id))),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.variable_reference,
                        NullableObjectId::NULL,
                        "None",
                    );
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::NumberVariable)
                    {
                        ui.selectable_value(
                            &mut self.variable_reference,
                            potential_child.id().into(),
                            format!(
                                "{:?}: {:?}",
                                u16::from(potential_child.id()),
                                potential_child.object_type()
                            ),
                        );
                    }
                });
        });

        // If no variable reference, allow setting initial value manually
        if self.variable_reference.0.is_none() {
            ui.label("Initial Value:");
            ui.add(egui::DragValue::new(&mut self.value).speed(1.0));
        }

        ui.horizontal(|ui| {
            ui.label("Target Value Variable Reference:");
            egui::ComboBox::from_id_salt("target_value_variable_reference")
                .selected_text(
                    self.target_value_variable_reference
                        .0
                        .map_or("None".to_string(), |id| format!("{:?}", u16::from(id))),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.target_value_variable_reference,
                        NullableObjectId::NULL,
                        "None",
                    );
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::NumberVariable)
                    {
                        ui.selectable_value(
                            &mut self.target_value_variable_reference,
                            potential_child.id().into(),
                            format!(
                                "{:?}: {:?}",
                                u16::from(potential_child.id()),
                                potential_child.object_type()
                            ),
                        );
                    }
                });
        });

        // If no target value variable reference, allow setting target value manually
        if self.target_value_variable_reference.0.is_none() {
            ui.label("Target Value:");
            ui.add(egui::DragValue::new(&mut self.target_value).speed(1.0));
        }

        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for OutputArchedBarGraph {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);

        ui.add(
            egui::Slider::new(&mut self.width, 0..=design.mask_size)
                .text("Width")
                .drag_value_speed(1.0),
        );
        ui.add(
            egui::Slider::new(&mut self.height, 0..=design.mask_size)
                .text("Height")
                .drag_value_speed(1.0),
        );

        color_swatch_selector(
            ui,
            &mut self.colour,
            design.get_pool().get_colour_palette(),
            "Bar Colour",
        );
        if self.options.draw_target_line {
            ui.add(
                egui::Slider::new(&mut self.target_line_colour, 0..=255)
                    .text("Target Line Colour")
                    .drag_value_speed(1.0),
            );
        }

        ui.checkbox(&mut self.options.draw_border, "Draw Border");
        ui.checkbox(&mut self.options.draw_target_line, "Draw Target Line");

        ui.horizontal(|ui| {
            ui.label("Bar Graph Type:");
            ui.radio_value(
                &mut self.options.bar_graph_type,
                BarGraphType::Filled,
                "Filled",
            );
            ui.radio_value(
                &mut self.options.bar_graph_type,
                BarGraphType::NotFilled,
                "Not Filled",
            );
        });

        ui.horizontal(|ui| {
            ui.label("Axis Orientation:");
            ui.radio_value(
                &mut self.options.axis_orientation,
                AxisOrientation::Vertical,
                "Vertical",
            );
            ui.radio_value(
                &mut self.options.axis_orientation,
                AxisOrientation::Horizontal,
                "Horizontal",
            );
        });

        ui.horizontal(|ui| {
            ui.label("Grow Direction:");
            ui.radio_value(
                &mut self.options.grow_direction,
                GrowDirection::GrowLeftDown,
                "Left/Down",
            );
            ui.radio_value(
                &mut self.options.grow_direction,
                GrowDirection::GrowRightUp,
                "Right/Up",
            );
        });

        ui.horizontal(|ui| {
            ui.label("Deflection Direction:");
            ui.radio_value(
                &mut self.options.deflection_direction,
                DeflectionDirection::AntiClockwise,
                "Anti-clockwise",
            );
            ui.radio_value(
                &mut self.options.deflection_direction,
                DeflectionDirection::Clockwise,
                "Clockwise",
            );
        });

        ui.add(
            egui::DragValue::new(&mut self.start_angle)
                .speed(1.0)
                .prefix("Start Angle: "),
        );
        ui.add(
            egui::DragValue::new(&mut self.end_angle)
                .speed(1.0)
                .prefix("End Angle: "),
        );
        ui.add(
            egui::DragValue::new(&mut self.bar_graph_width)
                .speed(1.0)
                .prefix("Bar Graph Width: "),
        );
        ui.add(
            egui::DragValue::new(&mut self.min_value)
                .speed(1.0)
                .prefix("Min Value: "),
        );
        ui.add(
            egui::DragValue::new(&mut self.max_value)
                .speed(1.0)
                .prefix("Max Value: "),
        );

        ui.horizontal(|ui| {
            ui.label("Variable Reference:");
            egui::ComboBox::from_id_salt("variable_reference")
                .selected_text(
                    self.variable_reference
                        .0
                        .map_or("None".to_string(), |id| format!("{:?}", u16::from(id))),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.variable_reference,
                        NullableObjectId::NULL,
                        "None",
                    );
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::NumberVariable)
                    {
                        ui.selectable_value(
                            &mut self.variable_reference,
                            potential_child.id().into(),
                            format!(
                                "{:?}: {:?}",
                                u16::from(potential_child.id()),
                                potential_child.object_type()
                            ),
                        );
                    }
                });
        });

        // If no variable reference, set initial value
        if self.variable_reference.0.is_none() {
            ui.label("Initial Value:");
            ui.add(egui::DragValue::new(&mut self.value).speed(1.0));
        }

        ui.horizontal(|ui| {
            ui.label("Target Value Variable Reference:");
            egui::ComboBox::from_id_salt("target_value_variable_reference")
                .selected_text(
                    self.target_value_variable_reference
                        .0
                        .map_or("None".to_string(), |id| format!("{:?}", u16::from(id))),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.target_value_variable_reference,
                        NullableObjectId::NULL,
                        "None",
                    );
                    for potential_child in design
                        .get_pool()
                        .objects_by_type(ObjectType::NumberVariable)
                    {
                        ui.selectable_value(
                            &mut self.target_value_variable_reference,
                            potential_child.id().into(),
                            format!(
                                "{:?}: {:?}",
                                u16::from(potential_child.id()),
                                potential_child.object_type()
                            ),
                        );
                    }
                });
        });

        // If no target value variable reference, set target value
        if self.target_value_variable_reference.0.is_none() {
            ui.label("Target Value:");
            ui.add(egui::DragValue::new(&mut self.target_value).speed(1.0));
        }

        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for PictureGraphic {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);
        ui.add(
            egui::Slider::new(&mut self.width, 0..=design.mask_size)
                .text("Width")
                .drag_value_speed(1.0),
        );
        ui.label(format!("Actual Image Width: {}", self.actual_width));
        ui.label(format!("Actual Image Height: {}", self.actual_height));
        ui.horizontal(|ui| {
            ui.label("Format:");
            if ui
                .radio(
                    self.format == PictureGraphicFormat::Monochrome,
                    "Monochrome",
                )
                .clicked()
            {
                match self.format {
                    PictureGraphicFormat::FourBit => {
                        self.data = self
                            .data_as_raw_encoded()
                            .windows(4)
                            .step_by(4)
                            .flat_map(|chunk| {
                                let mut byte = 0;
                                for (i, bit) in chunk.iter().enumerate() {
                                    for j in 0..2 {
                                        if *bit & (1 << j) != 0 {
                                            byte |= 1 << (i * 2 + j);
                                        }
                                    }
                                }
                                vec![byte]
                            })
                            .collect();
                    }
                    PictureGraphicFormat::EightBit => {
                        self.data = self
                            .data_as_raw_encoded()
                            .windows(8)
                            .step_by(8)
                            .flat_map(|chunk| {
                                let mut byte = 0;
                                for (i, bit) in chunk.iter().enumerate() {
                                    if *bit != 0 {
                                        byte |= 1 << i;
                                    }
                                }
                                vec![byte]
                            })
                            .collect();
                    }
                    _ => {}
                }
                self.format = PictureGraphicFormat::Monochrome;
                self.options.data_code_type = DataCodeType::Raw;
            }
            if ui
                .radio(self.format == PictureGraphicFormat::FourBit, "4-bit colour")
                .clicked()
            {
                match self.format {
                    PictureGraphicFormat::Monochrome => {
                        self.data = self
                            .data_as_raw_encoded()
                            .iter()
                            .flat_map(|value| {
                                let mut result = vec![];
                                for idx in 0..8 {
                                    let bit_color = value << idx & 0x01;
                                    if idx % 2 == 0 {
                                        result.push(bit_color);
                                    } else if let Some(last) = result.last_mut() {
                                        *last |= bit_color >> 4;
                                    }
                                }
                                result
                            })
                            .collect();
                    }
                    PictureGraphicFormat::EightBit => {
                        self.data = self
                            .data_as_raw_encoded()
                            .windows(2)
                            .step_by(2)
                            .flat_map(|values| {
                                let high = (values[0] & 0x0F) << 4;
                                let low = values[1] & 0x0F;
                                vec![high | low]
                            })
                            .collect();
                    }
                    _ => {}
                }
                self.format = PictureGraphicFormat::FourBit;
                self.options.data_code_type = DataCodeType::Raw;
            }
            if ui
                .radio(
                    self.format == PictureGraphicFormat::EightBit,
                    "8-bit colour",
                )
                .clicked()
            {
                match self.format {
                    PictureGraphicFormat::Monochrome => {
                        self.data = self
                            .data_as_raw_encoded()
                            .iter()
                            .flat_map(|value| {
                                let mut result = vec![];
                                for bit in 0..8 {
                                    result.push(value >> bit & 0x01);
                                }
                                result
                            })
                            .collect();
                    }
                    PictureGraphicFormat::FourBit => {
                        self.data = self
                            .data_as_raw_encoded()
                            .iter()
                            .flat_map(|value| {
                                let high = (value >> 4) & 0x0F;
                                let low = value & 0x0F;
                                vec![high, low]
                            })
                            .collect();
                    }
                    _ => {}
                }
                self.format = PictureGraphicFormat::EightBit;
                self.options.data_code_type = DataCodeType::Raw;
            }
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.options.transparent, "Transparent Pixels");
            if self.options.transparent {
                ui.add(
                    egui::Slider::new(&mut self.transparency_colour, 0..=255)
                        .text("Transparent Colour")
                        .drag_value_speed(1.0),
                );
            }
        });
        ui.checkbox(&mut self.options.flashing, "Flashing");

        ui.separator();
        ui.label("Image:");
        if ui
            .button("Load Image")
            .on_hover_text("Load an image file (PNG, JPG, BMP, etc.)")
            .clicked()
        {
            design.request_image_load(self.id);
        }

        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for NumberVariable {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);

        ui.horizontal(|ui| {
            ui.label("Initial Value:");
            ui.add(egui::DragValue::new(&mut self.value).speed(1.0));
        });
    }
}

impl ConfigurableObject for StringVariable {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);

        ui.horizontal(|ui| {
            ui.label("Initial Value:");
            ui.text_edit_singleline(&mut self.value);
        });
    }
}

impl ConfigurableObject for FontAttributes {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);

        color_swatch_selector(
            ui,
            &mut self.font_colour,
            design.get_pool().get_colour_palette(),
            "Font Colour",
        );

        // let is_proportional = self.font_style.proportional; // TODO: check if we have VT version 4 or later
        let is_proportional = false;

        // If proportional bit is set, font_size is proportional, otherwise non-proportional.
        if is_proportional {
            // Proportional font: we have a pixel height
            let mut height = match self.font_size {
                FontSize::Proportional(h) => h,
                FontSize::NonProportional(_) => 8, // default to minimal proportional height if needed
            };
            ui.horizontal(|ui| {
                ui.label("Proportional Font Height (≥ 8):");
                if ui.add(egui::DragValue::new(&mut height)).changed() {
                    self.font_size = FontSize::Proportional(height);
                }
            });
        } else {
            // Non-proportional font sizes: combo box
            let current_size = match &self.font_size {
                FontSize::NonProportional(s) => *s,
                FontSize::Proportional(_) => NonProportionalFontSize::Px6x8,
            };

            egui::ComboBox::from_label("Non-Proportional Font Size")
                .selected_text(format!("{:?}", current_size))
                .show_ui(ui, |ui| {
                    for value in [
                        NonProportionalFontSize::Px6x8,
                        NonProportionalFontSize::Px8x8,
                        NonProportionalFontSize::Px8x12,
                        NonProportionalFontSize::Px12x16,
                        NonProportionalFontSize::Px16x16,
                        NonProportionalFontSize::Px16x24,
                        NonProportionalFontSize::Px24x32,
                        NonProportionalFontSize::Px32x32,
                        NonProportionalFontSize::Px32x48,
                        NonProportionalFontSize::Px48x64,
                        NonProportionalFontSize::Px64x64,
                        NonProportionalFontSize::Px64x96,
                        NonProportionalFontSize::Px96x128,
                        NonProportionalFontSize::Px128x128,
                        NonProportionalFontSize::Px128x192,
                    ] {
                        ui.selectable_value(
                            &mut self.font_size,
                            FontSize::NonProportional(value),
                            format!("{:?}", value),
                        );
                    }
                });
        }

        ui.separator();
        let mut is_proprietary = if let FontType::Proprietary(_) = self.font_type {
            true
        } else {
            false
        };
        ui.checkbox(&mut is_proprietary, "Proprietary Font");

        if is_proprietary {
            const PROPRIETARY_RANGE_V3_AND_PRIOR: std::ops::RangeInclusive<u8> = 255..=255;
            const PROPRIETARY_RANGE_V4_AND_LATER: std::ops::RangeInclusive<u8> = 240..=255;

            let range = PROPRIETARY_RANGE_V3_AND_PRIOR; // TODO: check if we have VT version 4 or later

            let mut raw_value = match self.font_type {
                FontType::Proprietary(v) => v,
                _ => range.clone().last().unwrap(),
            };
            ui.horizontal(|ui| {
                ui.label("Proprietary Font Value:");
                ui.add(egui::DragValue::new(&mut raw_value).range(range).speed(1.0));
            });
            self.font_type = FontType::Proprietary(raw_value);
        } else {
            // Reset to Latin1 if we were proprietary or reserved
            match self.font_type {
                FontType::Proprietary(_) | FontType::Reserved(_) => {
                    self.font_type = FontType::Latin1;
                }
                _ => {}
            }

            ui.horizontal(|ui| {
                ui.label("Font Type:");
                egui::ComboBox::from_id_salt("font_type")
                    .selected_text(format!("{:?}", self.font_type))
                    .show_ui(ui, |ui| {
                        // Known fonts
                        for value in &[
                            FontType::Latin1,
                            FontType::Latin9,
                            // TODO: check if we have VT version 4 or later
                            // FontType::Latin2,
                            // FontType::Latin4,
                            // FontType::Cyrillic,
                            // FontType::Greek,
                        ] {
                            if ui
                                .selectable_label(&self.font_type == value, format!("{:?}", value))
                                .clicked()
                            {
                                self.font_type = value.clone();
                            }
                        }
                    });
            });
        }

        ui.separator();
        ui.label("Font Style:");
        ui.checkbox(&mut self.font_style.bold, "Bold");
        ui.checkbox(&mut self.font_style.crossed_out, "Crossed Out");
        ui.checkbox(&mut self.font_style.underlined, "Underlined");
        ui.checkbox(&mut self.font_style.italic, "Italic");
        ui.checkbox(&mut self.font_style.inverted, "Inverted");
        ui.checkbox(&mut self.font_style.flashing_inverted, "Flashing Inverted");
        ui.checkbox(&mut self.font_style.flashing_hidden, "Flashing Hidden");
        if settings.vt_version >= VtVersion::Version4 {
            ui.checkbox(&mut self.font_style.proportional, "Proportional"); // TODO: check if we have VT version 4 or later
        }

        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for LineAttributes {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);

        color_swatch_selector(
            ui,
            &mut self.line_colour,
            design.get_pool().get_colour_palette(),
            "Line Colour",
        );

        ui.add(
            egui::Slider::new(&mut self.line_width, 0..=255)
                .text("Line Width")
                .drag_value_speed(1.0),
        );

        ui.label("Line Art Pattern (16 bits):")
            .on_hover_text("Each bit in this 16-bit pattern represents a 'paintbrush spot' along the line. ")
            .on_hover_text("A '1' bit means that spot is drawn in the line color, while a '0' bit means that spot is skipped (shows background).");

        ui.horizontal(|ui| {
            for i in (0..16).rev() {
                let bit_mask = 1 << i;
                let mut bit_is_set = (self.line_art & bit_mask) != 0;
                let check = ui.checkbox(&mut bit_is_set, "");
                if check.changed() {
                    if bit_is_set {
                        self.line_art |= bit_mask;
                    } else {
                        self.line_art &= !bit_mask;
                    }
                }
                check.on_hover_text(format!(
                    "Bit {}: {} ({}). Click to toggle.\n1 = Draw line colour\n0 = Skip (background)",
                    i,
                    if bit_is_set { "Currently: Draw" } else { "Currently: Skip" },
                    if bit_is_set { "One (1)" } else { "Zero (0)" }
                ));
            }
        });

        ui.horizontal(|ui| {
            ui.label("Current Binary Pattern:");
            ui.label(format!("{:016b}", self.line_art))
                .on_hover_text("This shows the full 16-bit pattern of the line art. '1' bits represent drawn spots; '0' bits represent skipped spots.");
        });

        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for FillAttributes {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);
        ui.label("Fill Type:").on_hover_text(
            "Select how this area should be filled:\n\
                            0 = No fill\n\
                            1 = Fill with line colour\n\
                            2 = Fill with a specified fill colour\n\
                            3 = Fill with a specified pattern (PictureGraphic)",
        );

        ui.horizontal(|ui| {
            ui.radio_value(&mut self.fill_type, 0, "No fill")
                .on_hover_text("No fill will be drawn, the background will be visible.");
            ui.radio_value(&mut self.fill_type, 1, "Fill with line colour")
                .on_hover_text("The area will be filled using the currently set line colour of the parent shape.");
            ui.radio_value(&mut self.fill_type, 2, "Fill with specified colour")
                .on_hover_text("The area will be filled using the 'fill_colour' attribute specified below.");
            ui.radio_value(&mut self.fill_type, 3, "Fill with pattern")
                .on_hover_text("The area will be filled using a pattern defined by a PictureGraphic object referenced below.");
        });

        if self.fill_type == 2 {
            color_swatch_selector(
                ui,
                &mut self.fill_colour,
                design.get_pool().get_colour_palette(),
                "Fill Colour",
            );
        } else if self.fill_type == 3 {
            ui.label("Fill Pattern (PictureGraphic Object):")
                .on_hover_text("Select a PictureGraphic object to use as a pattern.\n\
                                Make sure the PictureGraphic width and format match the restrictions.");
            // Render a nullable object selector restricted to PictureGraphic objects
            ui.horizontal(|ui| {
                render_nullable_object_id_selector(
                    ui,
                    0,
                    design,
                    &mut self.fill_pattern,
                    &[ObjectType::PictureGraphic],
                    Some(self.id),
                );

                if let Some(pattern_id) = self.fill_pattern.0 {
                    if design.get_pool().object_by_id(pattern_id).is_some() {
                        if ui.link("(view)").clicked() {
                            *design.get_mut_selected().borrow_mut() = pattern_id.into();
                        }
                    } else {
                        ui.colored_label(egui::Color32::RED, "Missing pattern object");
                    }
                } else {
                    ui.label("None");
                }
            });
        }

        ui.separator();
        ui.label("Macros:")
            .on_hover_text("Define macros that could be triggered by events associated with this object.\n\
                            Currently, FillAttributes does not trigger events, but this is included for consistency.");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for InputAttributes {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);

        ui.horizontal(|ui| {
            ui.label("Validation Type:");
            ui.radio_value(
                &mut self.validation_type,
                ValidationType::ValidCharacters,
                "Valid Characters",
            );
            ui.radio_value(
                &mut self.validation_type,
                ValidationType::InvalidCharacters,
                "Invalid Characters",
            );
        });

        ui.label("Validation String:");
        ui.text_edit_singleline(&mut self.validation_string);

        ui.separator();
        ui.label("Macros:");
        render_macro_references(
            ui,
            design,
            &mut self.macro_refs,
            &Self::get_possible_events(),
        );
    }
}

impl ConfigurableObject for ObjectPointer {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);
        ui.horizontal(|ui| {
            ui.label("Object reference:");
            egui::ComboBox::from_id_salt("object_reference")
                .selected_text(format!("{:?}", u16::from(self.value)))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.value, NullableObjectId::NULL, "None");
                    let object_types: Vec<ObjectType> = design
                        .get_pool()
                        .parent_objects(self.id)
                        .iter()
                        .flat_map(|parent_obj| {
                            get_allowed_child_refs(parent_obj.object_type(), _settings.vt_version)
                                .into_iter()
                        })
                        .collect();
                    for potential_child in design.get_pool().objects_by_types(&object_types) {
                        ui.selectable_value(
                            &mut self.value,
                            potential_child.id().into(),
                            format!(
                                "{:?}: {:?}",
                                u16::from(potential_child.id()),
                                potential_child.object_type()
                            ),
                        );
                    }
                });
            if let Some(id) = self.value.into() {
                if let Some(object) = design.get_pool().object_by_id(id) {
                    if ui.link(format!("{:?}", object.object_type())).clicked() {
                        *design.get_mut_selected().borrow_mut() = id.into();
                    }
                } else {
                    ui.colored_label(egui::Color32::RED, "Missing object in pool");
                }
            }
        });
    }
}

const ALLOWED_MACRO_COMMANDS: &[(u8, &str, VtVersion)] = &[
    (0xA0, "Hide/Show Object command", VtVersion::Version2),
    (0xA1, "Enable/Disable Object command", VtVersion::Version2),
    (0xA2, "Select Input Object command", VtVersion::Version2),
    (0x92, "ESC command", VtVersion::Version2),
    (0xA3, "Control Audio Signal command", VtVersion::Version2),
    (0xA4, "Set Audio Volume command", VtVersion::Version2),
    (0xA5, "Change Child Location command", VtVersion::Version2),
    (0xB4, "Change Child Position command", VtVersion::Version2),
    (0xA6, "Change Size command", VtVersion::Version2),
    (
        0xA7,
        "Change Background Colour command",
        VtVersion::Version2,
    ),
    (0xA8, "Change Numeric Value command", VtVersion::Version2),
    (0xB3, "Change String Value command", VtVersion::Version2),
    (0xA9, "Change End Point command", VtVersion::Version2),
    (0xAA, "Change Font Attributes command", VtVersion::Version2),
    (0xAB, "Change Line Attributes command", VtVersion::Version2),
    (0xAC, "Change Fill Attributes command", VtVersion::Version2),
    (0xAD, "Change Active Mask command", VtVersion::Version2),
    (0xAE, "Change Soft Key Mask command", VtVersion::Version2),
    (0xAF, "Change Attribute command", VtVersion::Version2),
    (0xB0, "Change priority command", VtVersion::Version2),
    (0xB1, "Change List item command", VtVersion::Version2),
    (0xBD, "Lock/Unlock Mask command", VtVersion::Version4),
    (0xBE, "Execute Macro command", VtVersion::Version4),
    (0xB5, "Change Object Label command", VtVersion::Version4),
    (0xB6, "Change Polygon Point command", VtVersion::Version4),
    (0xB7, "Change Polygon Scale command", VtVersion::Version4),
    (0xB8, "Graphics Context command", VtVersion::Version4),
    (
        0xBA,
        "Select Colour Map or Palette command",
        VtVersion::Version4,
    ),
    (0xBC, "Execute Extended Macro command", VtVersion::Version5),
    (
        0x90,
        "Select Active Working Set command",
        VtVersion::Version6,
    ),
];

impl ConfigurableObject for Macro {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);

        ui.label("Macro Commands:");
        egui::Grid::new("macro_commands_grid")
            .striped(true)
            .min_col_width(0.0)
            .show(ui, |ui| {
                let mut idx = 0;
                while idx < self.commands.len() {
                    let code = self.commands[idx];
                    let command_name = ALLOWED_MACRO_COMMANDS
                        .iter()
                        .find(|&&(c, _, __)| c == code)
                        .map(|&(_, name, __)| name)
                        .unwrap_or("Unknown");

                    ui.label(format!("0x{:02X}", code));
                    ui.label(command_name);
                    render_index_modifiers(ui, idx, &mut self.commands);
                    ui.end_row();

                    idx += 1;
                }
            });

        ui.horizontal(|ui| {
            ui.label("Add command:");
            egui::ComboBox::from_id_salt("add_macro_command")
                .selected_text("Select command")
                .show_ui(ui, |ui| {
                    for &(code, name, version) in ALLOWED_MACRO_COMMANDS {
                        if version > VtVersion::Version3 {
                            continue; // TODO: check which version pool we have
                        }

                        if ui
                            .selectable_label(false, format!("0x{:02X} {}", code, name))
                            .clicked()
                        {
                            self.commands.push(code);
                        }
                    }
                });
        });
    }
}

impl ConfigurableObject for AuxiliaryFunctionType2 {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);

        ui.add(
            egui::Slider::new(&mut self.background_colour, 0..=255)
                .text("Background Colour")
                .drag_value_speed(1.0),
        );

        ui.horizontal(|ui| {
            ui.label("Function Type:");
            egui::ComboBox::from_id_salt("function_type")
                .selected_text(format!("{:?}", self.function_attributes.function_type))
                .show_ui(ui, |ui| {
                    let selectable_types = &[
                        AuxiliaryFunctionType::BooleanLatching,
                        AuxiliaryFunctionType::AnalogueMaintains,
                        AuxiliaryFunctionType::BooleanNonLatching,
                        AuxiliaryFunctionType::AnalogueReturnToCenter,
                        AuxiliaryFunctionType::AnalogueReturnToZero,
                        AuxiliaryFunctionType::DualBooleanLatching,
                        AuxiliaryFunctionType::DualBooleanNonLatching,
                        AuxiliaryFunctionType::DualBooleanLatchingUp,
                        AuxiliaryFunctionType::DualBooleanLatchingDown,
                        AuxiliaryFunctionType::CombinedAnalogueReturnWithLatch,
                        AuxiliaryFunctionType::CombinedAnalogueMaintainsWithLatch,
                        AuxiliaryFunctionType::QuadratureBooleanNonLatching,
                        AuxiliaryFunctionType::QuadratureAnalogueMaintains,
                        AuxiliaryFunctionType::QuadratureAnalogueReturnToCenter,
                        AuxiliaryFunctionType::BidirectionalEncoder,
                    ];

                    for ft in selectable_types {
                        ui.selectable_value(
                            &mut self.function_attributes.function_type,
                            *ft,
                            format!("{:?}", ft),
                        );
                    }
                });
        });

        ui.checkbox(&mut self.function_attributes.critical, "Critical");
        ui.checkbox(&mut self.function_attributes.restricted, "Restricted");
        ui.checkbox(
            &mut self.function_attributes.single_assignment,
            "Single-assignment",
        );

        ui.separator();
        ui.label("Objects:");
        render_object_references_list(
            ui,
            design,
            design.mask_size,
            design.mask_size,
            &mut self.object_refs,
            &Self::get_allowed_child_refs(VtVersion::Version3),
            self.id,
        );
    }
}

impl ConfigurableObject for AuxiliaryInputType2 {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);

        ui.add(
            egui::Slider::new(&mut self.background_colour, 0..=255)
                .text("Background Colour")
                .drag_value_speed(1.0),
        );

        ui.horizontal(|ui| {
            ui.label("Function Type:");
            egui::ComboBox::from_id_salt("input_function_type")
                .selected_text(format!("{:?}", self.function_attributes.function_type))
                .show_ui(ui, |ui| {
                    let selectable_types = &[
                        AuxiliaryFunctionType::BooleanLatching,
                        AuxiliaryFunctionType::AnalogueMaintains,
                        AuxiliaryFunctionType::BooleanNonLatching,
                        AuxiliaryFunctionType::AnalogueReturnToCenter,
                        AuxiliaryFunctionType::AnalogueReturnToZero,
                        AuxiliaryFunctionType::DualBooleanLatching,
                        AuxiliaryFunctionType::DualBooleanNonLatching,
                        AuxiliaryFunctionType::DualBooleanLatchingUp,
                        AuxiliaryFunctionType::DualBooleanLatchingDown,
                        AuxiliaryFunctionType::CombinedAnalogueReturnWithLatch,
                        AuxiliaryFunctionType::CombinedAnalogueMaintainsWithLatch,
                        AuxiliaryFunctionType::QuadratureBooleanNonLatching,
                        AuxiliaryFunctionType::QuadratureAnalogueMaintains,
                        AuxiliaryFunctionType::QuadratureAnalogueReturnToCenter,
                        AuxiliaryFunctionType::BidirectionalEncoder,
                    ];

                    for ft in selectable_types {
                        ui.selectable_value(
                            &mut self.function_attributes.function_type,
                            *ft,
                            format!("{:?}", ft),
                        );
                    }
                });
        });

        ui.checkbox(&mut self.function_attributes.critical, "Critical");
        ui.checkbox(
            &mut self.function_attributes.single_assignment,
            "Single-assignment",
        );

        ui.separator();
        ui.label("Objects:");
        render_object_references_list(
            ui,
            design,
            design.mask_size,
            design.mask_size,
            &mut self.object_refs,
            &Self::get_allowed_child_refs(_settings.vt_version),
            self.id,
        );
    }
}

impl ConfigurableObject for AuxiliaryControlDesignatorType2 {
    fn render_parameters(
        &mut self,
        ui: &mut egui::Ui,
        design: &EditorProject,
        _settings: &DesignerSettings,
    ) {
        render_object_id(ui, &mut self.id, design);

        ui.horizontal(|ui| {
            ui.label("Pointer Type:");
            egui::ComboBox::from_id_salt("aux_control_pointer_type")
                .selected_text(format!("{}", self.pointer_type))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.pointer_type,
                        0,
                        "0 (Points to Auxiliary Object)",
                    );
                    ui.selectable_value(
                        &mut self.pointer_type,
                        1,
                        "1 (Points to Assigned Aux Objects)",
                    );
                    ui.selectable_value(
                        &mut self.pointer_type,
                        2,
                        "2 (Points to WS Object of this Pool)",
                    );
                    ui.selectable_value(
                        &mut self.pointer_type,
                        3,
                        "3 (Points to WS Object of Assigned)",
                    );
                });
        });

        // According to Table J.6 and J.7, when pointer_type = 2, auxiliary_object_id should be NULL (0xFFFF).
        let must_be_null = self.pointer_type == 2;
        if must_be_null {
            self.auxiliary_object_id = NullableObjectId::NULL;
        } else {
            // Allow user to select an Auxiliary Input or Auxiliary Function object.
            ui.horizontal(|ui| {
                ui.label("Auxiliary Object ID:");
                egui::ComboBox::from_id_salt("aux_object_id_selector")
                    .selected_text(format!("{:?}", u16::from(self.auxiliary_object_id)))
                    .show_ui(ui, |ui| {
                        // Let’s consider that we might assign Auxiliary Function Type 2 (31) or Auxiliary Input Type 2 (32) objects.
                        let allowed_types = &[
                            ObjectType::AuxiliaryFunctionType2,
                            ObjectType::AuxiliaryInputType2,
                        ];

                        for potential_child in design.get_pool().objects_by_types(allowed_types) {
                            if ui
                                .selectable_label(
                                    NullableObjectId::from(potential_child.id())
                                        == self.auxiliary_object_id,
                                    format!(
                                        "{:?}: {:?}",
                                        u16::from(potential_child.id()),
                                        potential_child.object_type()
                                    ),
                                )
                                .clicked()
                            {
                                self.auxiliary_object_id = potential_child.id().into();
                            }
                        }
                    });

                // Provide a link to navigate to the selected object
                if let Some(ref_id) = self.auxiliary_object_id.into() {
                    if let Some(obj) = design.get_pool().object_by_id(ref_id) {
                        if ui.link(format!("{:?}", obj.object_type())).clicked() {
                            *design.get_mut_selected().borrow_mut() = ref_id.into();
                        }
                    } else {
                        ui.colored_label(egui::Color32::RED, "Missing object in pool");
                    }
                }
            });
        }
    }
}
