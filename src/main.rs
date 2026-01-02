/// Remap all internal ObjectId references in an object using the provided id_map
fn remap_object_references(
    obj: &mut Object,
    id_map: &std::collections::HashMap<ObjectId, ObjectId>,
) {
    match obj {
        Object::WorkingSet(ws) => {
            // Remap active_mask
            if let Some(new_id) = id_map.get(&ws.active_mask) {
                ws.active_mask = *new_id;
            }
            // Remap object_refs
            for obj_ref in &mut ws.object_refs {
                if let Some(new_id) = id_map.get(&obj_ref.id) {
                    obj_ref.id = *new_id;
                }
            }
        }
        Object::DataMask(dm) => {
            // Remap soft_key_mask
            if let Some(old_id) = dm.soft_key_mask.into() {
                if let Some(new_id) = id_map.get(&old_id) {
                    dm.soft_key_mask = (*new_id).into();
                }
            }
            // Remap object_refs
            for obj_ref in &mut dm.object_refs {
                if let Some(new_id) = id_map.get(&obj_ref.id) {
                    obj_ref.id = *new_id;
                }
            }
        }
        Object::AlarmMask(am) => {
            // Remap soft_key_mask
            if let Some(old_id) = am.soft_key_mask.into() {
                if let Some(new_id) = id_map.get(&old_id) {
                    am.soft_key_mask = (*new_id).into();
                }
            }
            // Remap object_refs
            for obj_ref in &mut am.object_refs {
                if let Some(new_id) = id_map.get(&obj_ref.id) {
                    obj_ref.id = *new_id;
                }
            }
        }
        Object::Container(c) => {
            for obj_ref in &mut c.object_refs {
                if let Some(new_id) = id_map.get(&obj_ref.id) {
                    obj_ref.id = *new_id;
                }
            }
        }
        Object::SoftKeyMask(sk) => {
            for obj_id in &mut sk.objects {
                if let Some(old_id) = obj_id.0 {
                    if let Some(new_id) = id_map.get(&old_id) {
                        *obj_id = (*new_id).into();
                    }
                }
            }
        }
        Object::Key(k) => {
            for obj_ref in &mut k.object_refs {
                if let Some(new_id) = id_map.get(&obj_ref.id) {
                    obj_ref.id = *new_id;
                }
            }
        }
        Object::Button(b) => {
            for obj_ref in &mut b.object_refs {
                if let Some(new_id) = id_map.get(&obj_ref.id) {
                    obj_ref.id = *new_id;
                }
            }
        }
        // Add similar logic for other object types with object_refs or object id fields as needed
        _ => {}
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CopyPasteAction {
    CopyExact,
    CopyAsNew,
    Paste,
}
// Copyright 2024 - The Open-Agriculture Developers
// SPDX-License-Identifier: GPL-3.0-or-later
// Authors: Daan Steenbergen
use ag_iso_stack::object_pool::object::*;
use ag_iso_stack::object_pool::object_attributes::{DataCodeType, PictureGraphicFormat, Point};
use ag_iso_stack::object_pool::NullableObjectId;
use ag_iso_stack::object_pool::ObjectId;
use ag_iso_stack::object_pool::ObjectPool;
use ag_iso_stack::object_pool::ObjectType;
use ag_iso_terminal_designer::ConfigurableObject;
use ag_iso_terminal_designer::DesignerSettings;
use ag_iso_terminal_designer::EditorProject;
use ag_iso_terminal_designer::InteractiveMaskRenderer;
use ag_iso_terminal_designer::RenderableObject;
use eframe::egui;
use std::future::Future;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;

const OBJECT_HIERARCHY_ID: &str = "object_hierarchy_ui";

enum FileDialogReason {
    LoadPool,
    LoadProject,
    ImportPool,
    OpenImagePictureGraphics(ObjectId),
}

pub struct DesignerApp {
    // Path to settings file
    settings_path: String,
    project: Option<EditorProject>,
    file_dialog_reason: Option<FileDialogReason>,
    file_channel: (Sender<Vec<u8>>, Receiver<Vec<u8>>),
    show_development_popup: bool,
    new_object_dialog: Option<(ObjectType, String)>,
    apply_smart_naming_on_import: bool,
    main_focus_id: Option<ObjectId>,
    main_focus_bg: egui::Color32,

    // Internal clipboard for copy/paste
    clipboard: Option<Vec<Object>>,

    // Settings modal fields
    show_settings_modal: bool,
    softkey_key_width: u16,
    softkey_key_height: u16,
    key_width: u16,
    key_height: u16,
    temp_softkey_key_width: u16,
    temp_softkey_key_height: u16,
    temp_key_width: u16,
    temp_key_height: u16,
    temp_softkey_orientation: SoftKeyMaskOrientation,
    temp_softkey_order: SoftKeyOrder,
    softkey_mask_orientation: SoftKeyMaskOrientation,
    softkey_mask_key_order: SoftKeyOrder,

    // VT version setting
    vt_version: ag_iso_stack::object_pool::vt_version::VtVersion,
    temp_vt_version: ag_iso_stack::object_pool::vt_version::VtVersion,

    // Import selection state
    import_tree_selection: Option<ImportTreeSelection>,
    // Multi-selection for object list
    object_list_multi_selected: std::collections::HashSet<ObjectId>,

    // Autosave feature
    last_autosave: std::time::Instant,
    autosave_interval_secs: u64,
}

struct ImportTreeSelection {
    pool: ObjectPool,
    selected: std::collections::HashSet<ObjectId>,
    show_modal: bool,
}

impl DesignerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let settings_path = "designer_settings.json".to_string();
        let mut settings = None;
        match std::fs::read_to_string(&settings_path) {
            Ok(data) => match serde_json::from_str::<DesignerSettings>(&data) {
                Ok(loaded) => {
                    // Insert loaded settings into egui context temp data for global access
                    cc.egui_ctx.data_mut(|data| {
                        data.insert_temp(egui::Id::new("designer_settings"), loaded.clone());
                    });
                    settings = Some(loaded);
                }
                Err(e) => {
                    eprintln!(
                        "[ERROR] Failed to parse settings file: {}\nData: {}",
                        e, data
                    );
                }
            },
            Err(e) => {
                eprintln!("[ERROR] Failed to read settings file: {}", e);
            }
        }
        let mut fonts = egui::FontDefinitions::default();

        // Configure monospace font for NonProportional block fonts
        // The monospace font will be used for rendering NonProportional text blocks
        // which are constrained to fit within their defined box dimensions (e.g., 24×32px per character)
        fonts
            .families
            .get_mut(&egui::FontFamily::Monospace)
            .unwrap()
            .clear();
        // Use system monospace fonts as fallback
        fonts
            .families
            .get_mut(&egui::FontFamily::Monospace)
            .unwrap()
            .extend_from_slice(&[
                "Courier New".to_owned(),
                "Consolas".to_owned(),
                "DejaVu Sans Mono".to_owned(),
                "Liberation Mono".to_owned(),
            ]);

        // TODO: Future enhancement - load custom font files for ISO 8859 character sets
        //// Install ISO 8859-1 (ISO Latin 1) font
        // fonts.font_data.insert(
        //     "iso_latin_1".to_owned(),
        //     egui::FontData::from_static(include_bytes!("assets/fonts/iso-latin1.ttf")),
        // );
        // fonts
        //     .families
        //     .get_mut(&egui::FontFamily::Name("ISO Latin 1".into()))
        //     .unwrap()
        //     .insert(0, "iso_latin_1".to_owned());

        // // Install ISO 8859-15 (ISO Latin 9) font
        // fonts.font_data.insert(
        //     "iso_latin_9".to_owned(),
        //     egui::FontData::from_static(include_bytes!("assets/fonts/iso-latin9.ttf")),
        // );
        // fonts
        //     .families
        //     .get_mut(&egui::FontFamily::Name("ISO Latin 9".into()))
        //     .unwrap()
        //     .insert(0, "iso_latin_9".to_owned());

        // // Install ISO 8859-2 (ISO Latin 2) font
        // fonts.font_data.insert(
        //     "iso_latin_2".to_owned(),
        //     egui::FontData::from_static(include_bytes!("assets/fonts/iso-latin2.ttf")),
        // );
        // fonts
        //     .families
        //     .get_mut(&egui::FontFamily::Name("ISO Latin 2".into()))
        //     .unwrap()
        //     .insert(0, "iso_latin_2".to_owned());

        // // Install ISO 8859-4 (ISO Latin 4) font
        // fonts.font_data.insert(
        //     "iso_latin_4".to_owned(),
        //     egui::FontData::from_static(include_bytes!("assets/fonts/iso-latin4.ttf")),
        // );
        // fonts
        //     .families
        //     .get_mut(&egui::FontFamily::Name("ISO Latin 4".into()))
        //     .unwrap()
        //     .insert(0, "iso_latin_4".to_owned());

        // // Install ISO 8859-5 (Cyrillic) font
        // fonts.font_data.insert(
        //     "iso_cyrillic".to_owned(),
        //     egui::FontData::from_static(include_bytes!("assets/fonts/iso-cyrillic.ttf")),
        // );
        // fonts
        //     .families
        //     .get_mut(&egui::FontFamily::Name("ISO Cyrillic".into()))
        //     .unwrap()
        //     .insert(0, "iso_cyrillic".to_owned());

        // // Install ISO 8859-7 (Greek) font
        // fonts.font_data.insert(
        //     "iso_greek".to_owned(),
        //     egui::FontData::from_static(include_bytes!("assets/fonts/iso-greek.ttf")),
        // );
        // fonts
        //     .families
        //     .get_mut(&egui::FontFamily::Name("ISO Greek".into()))
        //     .unwrap()
        //     .insert(0, "iso_greek".to_owned());

        Self {
            project: None,
            file_dialog_reason: None,
            file_channel: std::sync::mpsc::channel(),
            show_development_popup: true,
            new_object_dialog: None,
            apply_smart_naming_on_import: true, // Default to true for better UX
            main_focus_id: None,
            main_focus_bg: egui::Color32::from_rgb(240, 240, 240),

            clipboard: None,

            // Settings modal defaults
            show_settings_modal: false,
            softkey_key_width: settings.as_ref().map_or(80, |s| s.softkey_key_width),
            softkey_key_height: settings.as_ref().map_or(80, |s| s.softkey_key_height),
            key_width: 80,
            key_height: 80,
            temp_softkey_key_width: settings.as_ref().map_or(80, |s| s.softkey_key_width),
            temp_softkey_key_height: settings.as_ref().map_or(80, |s| s.softkey_key_height),
            temp_key_width: 80,
            temp_key_height: 80,
            temp_softkey_orientation: settings
                .as_ref()
                .map_or(SoftKeyMaskOrientation::RightRight, |s| {
                    s.softkey_mask_orientation.clone()
                }),
            temp_softkey_order: settings
                .as_ref()
                .map_or(SoftKeyOrder::TopRight, |s| s.softkey_mask_key_order.clone()),
            softkey_mask_orientation: settings
                .as_ref()
                .map_or(SoftKeyMaskOrientation::RightRight, |s| {
                    s.softkey_mask_orientation.clone()
                }),
            softkey_mask_key_order: settings
                .as_ref()
                .map_or(SoftKeyOrder::TopRight, |s| s.softkey_mask_key_order.clone()),

            // Import selection state
            import_tree_selection: None,

            // VT version
            vt_version: settings.as_ref().map_or(
                ag_iso_stack::object_pool::vt_version::VtVersion::Version3,
                |s| s.vt_version,
            ),
            temp_vt_version: settings.as_ref().map_or(
                ag_iso_stack::object_pool::vt_version::VtVersion::Version3,
                |s| s.vt_version,
            ),
            settings_path,
            object_list_multi_selected: std::collections::HashSet::new(),
            last_autosave: std::time::Instant::now(),
            autosave_interval_secs: 30, // Autosave every 30 seconds
        }
    }
}

