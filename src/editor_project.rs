//! Copyright 2024 - The Open-Agriculture Developers
//! SPDX-License-Identifier: GPL-3.0-or-later
//! Authors: Daan Steenbergen

use std::{cell::RefCell, collections::HashMap};

use ag_iso_stack::object_pool::{
    object::Object, NullableObjectId, ObjectId, ObjectPool, ObjectType,
};

use crate::{project_file::ProjectFile, smart_naming, ObjectInfo};

const MAX_UNDO_REDO_POOL: usize = 10;
const MAX_UNDO_REDO_SELECTED: usize = 20;

#[derive(Default, Clone)]
pub struct EditorProject {
    pool: ObjectPool,
    mut_pool: RefCell<ObjectPool>,
    undo_pool_history: Vec<ObjectPool>,
    redo_pool_history: Vec<ObjectPool>,
    selected_object: NullableObjectId,
    mut_selected_object: RefCell<NullableObjectId>,
    undo_selected_history: Vec<NullableObjectId>,
    redo_selected_history: Vec<NullableObjectId>,
    pub mask_size: u16,
    soft_key_size: (u16, u16),
    pub object_info: RefCell<HashMap<ObjectId, ObjectInfo>>,

    /// Used to keep track of the object that is being renamed
    renaming_object: RefCell<Option<(eframe::egui::Id, ObjectId, String)>>,

    /// Cached next available ID for efficient allocation
    next_available_id: RefCell<u16>,

    /// Cached default object names for efficient lookup
    default_object_names: RefCell<HashMap<ObjectId, String>>,

    /// Request to open image file dialog for PictureGraphic object
    image_load_request: RefCell<Option<ObjectId>>,
}

impl From<ObjectPool> for EditorProject {
    fn from(pool: ObjectPool) -> Self {
        let (mask_size, soft_key_size) = pool.get_minimum_mask_sizes();

        // Find the highest ID in use to initialize next_available_id
        let max_id = pool
            .objects()
            .iter()
            .map(|obj| obj.id().value())
            .max()
            .unwrap_or(0);

        EditorProject {
            mut_pool: RefCell::new(pool.clone()),
            pool,
            undo_pool_history: Default::default(),
            redo_pool_history: Default::default(),
            selected_object: NullableObjectId::default(),
            mut_selected_object: RefCell::new(NullableObjectId::default()),
            undo_selected_history: Default::default(),
            redo_selected_history: Default::default(),
            mask_size,
            soft_key_size,
            object_info: RefCell::new(HashMap::new()),
            renaming_object: RefCell::new(None),
            next_available_id: RefCell::new(max_id.saturating_add(1)),
            default_object_names: RefCell::new(HashMap::new()),
            image_load_request: RefCell::new(None),
        }
    }
}

impl EditorProject {
    /// Copy selected objects exactly (preserve IDs and references)
    pub fn copy_selected_objects_exact(&self) -> Vec<Object> {
        let mut result = Vec::new();
        if let Some(id) = self.selected_object.0 {
            if let Some(obj) = self.pool.object_by_id(id) {
                result.push(obj.clone());
            }
        }
        result
    }

