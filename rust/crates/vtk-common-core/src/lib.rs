//! Port of VTK::CommonCore. See ROADMAP.md Phase 1 for scope and dependency order.

use std::os::unix::fs::PermissionsExt;

pub fn mode_of(p: std::fs::Permissions) -> u32 {
    p.mode()
}