impl DesignerApp {
    /// Open a file dialog
    fn open_file_dialog(&mut self, reason: FileDialogReason, ctx: &egui::Context) {
        let is_image_loading = matches!(reason, FileDialogReason::OpenImagePictureGraphics(_));
        self.file_dialog_reason = Some(reason);

        let sender = self.file_channel.0.clone();
        let mut dialog = rfd::AsyncFileDialog::new();

        // Add image file filters for image loading
        if is_image_loading {
            dialog = dialog.add_filter(
                "Image Files",
                &[
                    "png", "jpg", "jpeg", "bmp", "gif", "ico", "tiff", "tif", "webp",
                ],
            );
        }

        let task = dialog.pick_file();
        let ctx = ctx.clone();
        execute(async move {
            let file = task.await;
            if let Some(file) = file {
                let content = file.read().await;
                let _ = sender.send(content);
            }
            ctx.request_repaint();
        });
    }

    /// Handle a file loaded in the file dialog
    fn handle_file_loaded(&mut self) {
        if let Ok(content) = self.file_channel.1.try_recv() {
            match self.file_dialog_reason.take() {
                Some(FileDialogReason::LoadPool) => {
                    let project = EditorProject::from(ObjectPool::from_iop(content));
                    // Apply smart naming to all objects that don't have custom names (if enabled)
                    // Only set default alignment if missing (should not overwrite loaded value)
                    let pool = project.get_mut_pool();
                    if self.apply_smart_naming_on_import {
                        project.apply_smart_naming_to_all_objects();
                    }
                    self.project = Some(project);
                }
                Some(FileDialogReason::ImportPool) => {
                    if let Some(_pool) = &mut self.project {
                        let imported_pool = ObjectPool::from_iop(content);
                        self.import_tree_selection = Some(ImportTreeSelection {
                            pool: imported_pool,
                            selected: std::collections::HashSet::new(),
                            show_modal: true,
                        });
                    }
                }
                Some(FileDialogReason::LoadProject) => {
                    match EditorProject::load_project(content) {
                        Ok(project) => {
                            self.project = Some(project);
                        }
                        Err(e) => {
                            log::error!("Failed to load project: {}", e);
                            // TODO: Show error dialog
                        }
                    }
                }
                Some(FileDialogReason::OpenImagePictureGraphics(id)) => {
                    if let Some(pool) = &mut self.project {
                        if let Some(obj) = pool.get_mut_pool().borrow_mut().object_mut_by_id(id) {
                            match obj {
                                Object::PictureGraphic(o) => {
                                    if let Ok(img) = image::load_from_memory(&content) {
                                        // Update dimensions based on the new picture
                                        let w = img.width();
                                        let h = img.height();

                                        if w > u16::MAX as u32 || h > u16::MAX as u32 {
                                            log::error!(
                                                "Image dimensions exceed maximum size of {}x{}",
                                                u16::MAX,
                                                u16::MAX
                                            );
                                            return;
                                        }

                                        o.actual_width = w as u16;
                                        o.actual_height = h as u16;
                                        if o.width == 0 {
                                            o.width = o.actual_width;
                                        }

                                        // Set format by default to 8-bit color, user can change it in UI
                                        o.format = PictureGraphicFormat::EightBit;

                                        // We set transparent color to 1 (arbitrary choice) as we
                                        // only use index 15..255 for actual colors
                                        o.transparency_colour = 1;
                                        o.options.transparent = true;

                                        let rgba = if let Some(view) = img.as_rgba8() {
                                            // Borrowed view (no allocation)
                                            std::borrow::Cow::Borrowed(view)
                                        } else {
                                            // Allocates once if the image isn't already RGBA8
                                            std::borrow::Cow::Owned(img.to_rgba8())
                                        };

                                        // Build raw and run-length encoded data
                                        let pixel_count = (w as usize) * (h as usize);

                                        // Worst case: raw = N, rle = 2*N
                                        let mut raw = Vec::with_capacity(pixel_count);
                                        let mut rle = Vec::with_capacity(pixel_count * 2);

                                        let mut have_run = false;
                                        let mut run_value: u8 = 0;
                                        let mut run_count: u8 = 0;

                                        for p in rgba.pixels() {
                                            let idx = if p[3] == 0 {
                                                o.transparency_colour
                                            } else {
                                                find_closest_color_index(p[0], p[1], p[2])
                                            };

                                            raw.push(idx);

                                            if !have_run {
                                                have_run = true;
                                                run_value = idx;
                                                run_count = 1;
                                                continue;
                                            }

                                            if idx == run_value && run_count < u8::MAX {
                                                run_count += 1;
                                            } else {
                                                rle.push(run_count);
                                                rle.push(run_value);
                                                run_value = idx;
                                                run_count = 1;
                                            }
                                        }

                                        // flush final run
                                        if have_run {
                                            rle.push(run_count);
                                            rle.push(run_value);
                                        }

                                        // Choose the best encoding
                                        if rle.len() < raw.len() {
                                            o.data = rle;
                                            o.options.data_code_type = DataCodeType::RunLength;
                                            log::info!(
                                            "Selected run-length encoding ({} bytes) over raw ({} bytes)",
                                            o.data.len(),
                                            raw.len()
                                        );
                                        } else {
                                            o.data = raw;
                                            o.options.data_code_type = DataCodeType::Raw;
                                            log::info!(
                                            "Selected raw encoding ({} bytes) over run-length ({} bytes)",
                                            o.data.len(),
                                            rle.len()
                                        );
                                        }
                                    } else {
                                        log::error!("Failed to decode image");
                                    }
                                }
                                _ => (),
                            }
                        }
                    }
                }
                _ => (),
            }
        }
    }

    /// Open a file dialog to save a pool file
    fn save_pool(&mut self) {
        if let Some(pool) = &self.project {
            let task = rfd::AsyncFileDialog::new()
                .set_file_name("object_pool.iop")
                .save_file();
            let contents = pool.get_pool().as_iop();

            execute(async move {
                let file = task.await;
                if let Some(file) = file {
                    _ = file.write(&contents).await;
                }
            });
        }
    }

    /// Open a file dialog to save a project file
    fn save_project(&mut self) {
        if let Some(project) = &self.project {
            match project.save_project() {
                Ok(contents) => {
                    let task = rfd::AsyncFileDialog::new()
                        .set_file_name("project.aitp")
                        .add_filter("AgIsoTerminal Project", &["aitp"])
                        .save_file();
                    execute(async move {
                        let file = task.await;
                        if let Some(file) = file {
                            _ = file.write(&contents).await;
                        }
                    });
                }
                Err(e) => {
                    log::error!("Failed to save project: {}", e);
                    // TODO: Show error dialog
                }
            }
        }
    }