    /// Copy selected objects as new (generate new IDs and update references)
    pub fn copy_selected_objects_as_new(&mut self) -> Vec<Object> {
        let mut result = Vec::new();
        if let Some(id) = self.selected_object.0 {
            if let Some(obj) = self.pool.object_by_id(id) {
                let mut new_obj = obj.clone();
                let new_id = self.allocate_object_id_for_type(obj.object_type());
                // Set new ID by matching variant
                match &mut new_obj {
                    Object::WorkingSet(o) => o.id = new_id,
                    Object::DataMask(o) => o.id = new_id,
                    Object::AlarmMask(o) => o.id = new_id,
                    Object::Container(o) => o.id = new_id,
                    Object::SoftKeyMask(o) => o.id = new_id,
                    Object::Key(o) => o.id = new_id,
                    Object::Button(o) => o.id = new_id,
                    Object::InputBoolean(o) => o.id = new_id,
                    Object::InputString(o) => o.id = new_id,
                    Object::InputNumber(o) => o.id = new_id,
                    Object::InputList(o) => o.id = new_id,
                    Object::OutputString(o) => o.id = new_id,
                    Object::OutputNumber(o) => o.id = new_id,
                    Object::OutputLine(o) => o.id = new_id,
                    Object::OutputRectangle(o) => o.id = new_id,
                    Object::OutputEllipse(o) => o.id = new_id,
                    Object::OutputPolygon(o) => o.id = new_id,
                    Object::OutputMeter(o) => o.id = new_id,
                    Object::OutputLinearBarGraph(o) => o.id = new_id,
                    Object::OutputArchedBarGraph(o) => o.id = new_id,
                    Object::PictureGraphic(o) => o.id = new_id,
                    Object::NumberVariable(o) => o.id = new_id,
                    Object::StringVariable(o) => o.id = new_id,
                    Object::FontAttributes(o) => o.id = new_id,
                    Object::LineAttributes(o) => o.id = new_id,
                    Object::FillAttributes(o) => o.id = new_id,
                    Object::InputAttributes(o) => o.id = new_id,
                    Object::ObjectPointer(o) => o.id = new_id,
                    Object::Macro(o) => o.id = new_id,
                    Object::AuxiliaryFunctionType1(o) => o.id = new_id,
                    Object::AuxiliaryInputType1(o) => o.id = new_id,
                    Object::AuxiliaryFunctionType2(o) => o.id = new_id,
                    Object::AuxiliaryInputType2(o) => o.id = new_id,
                    Object::AuxiliaryControlDesignatorType2(o) => o.id = new_id,
                    Object::WindowMask(o) => o.id = new_id,
                    Object::KeyGroup(o) => o.id = new_id,
                    Object::GraphicsContext(o) => o.id = new_id,
                    Object::OutputList(o) => o.id = new_id,
                    Object::ExtendedInputAttributes(o) => o.id = new_id,
                    Object::ColourMap(o) => o.id = new_id,
                    Object::ObjectLabelReferenceList(o) => o.id = new_id,
                    Object::ExternalObjectDefinition(o) => o.id = new_id,
                    Object::ExternalReferenceName(o) => o.id = new_id,
                    Object::ExternalObjectPointer(o) => o.id = new_id,
                    Object::Animation(o) => o.id = new_id,
                    Object::ColourPalette(o) => o.id = new_id,
                    Object::GraphicData(o) => o.id = new_id,
                    Object::WorkingSetSpecialControls(o) => o.id = new_id,
                    Object::ScaledGraphic(o) => o.id = new_id,
                }
                // TODO: Update references inside new_obj if needed
                result.push(new_obj);
            }
        }
        result
    }

    /// Paste objects into the pool
    pub fn paste_objects(&mut self, objects: Vec<Object>) {
        {
            let mut pool = self.mut_pool.borrow_mut();
            for obj in objects {
                pool.add(obj);
            }
        } // pool is dropped here before update_pool is called
        self.update_pool();
    }
    /// Get the current object pool
    pub fn get_pool(&self) -> &ObjectPool {
        &self.pool
    }

    /// Returns the allowed ID range for a given object type
    pub fn object_id_range(object_type: ObjectType) -> std::ops::RangeInclusive<u16> {
        match object_type {
            ObjectType::WorkingSet => 0..=0,
            ObjectType::DataMask => 1000..=1999,
            ObjectType::AlarmMask => 2000..=2999,
            ObjectType::Container => 3000..=3999,
            ObjectType::SoftKeyMask => 4000..=4999,
            ObjectType::Key => 5000..=5999,
            ObjectType::Button => 6000..=6999,
            ObjectType::InputBoolean => 7000..=7999,
            ObjectType::InputString => 8000..=8999,
            ObjectType::InputNumber => 9000..=9999,
            ObjectType::InputList => 10000..=10999,
            ObjectType::OutputString => 11000..=11999,
            ObjectType::OutputNumber => 12000..=12999,
            ObjectType::OutputList => 37000..=37999,
            ObjectType::OutputLine => 13000..=13999,
            ObjectType::OutputRectangle => 14000..=14999,
            ObjectType::OutputEllipse => 15000..=15999,
            ObjectType::OutputPolygon => 16000..=16999,
            ObjectType::OutputMeter => 17000..=17999,
            ObjectType::OutputLinearBarGraph => 18000..=18999,
            ObjectType::OutputArchedBarGraph => 19000..=19999,
            ObjectType::PictureGraphic => 20000..=20999,
            ObjectType::NumberVariable => 21000..=21999,
            ObjectType::StringVariable => 22000..=22999,
            ObjectType::FontAttributes => 23000..=23999,
            ObjectType::LineAttributes => 24000..=24999,
            ObjectType::FillAttributes => 25000..=25999,
            ObjectType::InputAttributes => 26000..=26999,
            ObjectType::ObjectPointer => 27000..=27999,
            ObjectType::Macro => 28000..=28999,
            ObjectType::AuxiliaryFunctionType1 => 29000..=29999,
            ObjectType::AuxiliaryInputType1 => 30000..=30999,
            ObjectType::AuxiliaryFunctionType2 => 31000..=31999,
            ObjectType::AuxiliaryInputType2 => 32000..=32999,
            ObjectType::AuxiliaryControlDesignatorType2 => 33000..=33999,
            ObjectType::ColourMap => 34000..=34999,
            ObjectType::GraphicsContext => 35000..=35999,
            ObjectType::ColourPalette => 36000..=36999,
            ObjectType::GraphicData => 37000..=37999,
            ObjectType::WorkingSetSpecialControls => 38000..=38999,
            ObjectType::ScaledGraphic => 39000..=39999,
            ObjectType::WindowMask => 40000..=40999,
            ObjectType::KeyGroup => 41000..=41999,
            ObjectType::ExtendedInputAttributes => 42000..=42999,
            ObjectType::ExternalObjectPointer => 43000..=43999,
            ObjectType::ExternalObjectDefinition => 44000..=44999,
            ObjectType::ExternalReferenceName => 45000..=45999,
            ObjectType::ObjectLabelReferenceList => 46000..=46999,
            ObjectType::Animation => 47000..=47999,
        }
    }

