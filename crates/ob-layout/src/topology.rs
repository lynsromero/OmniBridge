use ob_core::device::DeviceId;
use ob_core::layout::{Direction, LayoutConfig, ScreenConnection, ScreenTopology, TopologyScreen};
use ob_core::screen::{ScreenId, ScreenRect};
use std::collections::HashMap;
use tracing::info;

pub struct LayoutTopology {
    pub topology: ScreenTopology,
    device_screens: HashMap<DeviceId, Vec<ScreenId>>,
}

impl LayoutTopology {
    pub fn new() -> Self {
        Self {
            topology: ScreenTopology::new(),
            device_screens: HashMap::new(),
        }
    }

    pub fn add_device_screen(
        &mut self,
        device_id: DeviceId,
        screen: TopologyScreen,
    ) {
        self.device_screens
            .entry(device_id)
            .or_default()
            .push(screen.screen_id);
        self.topology.add_screen(screen);
        info!("Added screen for device {}", device_id);
    }

    pub fn connect_screens(
        &mut self,
        from_device: DeviceId,
        to_device: DeviceId,
        from_screen: ScreenId,
        to_screen: ScreenId,
        direction: Direction,
    ) {
        self.topology.add_connection(ScreenConnection {
            from_device,
            to_device,
            from_screen,
            to_screen,
            direction,
        });
        info!("Connected {:?} -> {:?}", from_device, to_device);
    }

    pub fn find_adjacent_device(
        &self,
        from_device: DeviceId,
        direction: Direction,
    ) -> Option<DeviceId> {
        self.topology.find_adjacent_device(from_device, direction)
    }

    pub fn virtual_bounds(&self) -> ScreenRect {
        self.topology.global_virtual_bounds()
    }

    pub fn auto_layout(&mut self, devices: &[(DeviceId, Vec<ScreenRect>)]) {
        info!("Computing auto-layout for {} devices", devices.len());

        let mut offset_x = 0;
        for (device_id, screens) in devices {
            if let Some(main_screen) = screens.first() {
                let rect = ScreenRect::new(
                    offset_x,
                    0,
                    main_screen.width,
                    main_screen.height,
                );

                let screen_id = ScreenId(0);
                self.add_device_screen(
                    *device_id,
                    TopologyScreen {
                        device_id: *device_id,
                        screen_id,
                        global_rect: rect,
                    },
                );

                offset_x += main_screen.width + 10;
            }
        }

        let device_ids: Vec<DeviceId> = devices.iter().map(|(id, _)| *id).collect();
        for i in 0..device_ids.len().saturating_sub(1) {
            self.connect_screens(
                device_ids[i],
                device_ids[i + 1],
                ScreenId(0),
                ScreenId(0),
                Direction::Right,
            );
        }
    }

    pub fn to_config(&self) -> LayoutConfig {
        let mut positions = HashMap::new();
        for screen in &self.topology.screens {
            positions.insert(
                screen.device_id,
                ob_core::layout::DevicePosition {
                    x: screen.global_rect.x,
                    y: screen.global_rect.y,
                },
            );
        }

        let edges = self.topology.connections.iter().map(|c| {
            ob_core::layout::EdgeConfig {
                from: c.from_device,
                to: c.to_device,
                direction: c.direction,
                enabled: true,
            }
        }).collect();

        LayoutConfig { device_positions: positions, edges }
    }
}

impl Default for LayoutTopology {
    fn default() -> Self {
        Self::new()
    }
}