    /// Convert a string to a valid C identifier
    fn to_c_identifier(name: &str) -> String {
        name.chars()
            .map(|c| match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' => c.to_ascii_uppercase(),
                _ => '_',
            })
            .collect()
    }

    /// Open a file dialog to save a C header file with object IDs
    fn save_header(&mut self) {
        if let Some(project) = &self.project {
            let pool = project.get_pool();

            // Start with the header
            let mut header = String::from("// Object IDs for the objects in the object pool.\n\n");
            header.push_str("#pragma once\n");
            header.push_str("#define UNDEFINED 65535\n");

            // Collect all objects with their names and IDs
            let mut objects: Vec<(String, u16)> = pool
                .objects()
                .iter()
                .map(|obj| {
                    let name = project.get_object_info(obj).get_name(obj);
                    let c_name = Self::to_c_identifier(&name);
                    let id = u16::from(obj.id());
                    (c_name, id)
                })
                .collect();

            // Sort by ID for consistent output
            objects.sort_by_key(|&(_, id)| id);

            // Add defines for each object
            for (name, id) in objects {
                header.push_str(&format!("#define {} {}\n", name, id));
            }

            let contents = header.into_bytes();
            let task = rfd::AsyncFileDialog::new()
                .set_file_name("object_pool.h")
                .add_filter("C Header", &["h"])
                .save_file();
            execute(async move {
                let file = task.await;
                if let Some(file) = file {
                    _ = file.write(&contents).await;
                }
            });
        }
    }
}

fn render_selectable_object(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    object: &Object,
    project: &EditorProject,
    set_main_focus: &mut dyn FnMut(ObjectId),
    multi_selected: &mut std::collections::HashSet<ObjectId>,
) -> Option<CopyPasteAction> {
    let this_ui_id = ui.id();
    let object_info = project.get_object_info(object);

    let renaming_object = project.get_renaming_object();
    if renaming_object
        .clone()
        .is_some_and(|(ui_id, id, _)| id == object.id() && ui_id == this_ui_id)
    {
        let mut name = renaming_object.unwrap().2;
        let response = ui.text_edit_singleline(&mut name);
        project.set_renaming_object(this_ui_id, object.id(), name); // Update the name in the project
        let cancelled = ui.input(|i| i.key_pressed(egui::Key::Escape));
        if response.lost_focus() {
            project.finish_renaming_object(!cancelled);
            return None;
        } else if !response.has_focus() {
            // We need to focus the text edit when we start renaming
            response.request_focus();
            return None;
        }
        return None;
    } else {
        let is_selected = multi_selected.contains(&object.id());
        let label_text = format!(
            "{}: {}",
            u16::from(object.id()),
            object_info.get_name(object)
        );
        let response = ui.selectable_label(is_selected, label_text);

        if response.clicked() {
            let ctrl = ctx.input(|i| i.modifiers.ctrl);
            let shift = ctx.input(|i| i.modifiers.shift);
            if ctrl {
                // Ctrl-click: toggle selection
                if multi_selected.contains(&object.id()) {
                    multi_selected.remove(&object.id());
                } else {
                    multi_selected.insert(object.id());
                }
                // Update last selected in egui context
                ctx.data_mut(|d| {
                    d.insert_temp(
                        egui::Id::new("last_object_list_selected"),
                        Some(object.id()),
                    )
                });
            } else if shift {
                // Shift-click: select range from last selected to this
                if let Some(object_list) =
                    ctx.data(|d| d.get_temp::<Vec<ObjectId>>(egui::Id::new("object_list_order")))
                {
                    let last = ctx
                        .data(|d| {
                            d.get_temp::<Option<ObjectId>>(egui::Id::new(
                                "last_object_list_selected",
                            ))
                        })
                        .flatten();
                    let anchor = last.unwrap_or(object.id());
                    let mut found_anchor = false;
                    let mut found_this = false;
                    let mut range = vec![];
                    for oid in object_list {
                        if oid == anchor || oid == object.id() {
                            if !found_anchor {
                                found_anchor = true;
                            } else {
                                found_this = true;
                                range.push(oid);
                                break;
                            }
                        }
                        if found_anchor {
                            range.push(oid);
                        }
                    }
                    if found_anchor && found_this {
                        for oid in range {
                            multi_selected.insert(oid);
                        }
                    } else {
                        multi_selected.insert(object.id());
                    }
                } else {
                    multi_selected.insert(object.id());
                }
                // Do NOT update last_object_list_selected on shift-click
            } else {
                // Normal click: single select
                multi_selected.clear();
                multi_selected.insert(object.id());
                // Update last selected in egui context
                ctx.data_mut(|d| {
                    d.insert_temp(
                        egui::Id::new("last_object_list_selected"),
                        Some(object.id()),
                    )
                });
            }
            project
                .get_mut_selected()
                .replace(NullableObjectId(Some(object.id())));
        }
        if response.double_clicked() {
            project.set_renaming_object(this_ui_id, object.id(), object_info.get_name(object));
        }

        let mut action = None;
        response.context_menu(|ui| {
            if ui
                .button("Copy (Exact)")
                .on_hover_text("Copy this object (Ctrl+C)")
                .clicked()
            {
                action = Some(CopyPasteAction::CopyExact);
                ui.close();
            }
            if ui
                .button("Copy as New")
                .on_hover_text("Copy as new object (Ctrl+Shift+C)")
                .clicked()
            {
                action = Some(CopyPasteAction::CopyAsNew);
                ui.close();
            }
            if ui
                .button("Paste")
                .on_hover_text("Paste object(s) (Ctrl+V)")
                .clicked()
            {
                action = Some(CopyPasteAction::Paste);
                ui.close();
            }
            if ui.button("Rename").on_hover_text("Rename object").clicked() {
                project.set_renaming_object(this_ui_id, object.id(), object_info.get_name(object));
                ui.close();
            }
            if multi_selected.is_empty() {
                if ui.button("Delete").on_hover_text("Delete object").clicked() {
                    project.get_mut_pool().borrow_mut().remove(object.id());
                    ui.close();
                }
            } else {
                let delete_text = if multi_selected.len() == 1 {
                    "Delete".to_string()
                } else {
                    format!("Delete {} selected", multi_selected.len())
                };
                if ui
                    .button(&delete_text)
                    .on_hover_text("Delete all selected objects")
                    .clicked()
                {
                    let ids_to_delete: Vec<_> = multi_selected.iter().copied().collect();
                    for id in ids_to_delete {
                        project.get_mut_pool().borrow_mut().remove(id);
                    }
                    multi_selected.clear();
                    ui.close();
                }
            }
            if ui
                .button("Set as main focus")
                .on_hover_text("Show this object as the main focus in the main view")
                .clicked()
            {
                set_main_focus(object.id());
                ui.close();
            }
        });
        action
    }
}
fn render_object_hierarchy(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    parent_id: egui::Id,
    object: &Object,
    project: &EditorProject,
    set_main_focus: &mut dyn FnMut(ObjectId),
    actions: &mut Vec<CopyPasteAction>,
) {
    let refs = object.referenced_objects();
    let mut dummy_multi_selected = std::collections::HashSet::new();
    if refs.is_empty() {
        if let Some(action) = render_selectable_object(
            ctx,
            ui,
            object,
            project,
            set_main_focus,
            &mut dummy_multi_selected,
        ) {
            actions.push(action);
        }
        //ui.add_space(ui.spacing().indent);
    } else {
        let id = parent_id.with(project.get_object_info(object).get_unique_id());
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
            .show_header(ui, |ui| {
                if let Some(action) = render_selectable_object(
                    ctx,
                    ui,
                    object,
                    project,
                    set_main_focus,
                    &mut dummy_multi_selected,
                ) {
                    actions.push(action);
                }
            })
            .body(|ui| {
                for (idx, obj_id) in refs.iter().enumerate() {
                    match project.get_pool().object_by_id(*obj_id) {
                        Some(obj) => {
                            render_object_hierarchy(
                                ctx,
                                ui,
                                id.with(idx),
                                obj,
                                project,
                                set_main_focus,
                                actions,
                            );
                        }
                        None => {
                            ui.colored_label(
                                egui::Color32::RED,
                                format!("Missing object: {:?}", id),
                            );
                        }
                    }
                }
            });
    }
}

fn update_object_hierarchy_headers(
    ctx: &egui::Context,
    parent_id: egui::Id,
    object: &Object,
    pool: &ObjectPool,
    new_selected: NullableObjectId,
) -> bool {
    let mut is_selected_or_descendant = new_selected == object.id().into();

    let refs = object.referenced_objects();
    if !refs.is_empty() {
        let id = parent_id.with(object.id().value());

        // Update in a depth-first manner
        for obj_id in refs {
            if let Some(obj) = pool.object_by_id(obj_id) {
                is_selected_or_descendant |=
                    update_object_hierarchy_headers(ctx, id, obj, pool, new_selected);
            }
        }

        if is_selected_or_descendant {
            if let Some(mut state) = egui::collapsing_header::CollapsingState::load(ctx, id) {
                if !state.is_open() {
                    state.set_open(true);
                    state.store(ctx);
                }
            }
        }
    }

    is_selected_or_descendant
}