    /// Allocate a new unique object ID within the allowed range for the given object type
    pub fn allocate_object_id_for_type(&self, object_type: ObjectType) -> ObjectId {
        let range = Self::object_id_range(object_type);
        let pool = &self.pool;
        for id in range.clone() {
            let oid = ObjectId::new(id).unwrap_or_default();
            if pool.object_by_id(oid).is_none() {
                return oid;
            }
        }
        panic!(
            "No available ObjectId in range {:?} for {:?}",
            range, object_type
        );
    }

    /// Update the next available ID cache based on the current pool
    fn update_next_available_id(&self) {
        let max_id = self
            .pool
            .objects()
            .iter()
            .map(|obj| obj.id().value())
            .max()
            .unwrap_or(0);
        self.next_available_id.replace(max_id.saturating_add(1));
    }

    /// Get the current selected object
    pub fn get_selected(&self) -> NullableObjectId {
        self.selected_object
    }

    /// Get the current mutating object pool
    /// This is used to make changes to the pool in the next frame
    /// without affecting the current pool
    pub fn get_mut_pool(&self) -> &RefCell<ObjectPool> {
        &self.mut_pool
    }

    /// Set the mutating selected object
    /// This is used to make changes to the selected object in the next frame
    /// without affecting the current selected object
    pub fn get_mut_selected(&self) -> &RefCell<NullableObjectId> {
        &self.mut_selected_object
    }

    /// If the mutating pool is different from the current pool, add the current pool to the history
    /// and update the current pool with the mutated pool.
    /// Returns true if the pool was updated
    pub fn update_pool(&mut self) -> bool {
        if self.mut_pool.borrow().to_owned() != self.pool {
            self.redo_pool_history.clear();
            self.undo_pool_history.push(self.pool.clone());
            if self.undo_pool_history.len() > MAX_UNDO_REDO_POOL {
                self.undo_pool_history
                    .drain(..self.undo_pool_history.len() - MAX_UNDO_REDO_POOL);
            }
            self.pool = self.mut_pool.borrow().clone();
            // Clear the default names cache since objects may have changed
            self.default_object_names.borrow_mut().clear();
            return true;
        }
        false
    }

    /// Undo the last action
    pub fn undo(&mut self) {
        if let Some(pool) = self.undo_pool_history.pop() {
            self.redo_pool_history.push(self.pool.clone());

            // Both need to be replaced here because otherwise it will be added to the undo history
            self.pool = pool.clone();
            self.mut_pool.replace(pool);

            // Update next_available_id based on the new pool state
            self.update_next_available_id();

            // Clear the default names cache since objects may have changed
            self.default_object_names.borrow_mut().clear();
        }
    }

    /// Check if there are actions available to undo
    pub fn undo_available(&self) -> bool {
        !self.undo_pool_history.is_empty()
    }

    /// Redo the last undone action
    pub fn redo(&mut self) {
        if let Some(pool) = self.redo_pool_history.pop() {
            self.undo_pool_history.push(self.pool.clone());
            // Both need to be replaced here because otherwise the redo history will be cleared
            self.pool = pool.clone();
            self.mut_pool.replace(pool);

            // Update next_available_id based on the new pool state
            self.update_next_available_id();

            // Clear the default names cache since objects may have changed
            self.default_object_names.borrow_mut().clear();
        }
    }

