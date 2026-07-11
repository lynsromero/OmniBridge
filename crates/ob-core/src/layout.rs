use crate::screen::ScreenRect;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenTopology {
    pub screens: Vec<TopologyScreen>,
    pub connections: Vec<ScreenConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyScreen {
    pub device_id: crate::device::DeviceId,
    pub screen_id: crate::screen::ScreenId,
    pub global_rect: ScreenRect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenConnection {
    pub from_device: crate::device::DeviceId,
    pub to_device: crate::device::DeviceId,
    pub from_screen: crate::screen::ScreenId,
    pub to_screen: crate::screen::ScreenId,
    pub direction: Direction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    pub device_positions: HashMap<crate::device::DeviceId, DevicePosition>,
    pub edges: Vec<EdgeConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevicePosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeConfig {
    pub from: crate::device::DeviceId,
    pub to: crate::device::DeviceId,
    pub direction: Direction,
    pub enabled: bool,
}

impl ScreenTopology {
    pub fn new() -> Self {
        Self { screens: Vec::new(), connections: Vec::new() }
    }

    pub fn add_screen(&mut self, screen: TopologyScreen) {
        self.screens.push(screen);
    }

    pub fn add_connection(&mut self, conn: ScreenConnection) {
        self.connections.push(conn);
    }

    pub fn find_adjacent_device(
        &self,
        from_device: crate::device::DeviceId,
        direction: Direction,
    ) -> Option<crate::device::DeviceId> {
        self.connections
            .iter()
            .find(|c| c.from_device == from_device && c.direction == direction)
            .map(|c| c.to_device)
    }

    pub fn global_virtual_bounds(&self) -> ScreenRect {
        if self.screens.is_empty() {
            return ScreenRect::new(0, 0, 0, 0);
        }
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        for screen in &self.screens {
            min_x = min_x.min(screen.global_rect.x);
            min_y = min_y.min(screen.global_rect.y);
            max_x = max_x.max(screen.global_rect.right());
            max_y = max_y.max(screen.global_rect.bottom());
        }
        ScreenRect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

impl Default for ScreenTopology {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self { device_positions: HashMap::new(), edges: Vec::new() }
    }
}