impl eframe::App for DesignerApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        // AUTOSAVE FEATURE
        if let Some(project) = &self.project {
            let now = std::time::Instant::now();
            if now.duration_since(self.last_autosave).as_secs() >= self.autosave_interval_secs {
                if let Ok(contents) = project.save_project() {
                    let _ = std::fs::write("autosave.aitp", &contents);
                    self.last_autosave = now;
                }
            }
        }
        ctx.style_mut(|style| {
            style.interaction.selectable_labels = false;
        });

        // Handle file dialog
        self.handle_file_loaded();

        // Check for image load requests
        if let Some(pool) = &self.project {
            if let Some(object_id) = pool.take_image_load_request() {
                self.open_file_dialog(FileDialogReason::OpenImagePictureGraphics(object_id), ctx);
            }
        }

        if self.show_development_popup {
            egui::Window::new("🚧 Under Active Development")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.add_space(10.0);
                    ui.label("This application is still under active development. Some features may be missing or broken. We appreciate your patience and feedback!");

                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label("If you encounter issues, please report them at:");
                        ui.hyperlink("https://github.com/Open-Agriculture/AgIsoTerminalDesigner/issues");
                    });

                    ui.add_space(20.0);
                    ui.horizontal(|ui| {
                        ui.add_space(ui.available_width() - 60.0);
                        if ui.button("OK").clicked() {
                            self.show_development_popup = false;
                        }
                    });
                });
            return;
        }

        // Show new object name dialog
        if let Some((object_type, mut name)) = self.new_object_dialog.clone() {
            let mut should_create = false;
            let mut should_cancel = false;

            egui::Window::new(format!("New {:?}", object_type))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Enter a name for the new object:");
                    ui.add_space(10.0);

                    let response = ui.text_edit_singleline(&mut name);

                    // Auto-focus the text field
                    if !response.has_focus() && !response.lost_focus() {
                        response.request_focus();
                    }

                    // Check for Enter key
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        should_create = true;
                    }

                    // Check for Escape key
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        should_cancel = true;
                    }

                    ui.add_space(20.0);
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() || should_create {
                            should_create = true;
                        }
                        if ui.button("Cancel").clicked() || should_cancel {
                            should_cancel = true;
                        }
                    });
                });

            if should_create {
                // Create the object with the given name
                if let Some(pool) = &mut self.project {
                    let mut new_obj = ag_iso_terminal_designer::default_object(
                        object_type,
                        Some(pool.get_pool()),
                    );

                    let id = pool.allocate_object_id_for_type(object_type);
                    new_obj.mut_id().set_value(id.value()).ok();

                    // Add object to pool
                    pool.get_mut_pool().borrow_mut().add(new_obj.clone());

                    // Auto-sort objects by name after adding (avoid borrow checker issues)
                    // Auto-sort objects by ID after adding
                    pool.sort_objects_by(|a, b| u16::from(a.id()).cmp(&u16::from(b.id())));

                    // Set the custom name
                    let mut object_info = pool.object_info.borrow_mut();
                    let info = object_info
                        .entry(new_obj.id())
                        .or_insert_with(|| ag_iso_terminal_designer::ObjectInfo::new(&new_obj));
                    info.set_name(name);
                    drop(object_info);

                    // Select the new object
                    pool.get_mut_selected()
                        .replace(NullableObjectId::new(id.value()));
                }
                self.new_object_dialog = None;
            } else if should_cancel {
                self.new_object_dialog = None;
            } else {
                // Update the name in the dialog state
                self.new_object_dialog = Some((object_type, name));
            }
        }

        egui::TopBottomPanel::top("topbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if self.main_focus_id.is_some() {
                    if ui
                        .button("Exit Focus Mode")
                        .on_hover_text("Return to normal view")
                        .clicked()
                    {
                        self.main_focus_id = None;
                    }
                    ui.separator();
                }
                egui::widgets::global_theme_preference_buttons(ui);
                ui.separator();

                // Undo/redo buttons
                if let Some(pool) = &mut self.project {
                    let undo_shortcut =
                        egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Z);
                    let redo_shortcut =
                        egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Y);

                    // Copy/Paste shortcuts
                    let undo_shortcut =
                        egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Z);
                    let redo_shortcut =
                        egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Y);

                    // Copy/Paste shortcuts
                    let copy_shortcut =
                        egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::C);
                    let copy_as_new_shortcut = egui::KeyboardShortcut::new(
                        egui::Modifiers {
                            ctrl: true,
                            shift: true,
                            ..Default::default()
                        },
                        egui::Key::C,
                    );
                    let paste_shortcut =
                        egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::V);

                    if ui
                        .add_enabled(
                            pool.undo_available(),
                            egui::widgets::Button::new("\u{2BAA}"),
                        )
                        .on_hover_text(format!("Undo ({})", ctx.format_shortcut(&undo_shortcut)))
                        .clicked()
                        || ctx.input_mut(|i| i.consume_shortcut(&undo_shortcut))
                    {
                        pool.undo();
                    }
                    if ui
                        .add_enabled(
                            pool.redo_available(),
                            egui::widgets::Button::new("\u{2BAB}"),
                        )
                        .on_hover_text(format!("Redo ({})", ctx.format_shortcut(&redo_shortcut)))
                        .clicked()
                        || ctx.input_mut(|i| i.consume_shortcut(&redo_shortcut))
                    {
                        pool.redo();
                    }
                    ui.separator();

                    // Copy/Paste UI
                    let can_copy = pool.get_selected().0.is_some();
                    let can_paste = self.clipboard.is_some();

                    if ui
                        .add_enabled(can_copy, egui::Button::new("Copy (Exact)"))
                        .on_hover_text(format!("Copy selected object (Ctrl+C)"))
                        .clicked()
                        || ctx.input_mut(|i| i.consume_shortcut(&copy_shortcut))
                    {
                        let objs = pool.copy_selected_objects_exact();
                        if !objs.is_empty() {
                            self.clipboard = Some(objs);
                        }
                    }
                    if ui
                        .add_enabled(can_copy, egui::Button::new("Copy as New"))
                        .on_hover_text(format!("Copy as new object (Ctrl+Shift+C)"))
                        .clicked()
                        || ctx.input_mut(|i| i.consume_shortcut(&copy_as_new_shortcut))
                    {
                        let objs = pool.copy_selected_objects_as_new();
                        if !objs.is_empty() {
                            self.clipboard = Some(objs);
                        }
                    }
                    if ui
                        .add_enabled(can_paste, egui::Button::new("Paste"))
                        .on_hover_text(format!("Paste object(s) (Ctrl+V)"))
                        .clicked()
                        || ctx.input_mut(|i| i.consume_shortcut(&paste_shortcut))
                    {
                        if let Some(objs) = self.clipboard.clone() {
                            pool.paste_objects(objs);
                        }
                    }
                    ui.separator();
                }

                // Debug: Log largest object in pool
                if let Some(project) = &self.project {
                    if ui
                        .button("Log Top 5 Largest Objects")
                        .on_hover_text("Log the top 5 largest objects in the pool to the debug log")
                        .clicked()
                    {
                        log_top_largest_objects(project.get_pool(), 20);
                    }
                }

                ui.menu_button("File", |ui| {
                    ui.label("Project Files");
                    if ui.button("Open Project (.aitp)").clicked() {
                        self.open_file_dialog(FileDialogReason::LoadProject, ctx);
                        ui.close();
                    }
                    if self.project.is_some() && ui.button("Save Project (.aitp)").clicked() {
                        self.save_project();
                        ui.close();
                    }

                    ui.separator();
                    ui.label("ISOBUS Files");

                    if ui.button("Import IOP (.iop)").clicked() {
                        self.open_file_dialog(FileDialogReason::LoadPool, ctx);
                        ui.close();
                    }
                    if ui.button("Import IOP into current File (.iop)").clicked() {
                        self.open_file_dialog(FileDialogReason::ImportPool, ctx);
                        ui.close();
                    }

                    ui.checkbox(
                        &mut self.apply_smart_naming_on_import,
                        "Apply smart naming on import",
                    )
                    .on_hover_text(
                        "Automatically apply smart naming to objects when importing IOP files",
                    );
                    if self.project.is_some() && ui.button("Export IOP (.iop)").clicked() {
                        self.save_pool();
                        ui.close();
                    }
                    if self.project.is_some() && ui.button("Export Header (.h)").clicked() {
                        self.save_header();
                        ui.close();
                    }
                });

                if self.project.is_some() {
                    // Add a new object
                    ui.menu_button("Add object", |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for object_type in ObjectType::values() {
                                if ui.button(format!("{:?}", object_type)).clicked() {
                                    if object_type == ObjectType::PictureGraphic {
                                        // Trigger the file dialog for PictureGraphic
                                        self.open_file_dialog(
                                            FileDialogReason::OpenImagePictureGraphics(
                                                ObjectId::new(255).unwrap(), // Temporary ID, will be set later
                                            ),
                                            ctx,
                                        );
                                    }
                                    // Generate smart default name
                                    let pool = self.project.as_ref().unwrap();
                                    let default_name =
                                        pool.generate_smart_name_for_new_object(object_type);
                                    self.new_object_dialog = Some((object_type, default_name));
                                    ui.close();
                                }
                            }
                        });
                    });
                }
                if ui
                    .button("Settings")
                    .on_hover_text("Configure SoftKeyMask & Key sizes")
                    .clicked()
                {
                    self.show_settings_modal = true;
                }
                if let Some(pool) = &mut self.project {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(
                            egui::Slider::new(&mut pool.mask_size, 100..=2000)
                                .text("Virtual Mask size"),
                        );
                    });
                }
                //if let Some(pool) = &mut self.project {
                //    // Square mask size slider with snap points
                //    let snap_points = [320, 480, 800, 1024];
                //    let snap_threshold = 8;
                //    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                //        ui.label("Virtual Mask size (square):");
                //        let response = ui.add(
                //            egui::Slider::new(&mut pool.mask_size, 200..=1200)
                //                .step_by(1.0)
                //                .text("px"),
                //        );
                //        // Only snap when dragging the slider, not when typing
                //
                //        if response.dragged() {
                //            for &snap in &snap_points {
                //                if ((pool.mask_size as i32) - (snap as i32)).abs() <= snap_threshold
                //                {
                //                    pool.mask_size = snap;
                //                    break;
                //                }
                //            }
                //        }
                //    });
                //}
            });
        });

        if let Some(pool) = &mut self.project {
            // Set forward and backward navigation shortcuts to mouse buttons
            if ctx.input(|i| i.pointer.button_released(egui::PointerButton::Extra1)) {
                pool.set_previous_selected();
            } else if ctx.input(|i| i.pointer.button_released(egui::PointerButton::Extra2)) {
                pool.set_next_selected();
            }

            // Clone a reference to self.object_list_multi_selected for use in the closure
            let object_list_multi_selected_ptr = &mut self.object_list_multi_selected;
            let clipboard_ptr = &mut self.clipboard;

            // Limit the mutable borrow of self.project to this block only
            egui::SidePanel::left("left_panel").show(ctx, |ui| {
                let mut actions = Vec::new();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                    if let Some(working_set) = pool.get_pool().working_set_object() {
                        let mut set_main_focus = |object_id: ObjectId| {
                            self.main_focus_id = Some(object_id);
                        };
                        render_object_hierarchy(
                            ctx,
                            ui,
                            egui::Id::new(OBJECT_HIERARCHY_ID),
                            &Object::WorkingSet(working_set.clone()),
                            pool,
                            &mut set_main_focus,
                            &mut actions,
                        );
                    }
                    // If you have auxiliary_objects, handle them here
                    let auxiliary_objects = pool.get_pool().objects_by_types(&[
                        ObjectType::AuxiliaryFunctionType1,
                        ObjectType::AuxiliaryInputType1,
                        ObjectType::AuxiliaryFunctionType2,
                        ObjectType::AuxiliaryInputType2,
                    ]);
                    if !auxiliary_objects.is_empty() {
                        ui.separator();
                        let mut set_main_focus = |_id| {};
                        let ctx_ref = ui.ctx().clone();
                        for object in auxiliary_objects {
                            if let Some(action) = render_selectable_object(
                                &ctx_ref,
                                ui,
                                object,
                                pool,
                                &mut set_main_focus,
                                object_list_multi_selected_ptr,
                            ) {
                                actions.push(action);
                            }
                        }
                    }
                    ui.separator();
                    // After UI pass, process actions with mutable borrow
                    for action in actions {
                        match action {
                            CopyPasteAction::CopyExact => {
                                let objs = pool.copy_selected_objects_exact();
                                if !objs.is_empty() {
                                    *clipboard_ptr = Some(objs);
                                }
                            }
                            CopyPasteAction::CopyAsNew => {
                                let objs = pool.copy_selected_objects_as_new();
                                if !objs.is_empty() {
                                    *clipboard_ptr = Some(objs);
                                }
                            }
                            CopyPasteAction::Paste => {
                                if let Some(objs) = clipboard_ptr.clone() {
                                    pool.paste_objects(objs);
                                }
                            }
                        }
                    }
                    // Filter objects in the pool by name
                    let filter_id = ui.id().with("filter_text");
                    let mut filter_text = ui
                        .data(|data| data.get_temp::<String>(filter_id))
                        .unwrap_or_default();

                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(ui.spacing().scroll.bar_width);
                            ui.menu_button("\u{2195}", |ui| {
                                if ui.button("Sort by name").clicked() {
                                    let pool_copy = pool.clone();
                                    pool.sort_objects_by(|a, b| {
                                        pool_copy
                                            .get_object_info(a)
                                            .get_name(a)
                                            .cmp(&pool_copy.get_object_info(b).get_name(b))
                                    });
                                    ui.close();
                                }
                                if ui.button("Sort by id").clicked() {
                                    pool.sort_objects_by(|a, b| {
                                        u16::from(a.id()).cmp(&u16::from(b.id()))
                                    });
                                    ui.close();
                                }
                            })
                            .response
                            .on_hover_text("Sort objects");

                            let filter_shortcut =
                                egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::F);

                            let response = ui
                                .add(
                                    egui::TextEdit::singleline(&mut filter_text)
                                        .hint_text("Filter object by name...")
                                        .desired_width(ui.available_width()),
                                )
                                .on_hover_text(format!(
                                    "Search shortcut ({})",
                                    ctx.format_shortcut(&filter_shortcut)
                                ));
                            if response.changed() {
                                ui.data_mut(|data| {
                                    data.insert_temp(filter_id, filter_text.clone())
                                });
                            } else if ctx.input_mut(|i| i.consume_shortcut(&filter_shortcut)) {
                                response.request_focus();
                            }
                        });
                    });

                    let filter_text = filter_text.to_lowercase();
                    let mut set_main_focus = |object_id: ObjectId| {
                        self.main_focus_id = Some(object_id);
                    };
                    let object_ids: Vec<ObjectId> =
                        pool.get_pool().objects().iter().map(|o| o.id()).collect();
                    ctx.data_mut(|d| {
                        d.insert_temp(egui::Id::new("object_list_order"), object_ids.clone())
                    });
                    for object in pool.get_pool().objects() {
                        if filter_text.is_empty()
                            || pool
                                .get_object_info(object)
                                .get_name(object)
                                .to_lowercase()
                                .contains(&filter_text)
                        {
                            render_selectable_object(
                                ctx,
                                ui,
                                object,
                                pool,
                                &mut set_main_focus,
                                object_list_multi_selected_ptr,
                            );
                        }
                    }

                    ui.allocate_space(ui.available_size());
                });
            });

            // Main panel with background color option for main focus
            egui::CentralPanel::default().show(ctx, |ui| {
                if let Some(main_id) = self.main_focus_id {
                    // Set background color first (with alpha)
                    let bg = self.main_focus_bg;
                    let rect = ui.max_rect();
                    ui.painter().rect_filled(rect, 0.0, bg);
                    ui.set_clip_rect(rect);
                    // Show color picker for main focus background (with alpha)
                    ui.horizontal(|ui| {
                        ui.label("Main focus background color (with transparency):");
                        let rgba = egui::Rgba::from(bg);
                        let mut arr = [rgba.r(), rgba.g(), rgba.b(), rgba.a()];
                        if ui.color_edit_button_rgba_unmultiplied(&mut arr).changed() {
                            self.main_focus_bg = egui::Color32::from_rgba_unmultiplied(
                                (arr[0] * 255.0) as u8,
                                (arr[1] * 255.0) as u8,
                                (arr[2] * 255.0) as u8,
                                (arr[3] * 255.0) as u8,
                            );
                        }
                    });
                    ui.separator();
                    // Show the main focus object if set
                    if let Some(obj) = pool.get_pool().object_by_id(main_id) {
                        let (width, height) = pool.get_pool().content_size(obj);
                        let desired_size = egui::Vec2::new(width as f32, height as f32);
                        egui::ScrollArea::both().show(ui, |ui| {
                            // Render DataMask and SoftKeyMask in the same coordinate space, side by side
                            let mut softkey_size = egui::Vec2::ZERO;
                            let mut softkey_obj = None;
                            if let Object::DataMask(dm) = obj {
                                if let Some(softkey_id) = dm.soft_key_mask.into() {
                                    if let Some(sk_obj) = pool.get_pool().object_by_id(softkey_id) {
                                        let (sk_width, sk_height) = pool.get_pool().content_size(sk_obj);
                                        softkey_size = egui::Vec2::new(sk_width as f32, sk_height as f32);
                                        softkey_obj = Some(sk_obj);
                                    }
                                }
                            }
                            let total_width = desired_size.x + softkey_size.x;
                            let max_height = desired_size.y.max(softkey_size.y);
                            ui.scope(|ui| {
                                ui.allocate_ui(egui::Vec2::new(total_width, max_height), |ui| {
                                    ui.horizontal(|ui| {
                                        // DataMask on the left
                                        // DataMask on the left, top-aligned
                                        let (id_dm, rect_dm) = ui.allocate_space(egui::Vec2::new(desired_size.x, max_height));
                                        ui.put(
                                            rect_dm,
                                            InteractiveMaskRenderer {
                                                object: obj,
                                                pool: pool.get_pool(),
                                                selected_callback: Box::new(|_| {}),
                                            },
                                        );
                                        // SoftKeyMask to the right, top-aligned
                                        if let Some(sk_obj) = softkey_obj {
                                            let (id_sk, rect_sk) = ui.allocate_space(egui::Vec2::new(softkey_size.x, max_height));
                                            if let Object::SoftKeyMask(sk) = sk_obj {
                                                let colour = pool.get_pool().color_by_index(sk.background_colour);
                                                let bg = egui::Color32::from_rgba_unmultiplied(
                                                    colour.r,
                                                    colour.g,
                                                    colour.b,
                                                    colour.a,
                                                );
                                                ui.painter().rect_filled(rect_sk, 0.0, bg);
                                            }
                                            // Calculate top offset for mask rendering
                                            let offset_y = 0.0; // top-aligned; use (max_height - softkey_size.y)/2.0 for centering
                                            let mask_rect = egui::Rect::from_min_size(
                                                rect_sk.min + egui::vec2(0.0, offset_y),
                                                egui::Vec2::new(softkey_size.x, softkey_size.y),
                                            );
                                            ui.put(
                                                mask_rect,
                                                InteractiveMaskRenderer {
                                                    object: sk_obj,
                                                    pool: pool.get_pool(),
                                                    selected_callback: Box::new(|_| {}),
                                                },
                                            );
                                        }
                                    });
                                });
                            });
                        });
                    } else {
                        ui.colored_label(
                            egui::Color32::RED,
                            format!("Main focus object not found: {}", u16::from(main_id)),
                        );
                    }
                } else {
                    // Default view when no main focus is set
                    if pool
                        .get_pool()
                        .objects_by_type(ObjectType::DataMask)
                        .is_empty()
                    {
                        ui.colored_label(
                            egui::Color32::RED,
                            "Missing data masks, please load a pool file or add a new mask...",
                        );
                    } else {
                        match pool.get_pool().working_set_object() {
                            Some(mask) => match pool.get_pool().object_by_id(mask.active_mask) {
                                Some(obj) => {
                                    let selected_ref = pool.get_mut_selected();
                                    egui::ScrollArea::both().show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            // DataMask panel
                                            ui.vertical(|ui| {

                                                ui.add_sized(
                                                    [pool.mask_size as f32, pool.mask_size as f32],
                                                    InteractiveMaskRenderer {
                                                        object: obj,
                                                        pool: pool.get_pool(),
                                                        selected_callback: Box::new(move |object_id| {
                                                            *selected_ref.borrow_mut() = NullableObjectId(Some(object_id));
                                                        }),
                                                    },
                                                );
                                            });
                                            // SoftKeyMask panel
                                            ui.separator();
                                            ui.vertical(|ui| {
                                                if let Object::DataMask(dm) = obj {
                                                    match dm.soft_key_mask.into() {
                                                        Some(softkey_id) => {
                                                            if let Some(sk_obj) = pool.get_pool().object_by_id(softkey_id) {
                                                                let (sk_width, sk_height) = pool.get_pool().content_size(sk_obj);
                                                                let sk_size = egui::Vec2::new(sk_width as f32, sk_height as f32);
                                                                ui.add_sized(
                                                                    [sk_size.x, sk_size.y],
                                                                    InteractiveMaskRenderer {
                                                                        object: sk_obj,
                                                                        pool: pool.get_pool(),
                                                                        selected_callback: Box::new(|_| {}),
                                                                    },
                                                                );
                                                            } else {
                                                                ui.colored_label(egui::Color32::RED, format!("SoftKeyMask assigned but not found in pool"));
                                                            }
                                                        }
                                                        None => {
                                                            ui.colored_label(egui::Color32::YELLOW, "No SoftKeyMask assigned to this DataMask");
                                                        }
                                                    }
                                                } else {
                                                    ui.colored_label(egui::Color32::YELLOW, "Main view is not a DataMask object");
                                                }
                                            });
                                        });
                                    });
                                }
                                None => {
                                    ui.colored_label(
                                        egui::Color32::RED,
                                        format!("Missing data mask: {:?}", mask),
                                    );
                                }
                            },
                            None => {
                                ui.colored_label(
                                    egui::Color32::RED,
                                    "No working sets, please add a new working set...",
                                );
                            }
                        }
                        let auxiliary_objects = pool.get_pool().objects_by_types(&[
                            ObjectType::AuxiliaryFunctionType1,
                            ObjectType::AuxiliaryInputType1,
                            ObjectType::AuxiliaryFunctionType2,
                            ObjectType::AuxiliaryInputType2,
                        ]);
                        if !auxiliary_objects.is_empty() {
                            ui.separator();
                            let mut set_main_focus = |_id| {};
                            let ctx_ref = ui.ctx().clone();
                            for object in auxiliary_objects {
                                render_selectable_object(
                                    &ctx_ref,
                                    ui,
                                    object,
                                    pool,
                                    &mut set_main_focus,
                                    &mut self.object_list_multi_selected,
                                );
                            }
                        }
                    }
                }
                ui.allocate_space(ui.available_size());
            });

            // Parameters panel
            egui::SidePanel::right("right_panel").show(ctx, |ui: &mut egui::Ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if let Some(id) = pool.get_selected().into() {
                        if let Some(obj) = pool.get_mut_pool().borrow_mut().object_mut_by_id(id) {
                            // Display editable object name as header
                            ui.horizontal(|ui| {
                                ui.label("Name:");

                                let object_info = pool.get_object_info(obj);
                                let mut name = object_info.get_name(obj);
                                let response = ui.text_edit_singleline(&mut name);

                                if response.changed() {
                                    let mut object_info_map = pool.object_info.borrow_mut();
                                    if let Some(info) = object_info_map.get_mut(&obj.id()) {
                                        info.set_name(name);
                                    }
                                }
                            });
                            ui.separator();

                            obj.render_parameters(
                                ui,
                                pool,
                                &DesignerSettings {
                                    softkey_key_width: self.softkey_key_width,
                                    softkey_key_height: self.softkey_key_height,
                                    softkey_mask_orientation: self.softkey_mask_orientation.clone(),
                                    softkey_mask_key_order: self.softkey_mask_key_order.clone(),
                                    vt_version: self.vt_version,
                                },
                            );
                            let (width, height) = pool.get_pool().content_size(obj);
                            ui.separator();
                            let desired_size = egui::Vec2::new(width as f32, height as f32);
                            ui.allocate_ui(desired_size, |ui| {
                                obj.render(ui, pool.get_pool(), Point::default());
                            });
                        } else {
                            ui.colored_label(
                                egui::Color32::RED,
                                format!("Selected object not found: {}", u16::from(id)),
                            );
                        }
                    }
                    ui.allocate_space(ui.available_size());
                });
            });
            if pool.update_pool() {
                ctx.request_repaint();
            }
            if pool.update_selected() {
                // Make sure all collapsing headers for the selected object are open
                if let Some(working_set) = pool.get_pool().working_set_object() {
                    update_object_hierarchy_headers(
                        ctx,
                        egui::Id::new(OBJECT_HIERARCHY_ID),
                        &Object::WorkingSet(working_set.clone()),
                        pool.get_pool(),
                        pool.get_selected(),
                    );
                }
            }

            // Show import tree selection modal if needed
            if let Some(import_tree) = &mut self.import_tree_selection {
                if import_tree.show_modal {
                    let mut close_modal = false;
                    let mut do_import = false;
                    egui::Window::new("Select objects to import (tree)")
                        .collapsible(false)
                        .resizable(true)
                        .show(ctx, |ui| {
                            ui.label("Select which objects (and their sub-objects) to import:");
                            ui.separator();
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                // Show tree for each root object (e.g. WorkingSet)
                                for obj in import_tree.pool.objects() {
                                    // Consider objects with no parent references as root objects
                                    let is_root = !import_tree.pool.objects().iter().any(|other| {
                                        other.referenced_objects().contains(&obj.id())
                                    });
                                    if is_root {
                                        show_import_tree_node(
                                            ui,
                                            obj,
                                            &import_tree.pool,
                                            &mut import_tree.selected,
                                        );
                                    }
                                }
                            });
                            ui.separator();
                            if ui.button("Import Selected").clicked() {
                                do_import = true;
                                close_modal = true;
                            }
                            if ui.button("Cancel").clicked() {
                                close_modal = true;
                            }
                        });
                    if do_import {
                        if let Some(pool) = &mut self.project {
                            use std::collections::{HashMap, HashSet, VecDeque};
                            // Collect all selected objects and their dependencies
                            let mut to_import = HashSet::new();
                            let mut queue: VecDeque<ObjectId> =
                                import_tree.selected.iter().cloned().collect();
                            while let Some(id) = queue.pop_front() {
                                if to_import.contains(&id) {
                                    continue;
                                }
                                to_import.insert(id);
                                if let Some(obj) = import_tree.pool.object_by_id(id) {
                                    for ref_id in obj.referenced_objects() {
                                        queue.push_back(ref_id);
                                    }
                                }
                            }
                            // Remove WorkingSet objects from to_import (allow selection for tree, but do not import)
                            to_import.retain(|&id| {
                                import_tree
                                    .pool
                                    .object_by_id(id)
                                    .map(|obj| obj.object_type() != ObjectType::WorkingSet)
                                    .unwrap_or(true)
                            });

                            // Gather all existing IDs and names in the pool before import
                            let mut pool_mut = pool.get_mut_pool().borrow_mut();
                            let existing_ids: HashSet<ObjectId> =
                                pool.get_pool().objects().iter().map(|o| o.id()).collect();
                            let existing_names: HashSet<String> = pool
                                .get_pool()
                                .objects()
                                .iter()
                                .filter_map(|o| {
                                    pool.object_info
                                        .borrow()
                                        .get(&o.id())
                                        .map(|info| info.get_name(o))
                                })
                                .collect();

                            // 1. Remap IDs for all imported objects using allowed range
                            let mut id_map: HashMap<ObjectId, ObjectId> = HashMap::new();
                            for &old_id in &to_import {
                                if let Some(obj) = import_tree.pool.object_by_id(old_id) {
                                    // Find a unique ID for this type not in pool or id_map.values()
                                    let range = EditorProject::object_id_range(obj.object_type());
                                    let pool_obj_ids: std::collections::HashSet<u16> = pool
                                        .get_pool()
                                        .objects()
                                        .iter()
                                        .map(|o| o.id().value())
                                        .collect();
                                    let already_assigned: std::collections::HashSet<u16> =
                                        id_map.values().map(|id| id.value()).collect();
                                    let mut found = None;
                                    for id in range.clone() {
                                        if !pool_obj_ids.contains(&id)
                                            && !already_assigned.contains(&id)
                                        {
                                            found = Some(id);
                                            break;
                                        }
                                    }
                                    let new_id = match found {
                                        Some(id) => ObjectId::new(id).unwrap(),
                                        None => panic!(
                                            "No available ObjectId in range {:?} for {:?}",
                                            range,
                                            obj.object_type()
                                        ),
                                    };
                                    id_map.insert(old_id, new_id);
                                }
                            }

                            // 2. Clone and remap all imported objects
                            let mut imported_objects = Vec::new();
                            for &old_id in &to_import {
                                match import_tree.pool.object_by_id(old_id) {
                                    Some(obj) => {
                                        let mut cloned = obj.clone();
                                        // Remap this object's ID
                                        if let Err(e) =
                                            cloned.mut_id().set_value(id_map[&old_id].value())
                                        {
                                            log::warn!(
                                                "Failed to remap ID for object {}: {:?}",
                                                old_id.value(),
                                                e
                                            );
                                            continue;
                                        }
                                        // Remap all referenced object IDs using the id_map
                                        remap_object_references(&mut cloned, &id_map);
                                        imported_objects.push(cloned);
                                    }
                                    None => {
                                        log::error!(
                                            "Object with ID {} not found in import pool",
                                            old_id.value()
                                        );
                                    }
                                }
                            }

                            // Sort imported objects by ID for consistent ordering
                            imported_objects.sort_by_key(|obj| obj.id().value());

                            // 3. Add imported objects to pool, renaming only if name conflicts
                            for imported_obj in &imported_objects {
                                pool_mut.add(imported_obj.clone());
                                let imported_name = pool
                                    .object_info
                                    .borrow()
                                    .get(&imported_obj.id())
                                    .map(|info| info.get_name(imported_obj));
                                let mut needs_rename = false;
                                if let Some(name) = imported_name {
                                    if existing_names.contains(&name) {
                                        needs_rename = true;
                                    }
                                } else {
                                    needs_rename = true;
                                }
                                if needs_rename {
                                    let smart_name = pool.generate_smart_name_for_new_object(
                                        imported_obj.object_type(),
                                    );
                                    pool.object_info
                                        .borrow_mut()
                                        .entry(imported_obj.id())
                                        .or_insert_with(|| {
                                            ag_iso_terminal_designer::ObjectInfo::new(imported_obj)
                                        })
                                        .set_name(smart_name);
                                }
                            }
                            // Drop the mutable borrow before calling sort_objects_by
                            drop(pool_mut);
                            // After import, sort objects so imported objects are last
                            pool.sort_objects_by(|a, b| u16::from(a.id()).cmp(&u16::from(b.id())));
                            pool.update_pool();
                            // Clear selection after import (or select first object if any)
                            let selected = pool.get_mut_selected();
                            let objects = pool.get_pool().objects();
                            if let Some(first) = objects.first() {
                                selected.replace(NullableObjectId(Some(first.id())));
                            } else {
                                selected.replace(NullableObjectId(None));
                            }
                            // Request UI repaint
                            ctx.request_repaint();
                        }
                    }
                    if close_modal {
                        self.import_tree_selection = None;
                    }
                }
            }
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("No object pool loaded, please load a pool file...");
            });
        }

        // Show settings modal if enabled
        // Only reset temp fields when modal is first opened
        if self.show_settings_modal {
            static mut LAST_MODAL_STATE: bool = false;
            let mut reset_temp_fields = false;
            unsafe {
                if !LAST_MODAL_STATE {
                    reset_temp_fields = true;
                }
                LAST_MODAL_STATE = true;
            }
            if reset_temp_fields {
                self.temp_softkey_key_width = self.softkey_key_width;
                self.temp_softkey_key_height = self.softkey_key_height;
                self.temp_key_width = self.key_width;
                self.temp_key_height = self.key_height;
                self.temp_softkey_orientation = self.softkey_mask_orientation.clone();
                self.temp_softkey_order = self.softkey_mask_key_order.clone();
                self.temp_vt_version = self.vt_version;
            }
            let mut show_settings_modal = self.show_settings_modal;
            egui::Window::new("Settings")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("SoftKeyMask Key Size:");
                    ui.horizontal(|ui| {
                        ui.label("Width:");
                        ui.add(
                            egui::Slider::new(&mut self.temp_softkey_key_width, 60..=200)
                                .text("px"),
                        );
                        ui.label("Height:");
                        ui.add(
                            egui::Slider::new(&mut self.temp_softkey_key_height, 32..=200)
                                .text("px"),
                        );
                    });
                    ui.separator();
                    ui.label("SoftKeyMask Orientation:");
                    egui::ComboBox::from_id_salt("softkey_orientation_combobox")
                        .selected_text(format!("{:?}", self.temp_softkey_orientation))
                        .show_ui(ui, |ui| {
                            for variant in [
                                SoftKeyMaskOrientation::RightRight,
                                SoftKeyMaskOrientation::LeftLeft,
                                SoftKeyMaskOrientation::TopTop,
                                SoftKeyMaskOrientation::BottomBottom,
                            ] {
                                let label = format!("{:?}", variant);
                                ui.selectable_value(
                                    &mut self.temp_softkey_orientation,
                                    variant,
                                    label,
                                );
                            }
                        });
                    ui.label("SoftKey Order:");
                    egui::ComboBox::from_id_salt("softkey_order_combobox")
                        .selected_text(format!("{:?}", self.temp_softkey_order))
                        .show_ui(ui, |ui| {
                            for variant in [
                                SoftKeyOrder::TopRight,
                                SoftKeyOrder::BottomRight,
                                SoftKeyOrder::TopLeft,
                                SoftKeyOrder::BottomLeft,
                            ] {
                                let label = format!("{:?}", variant);
                                ui.selectable_value(&mut self.temp_softkey_order, variant, label);
                            }
                        });
                    ui.separator();
                    ui.label("VT Version:");
                    egui::ComboBox::from_id_salt("vt_version_combobox")
                        .selected_text(format!("{:?}", self.temp_vt_version))
                        .show_ui(ui, |ui| {
                            for variant in [
                                ag_iso_stack::object_pool::vt_version::VtVersion::Version2,
                                ag_iso_stack::object_pool::vt_version::VtVersion::Version3,
                                ag_iso_stack::object_pool::vt_version::VtVersion::Version4,
                                ag_iso_stack::object_pool::vt_version::VtVersion::Version5,
                                ag_iso_stack::object_pool::vt_version::VtVersion::Version6,
                            ] {
                                let label = format!("{:?}", variant);
                                ui.selectable_value(&mut self.temp_vt_version, variant, label);
                            }
                        });
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Apply").clicked() {
                            self.softkey_key_width = self.temp_softkey_key_width;
                            self.softkey_key_height = self.temp_softkey_key_height;
                            self.key_width = self.temp_key_width;
                            self.key_height = self.temp_key_height;
                            self.vt_version = self.temp_vt_version;
                            self.softkey_mask_orientation = self.temp_softkey_orientation.clone();
                            self.softkey_mask_key_order = self.temp_softkey_order.clone();
                            // Save settings to file
                            let settings = crate::DesignerSettings {
                                softkey_key_width: self.softkey_key_width,
                                softkey_key_height: self.softkey_key_height,
                                softkey_mask_orientation: self.softkey_mask_orientation.clone(),
                                softkey_mask_key_order: self.softkey_mask_key_order.clone(),
                                vt_version: self.vt_version,
                            };
                            if let Ok(data) = serde_json::to_string_pretty(&settings) {
                                let _ = std::fs::write(&self.settings_path, data);
                            }
                            ui.ctx().data_mut(|data| {
                                data.insert_temp(egui::Id::new("designer_settings"), settings);
                            });
                            show_settings_modal = false;
                        }
                        if ui.button("Cancel").clicked() {
                            show_settings_modal = false;
                        }
                    });
                });
            self.show_settings_modal = show_settings_modal;
            unsafe {
                if !self.show_settings_modal {
                    LAST_MODAL_STATE = false;
                }
            }
        }
    }
}