    /// Check if there are actions available to redo
    pub fn redo_available(&self) -> bool {
        !self.redo_pool_history.is_empty()
    }

    /// Update the selected object with the mutating selected object if it is different
    /// Returns true if the selected object was updated
    pub fn update_selected(&mut self) -> bool {
        let mut_selected = self.mut_selected_object.borrow().to_owned();
        if mut_selected != self.selected_object {
            self.redo_selected_history.clear();
            if mut_selected != NullableObjectId::NULL {
                self.undo_selected_history.push(self.selected_object);
                if self.undo_selected_history.len() > MAX_UNDO_REDO_SELECTED {
                    self.undo_selected_history
                        .drain(..self.undo_selected_history.len() - MAX_UNDO_REDO_SELECTED);
                }
            }
            self.selected_object = mut_selected;
            return true;
        }
        false
    }

    /// Set the selected object to the previous object in the history
    pub fn set_previous_selected(&mut self) {
        if let Some(selected) = self.undo_selected_history.pop() {
            self.redo_selected_history.push(self.selected_object);
            // Both need to be replaced here because otherwise it will be added to the undo history
            self.selected_object = selected.clone();
            self.mut_selected_object.replace(selected);
        }
    }

    /// Set the selected object to the next object in the history
    pub fn set_next_selected(&mut self) {
        if let Some(selected) = self.redo_selected_history.pop() {
            self.undo_selected_history.push(self.selected_object);
            // Both need to be replaced here because otherwise the redo history will be cleared
            self.selected_object = selected.clone();
            self.mut_selected_object.replace(selected);
        }
    }

    /// Change an object id in the object info hashmap
    pub fn update_object_id_for_info(&self, old_id: ObjectId, new_id: ObjectId) {
        let mut object_info = self.object_info.borrow_mut();
        if let Some(info) = object_info.remove(&old_id) {
            object_info.insert(new_id, info);
        }
    }

    /// Get the object info for an object id
    /// If the object id is not mapped, we insert the default object info
    pub fn get_object_info(&self, object: &Object) -> ObjectInfo {
        let mut object_info = self.object_info.borrow_mut();
        object_info
            .entry(object.id())
            .or_insert_with(|| ObjectInfo::new(object))
            .clone()
    }

    /// Start renaming an object
    pub fn set_renaming_object(&self, ui_id: eframe::egui::Id, object_id: ObjectId, name: String) {
        self.renaming_object.replace(Some((ui_id, object_id, name)));
    }

    /// Get the current name of the object that is being renamed
    /// Returns None if no object is being renamed
    pub fn get_renaming_object(&self) -> Option<(eframe::egui::Id, ObjectId, String)> {
        self.renaming_object.borrow().clone()
    }

    /// Finish renaming an object
    /// If store is true, we store the new name in the object info hashmap
    pub fn finish_renaming_object(&self, store: bool) {
        if store {
            if let Some(renaming_object) = self.renaming_object.borrow().as_ref() {
                let mut object_info = self.object_info.borrow_mut();
                if let Some(info) = object_info.get_mut(&renaming_object.1) {
                    info.set_name(renaming_object.2.clone());
                }
            }
        }
        self.renaming_object.replace(None);
    }

    pub fn sort_objects_by<F>(&mut self, cmp: F)
    where
        F: Fn(&Object, &Object) -> std::cmp::Ordering,
    {
        self.mut_pool.borrow_mut().objects_mut().sort_by(cmp);
    }

    /// Get all existing object names for validation
    pub fn get_all_object_names(&self) -> HashMap<String, ObjectType> {
        let mut names = HashMap::new();
        let object_info = self.object_info.borrow();
        let mut default_names_cache = self.default_object_names.borrow_mut();

        for object in self.pool.objects() {
            let name = if let Some(info) = object_info.get(&object.id()) {
                info.get_name(object)
            } else {
                // Use cached default name if available, otherwise generate and cache it
                default_names_cache
                    .entry(object.id())
                    .or_insert_with(|| {
                        format!(
                            "Object {} ({})",
                            object.id().value(),
                            smart_naming::get_object_type_name(object.object_type())
                        )
                    })
                    .clone()
            };
            names.entry(name).or_insert(object.object_type());
        }
        names
    }

    /// Generate a smart default name for a new object
    pub fn generate_smart_name_for_new_object(&self, object_type: ObjectType) -> String {
        let existing_names = self.get_all_object_names();
        smart_naming::generate_smart_default_name(object_type, &existing_names)
    }

