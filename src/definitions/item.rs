// Copyright (c) 2017-2021 Decode Detroit
// Author: Patton Doyle
// Licence: GNU GPLv3
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! This module defines a basic identifier system (ItemId, ItemDescription,
//! and ItemPair) to allow robust identification of all events,
//! scenes, and other items in the program.

// Import standard library modules
use std::fmt;

// Import Serde macros
use serde::{Deserialize, Serialize};

/// Define the All Stop command (a.k.a. emergency stop)
const ALL_STOP: u32 = 0;

/// This structure is a generic identifier for a configuration element (e.g. event, scene, status).
///
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct ItemId {
    id: u32,
}

// Implement key ItemId struct features
impl ItemId {
    /// A function to create an item id from u32, CAN-compliant version.
    ///
    /// When program is built with CAN checking ON (default), this function will cap valid
    /// IDs at 29 bits (i.e. 2^29 or roughly half a million IDs). If you need
    /// additional IDs (unlikely) and are not using a CAN bus, you can
    /// disable this checking by rebuilding the program with the no_can_limit
    /// feature.
    ///
    /// # Disabling CAN Checking
    ///
    /// Your program was likely compiled with CAN checking ON. Rebuild your
    /// program without CAN checking with
    ///
    /// ```
    /// cargo build --features no_can_limit
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// let item = ItemId::new(1);
    /// ```
    ///
    /// # Errors
    ///
    /// This function will return None if the address exceeds the 29-bit
    /// address limit or conflicts with the ALL_STOP id.
    ///
    #[cfg(not(feature = "no_can_limit"))]
    pub fn new(id: u32) -> Option<ItemId> {
        // Verify id does not conflict with all stop
        if id == ALL_STOP {
            return None;
        }

        // Verify 29 bit limit for CAN bus
        if id >= 0x1FFFFFFF {
            return None;
        }

        // Return the new item id
        Some(ItemId { id })
    }

    /// A function to create an item id from u32, no CAN version.
    ///
    /// When the program is built with CAN checking OFF, this function will not
    /// check IDs validity for the CAN range. This should not be used in
    /// conjunction with a CAN bus.
    ///
    /// # Enabling CAN Checking
    ///
    /// Your program was likely compiled with CAN checking OFF. Rebuild
    /// your program with CAN checking (enabled by default) with
    ///
    /// ```
    /// cargo build
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// let item = ItemId::new(0xFFFFFFFF);
    /// ```
    ///
    /// # Errors
    ///
    /// This function will return None if the address conflicts with the
    /// ALL_STOP id.
    ///
    #[cfg(feature = "no_can_limit")]
    pub fn new(id: u32) -> Option<ItemId> {
        // Verify id does not conflict with all stop
        if id == ALL_STOP {
            return None;
        }

        // Return the new item id
        Some(ItemId { id })
    }

    /// A function to create an item id from u32, unchecked version.
    ///
    /// This function does not verify that the new ItemId complies with the CAN
    /// limit and does not check that the new ItemId does not collide with the
    /// all stop item id. This is useful when either (or both) of these cases
    /// are possible and desired or if the valid range is checked elsewhere.
    ///
    pub fn new_unchecked(id: u32) -> ItemId {
        ItemId { id }
    }

    /// A method to return the id of the item.
    ///
    /// # Examples
    ///
    /// ```
    /// let id = ItemId::new(5);
    /// assert_eq!(5, id.id());
    /// ```
    ///
    pub fn id(&self) -> u32 {
        self.id
    }

    /// A function to return a new all stop item id. This is a reserved id for
    /// halting all active processes returning operations to static state.
    ///
    pub fn all_stop() -> ItemId {
        ItemId { id: ALL_STOP }
    }
}

// Implement displaying that shows the ID
impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

// Tests of the item module
#[cfg(test)]
mod tests {
    use super::*;

    // Test id with CAN bus limiter
    #[test]
    #[cfg_attr(not(feature = "no_can_limit"), should_panic)]
    fn create_id() {
        // Try to create an id out of CAN range
        let _e = ItemId::new(0xFFFFFFFF).unwrap();
    }
}