// Removed duplicate log_top_largest_objects function (stack size version) to resolve multiple definition error.

// Estimate the total (stack + heap) size of an Object, recursively for common heap fields
fn object_total_size(obj: &Object) -> usize {
    use std::mem::size_of_val;
    match obj {
        Object::WorkingSet(ws) => {
            size_of_val(ws)
                + ws.object_refs.capacity() * ws.object_refs.get(0).map_or(0, |v| size_of_val(v))
                + ws.macro_refs.capacity() * ws.macro_refs.get(0).map_or(0, |v| size_of_val(v))
                + ws.language_codes.capacity() * std::mem::size_of::<String>()
                + ws.language_codes
                    .iter()
                    .map(|s| s.capacity())
                    .sum::<usize>()
        }
        Object::DataMask(dm) => {
            size_of_val(dm)
                + dm.object_refs.capacity() * dm.object_refs.get(0).map_or(0, |v| size_of_val(v))
                + dm.macro_refs.capacity() * dm.macro_refs.get(0).map_or(0, |v| size_of_val(v))
        }
        Object::AlarmMask(am) => {
            size_of_val(am)
                + am.object_refs.capacity() * am.object_refs.get(0).map_or(0, |v| size_of_val(v))
                + am.macro_refs.capacity() * am.macro_refs.get(0).map_or(0, |v| size_of_val(v))
        }
        Object::Container(c) => {
            size_of_val(c)
                + c.object_refs.capacity() * c.object_refs.get(0).map_or(0, |v| size_of_val(v))
                + c.macro_refs.capacity() * c.macro_refs.get(0).map_or(0, |v| size_of_val(v))
        }
        Object::SoftKeyMask(sk) => {
            size_of_val(sk)
                + sk.objects.capacity() * std::mem::size_of::<ObjectId>()
                + sk.macro_refs.capacity() * sk.macro_refs.get(0).map_or(0, |v| size_of_val(v))
        }
        Object::Key(k) => {
            size_of_val(k)
                + k.object_refs.capacity() * k.object_refs.get(0).map_or(0, |v| size_of_val(v))
                + k.macro_refs.capacity() * k.macro_refs.get(0).map_or(0, |v| size_of_val(v))
        }
        Object::Button(b) => {
            size_of_val(b)
                + b.object_refs.capacity() * b.object_refs.get(0).map_or(0, |v| size_of_val(v))
                + b.macro_refs.capacity() * b.macro_refs.get(0).map_or(0, |v| size_of_val(v))
        }
        Object::InputBoolean(ib) => {
            size_of_val(ib)
                + ib.macro_refs.capacity() * ib.macro_refs.get(0).map_or(0, |v| size_of_val(v))
        }
        Object::InputString(isg) => {
            size_of_val(isg)
                + isg.value.capacity()
                + isg.macro_refs.capacity() * isg.macro_refs.get(0).map_or(0, |v| size_of_val(v))
        }
        Object::InputNumber(inn) => {
            size_of_val(inn)
                + inn.macro_refs.capacity() * inn.macro_refs.get(0).map_or(0, |v| size_of_val(v))
        }
        Object::InputList(il) => {
            size_of_val(il)
                + il.list_items.capacity() * std::mem::size_of::<NullableObjectId>()
                + il.macro_refs.capacity() * il.macro_refs.get(0).map_or(0, |v| size_of_val(v))
        }
        Object::OutputString(os) => {
            size_of_val(os)
                + os.value.capacity()
                + os.macro_refs.capacity() * os.macro_refs.get(0).map_or(0, |v| size_of_val(v))
        }
        Object::OutputNumber(on) => {
            size_of_val(on)
                + on.macro_refs.capacity() * on.macro_refs.get(0).map_or(0, |v| size_of_val(v))
        }
        Object::OutputList(ol) => {
            size_of_val(ol)
                + ol.list_items.capacity() * std::mem::size_of::<NullableObjectId>()
                + ol.macro_refs.capacity() * ol.macro_refs.get(0).map_or(0, |v| size_of_val(v))
        }
        Object::OutputLine(oln) => {
            size_of_val(oln)
                + oln.macro_refs.capacity() * oln.macro_refs.get(0).map_or(0, |v| size_of_val(v))
        }
        Object::OutputRectangle(or) => {
            size_of_val(or)
                + or.macro_refs.capacity() * or.macro_refs.get(0).map_or(0, |v| size_of_val(v))
        }
        Object::OutputEllipse(oe) => {
            size_of_val(oe)
                + oe.macro_refs.capacity() * oe.macro_refs.get(0).map_or(0, |v| size_of_val(v))
        }
        Object::OutputPolygon(op) => {
            size_of_val(op)
                + op.points.capacity()
                    * std::mem::size_of::<ag_iso_stack::object_pool::object_attributes::Point<u16>>(
                    )
                + op.macro_refs.capacity() * op.macro_refs.get(0).map_or(0, |v| size_of_val(v))
        }
        Object::OutputMeter(om) => {
            size_of_val(om)
                + om.macro_refs.capacity() * om.macro_refs.get(0).map_or(0, |v| size_of_val(v))
        }
        Object::OutputLinearBarGraph(olg) => {
            size_of_val(olg)
                + olg.macro_refs.capacity() * olg.macro_refs.get(0).map_or(0, |v| size_of_val(v))
        }
        Object::OutputArchedBarGraph(oabg) => {
            size_of_val(oabg)
                + oabg.macro_refs.capacity() * oabg.macro_refs.get(0).map_or(0, |v| size_of_val(v))
        }
        Object::PictureGraphic(pg) => {
            std::mem::size_of_val(pg)
                + pg.data.capacity() * std::mem::size_of::<u8>()
                + pg.macro_refs.capacity()
                    * pg.macro_refs.get(0).map_or(0, |v| std::mem::size_of_val(v))
        }
        _ => size_of_val(obj),
    }
}

