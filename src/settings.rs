use ag_iso_stack::object_pool::object::SoftKeyMaskOrientation;
use ag_iso_stack::object_pool::object::SoftKeyOrder;
use ag_iso_stack::object_pool::vt_version::VtVersion;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignerSettings {
    pub softkey_key_width: u16,
    pub softkey_key_height: u16,
    pub softkey_mask_orientation: SoftKeyMaskOrientation,
    pub softkey_mask_key_order: SoftKeyOrder,
    pub vt_version: VtVersion,
}