    /// Apply smart naming to all objects efficiently
    pub fn apply_smart_naming_to_all_objects(&self) {
        let mut object_info = self.object_info.borrow_mut();

        // Build existing names map once for all remaining objects
        let mut existing_names = HashMap::new();
        for (id, info) in object_info.iter() {
            if let Some(name) = &info.name {
                if let Some(object) = self.pool.object_by_id(*id) {
                    existing_names
                        .entry(name.clone())
                        .or_insert(object.object_type());
                }
            }
        }

        // Generate names for remaining objects
        for object in self.pool.objects() {
            let new_name =
                smart_naming::generate_smart_default_name(object.object_type(), &existing_names);

            // Update the count for the new name to ensure uniqueness
            existing_names
                .entry(new_name.clone())
                .or_insert(object.object_type());

            let info = object_info
                .entry(object.id())
                .or_insert_with(|| ObjectInfo::new(object));
            info.set_name(new_name);
        }
    }

    /// Apply smart naming to an existing object if it doesn't have a custom name
    pub fn apply_smart_naming_to_object(&self, object: &Object) {
        let mut object_info = self.object_info.borrow_mut();

        // Check if the object already has a name
        if let Some(info) = object_info.get(&object.id()) {
            if info.name.is_some() {
                return; // Already has a custom name
            }
        }

        // Build names map inline to avoid extra iteration
        let mut existing_names = HashMap::new();
        let mut default_names_cache = self.default_object_names.borrow_mut();
        for obj in self.pool.objects() {
            let name = if let Some(info) = object_info.get(&obj.id()) {
                info.get_name(obj)
            } else if obj.id() == object.id() {
                continue; // Skip the object we're naming
            } else {
                default_names_cache
                    .entry(obj.id())
                    .or_insert_with(|| {
                        format!(
                            "Object {} ({})",
                            obj.id().value(),
                            smart_naming::get_object_type_name(obj.object_type())
                        )
                    })
                    .clone()
            };
            existing_names.entry(name).or_insert(obj.object_type());
        }

        let new_name =
            smart_naming::generate_smart_default_name(object.object_type(), &existing_names);

        let info = object_info
            .entry(object.id())
            .or_insert_with(|| ObjectInfo::new(object));
        info.set_name(new_name);
    }

    /// Save the project to a file
    pub fn save_project(&self) -> Result<Vec<u8>, serde_json::Error> {
        // Make sure we're saving the current state
        let object_info = self.object_info.borrow();
        let selected = if self.mut_selected_object.borrow().0.is_some() {
            self.mut_selected_object.borrow().0
        } else {
            self.selected_object.0
        };

        let project = ProjectFile::new(&self.pool, &object_info, self.mask_size, selected);
        project.to_bytes()
    }

    /// Load a project from file data
    pub fn load_project(data: Vec<u8>) -> Result<Self, String> {
        let project = ProjectFile::from_bytes(&data)
            .map_err(|e| format!("Failed to parse project file: {}", e))?;
        let pool = project.load_pool()?;
        let settings = project.get_settings();

        let mut editor_project = EditorProject::from(pool);
        editor_project.mask_size = settings.mask_size;

        // Restore object metadata
        let metadata = project.get_metadata();
        let mut object_info = editor_project.object_info.borrow_mut();
        let mut_pool = editor_project.get_mut_pool();
        for object in editor_project.pool.objects() {
            if let Some(meta) = metadata.get(&object.id().value()) {
                let info = object_info
                    .entry(object.id())
                    .or_insert_with(|| ObjectInfo::new(object));
                if let Some(name) = &meta.name {
                    info.set_name(name.clone());
                }
            }
        }
        drop(object_info);

        // Apply smart naming to objects without custom names
        for object in editor_project.pool.objects() {
            editor_project.apply_smart_naming_to_object(object);
        }

        // Restore last selected
        if let Some(selected_id) = settings.last_selected {
            if let Ok(id) = ObjectId::new(selected_id) {
                editor_project.selected_object = NullableObjectId(Some(id));
                editor_project
                    .mut_selected_object
                    .replace(NullableObjectId(Some(id)));
            }
        }

        Ok(editor_project)
    }

    /// Request to open image file dialog for a PictureGraphic object
    pub fn request_image_load(&self, object_id: ObjectId) {
        self.image_load_request.replace(Some(object_id));
    }

    /// Take and clear the image load request if any
    pub fn take_image_load_request(&self) -> Option<ObjectId> {
        self.image_load_request.replace(None)
    }
}