// Logs the top N largest objects in the ObjectPool by total (stack + heap) size
pub fn log_top_largest_objects(pool: &ObjectPool, count: usize) {
    let mut objects_with_size: Vec<_> = pool
        .objects()
        .iter()
        .map(|obj| (object_total_size(obj), obj))
        .collect();
    objects_with_size.sort_by_key(|&(size, _)| std::cmp::Reverse(size));
    let top_n = objects_with_size.into_iter().take(count);
    for (i, (size, obj)) in top_n.enumerate() {
        log::info!(
            "{}. id: {:?}, type: {:?}, total size: {} bytes",
            i + 1,
            obj.id(),
            obj.object_type(),
            size
        );
    }
    if pool.objects().is_empty() {
        log::info!("No objects in pool.");
    }
}

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // Initialize logging for native builds
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([600.0, 440.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };

    eframe::run_native(
        "AgIsoTerminalDesigner",
        native_options,
        Box::new(|cc| Ok(Box::new(DesignerApp::new(cc)))),
    )
    .ok();
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    let web_options = eframe::WebOptions::default();

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("terminal_designer_canvas_id")
            .expect("Failed to find terminal_designer_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("terminal_designer_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(DesignerApp::new(cc)))),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}

/// Find the closest color index in the palette for a given RGB value
fn find_closest_color_index(r: u8, g: u8, b: u8) -> u8 {
    fn quantize_channel(c: u8) -> u8 {
        // ((c + 25) / 51) in integer math, capped to 0..5
        let v = (c as u16 + 25) / 51;
        v.min(5) as u8
    }
    let rq = quantize_channel(r);
    let gq = quantize_channel(g);
    let bq = quantize_channel(b);

    16 + 36 * rq + 6 * gq + bq
}

#[cfg(not(target_arch = "wasm32"))]
fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
    // this is stupid... use any executor of your choice instead
    std::thread::spawn(move || futures::executor::block_on(f));
}

#[cfg(target_arch = "wasm32")]
fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}

// Extension trait for EditorProject to add import_pool
trait EditorProjectImportExt {
    fn import_pool(&mut self, imported: ObjectPool);
}

impl EditorProjectImportExt for EditorProject {
    fn import_pool(&mut self, imported: ObjectPool) {
        {
            let mut pool = self.get_mut_pool().borrow_mut();
            for obj in imported.objects() {
                pool.add(obj.clone());
            }
        }
        self.update_pool();
    }
}

fn show_import_tree_node(
    ui: &mut egui::Ui,
    obj: &Object,
    pool: &ObjectPool,
    selected: &mut std::collections::HashSet<ObjectId>,
) {
    let id = obj.id();
    let mut is_selected = selected.contains(&id);
    let label = format!("{}: {:?}", u16::from(id), obj.object_type());
    let response = ui.checkbox(&mut is_selected, label);
    if response.changed() {
        if is_selected {
            selected.insert(id);
        } else {
            selected.remove(&id);
        }
    }
    // Show children recursively
    let refs = obj.referenced_objects();
    if !refs.is_empty() {
        egui::CollapsingHeader::new("Children")
            .id_salt(format!("children_{:?}", id))
            .default_open(false)
            .show(ui, |ui| {
                for child_id in refs {
                    if let Some(child) = pool.object_by_id(child_id) {
                        show_import_tree_node(ui, child, pool, selected);
                    }
                }
            });
    }
}
