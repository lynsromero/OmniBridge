use ob_core::device::DeviceId;
use ob_core::layout::Direction;
use ob_core::screen::ScreenRect;


pub struct EdgeDetector {
    screen_bounds: Vec<(DeviceId, ScreenRect)>,
    edge_threshold: i32,
    current_device: DeviceId,
}

impl EdgeDetector {
    pub fn new(current_device: DeviceId, edge_threshold: i32) -> Self {
        Self {
            screen_bounds: Vec::new(),
            edge_threshold,
            current_device,
        }
    }

    pub fn update_screens(&mut self, screens: Vec<(DeviceId, ScreenRect)>) {
        self.screen_bounds = screens;
    }

    pub fn detect_edge_crossing(
        &self,
        x: i32,
        y: i32,
    ) -> Option<(DeviceId, Direction)> {
        let current = self.screen_bounds.iter().find(|(id, _)| *id == self.current_device)?;
        let (_, current_rect) = current;

        let direction = if x <= current_rect.x + self.edge_threshold {
            Some(Direction::Left)
        } else if x >= current_rect.right() - self.edge_threshold {
            Some(Direction::Right)
        } else if y <= current_rect.y + self.edge_threshold {
            Some(Direction::Top)
        } else if y >= current_rect.bottom() - self.edge_threshold {
            Some(Direction::Bottom)
        } else {
            return None;
        };

        let dir = direction?;

        for (device_id, rect) in &self.screen_bounds {
            if *device_id == self.current_device {
                continue;
            }

            match dir {
                Direction::Right => {
                    if (current_rect.right() - rect.x).abs() < self.edge_threshold * 2
                        && rect.y <= y && y <= rect.bottom()
                    {
                        return Some((*device_id, dir));
                    }
                }
                Direction::Left => {
                    if (rect.right() - current_rect.x).abs() < self.edge_threshold * 2
                        && rect.y <= y && y <= rect.bottom()
                    {
                        return Some((*device_id, dir));
                    }
                }
                Direction::Bottom => {
                    if (current_rect.bottom() - rect.y).abs() < self.edge_threshold * 2
                        && rect.x <= x && x <= rect.right()
                    {
                        return Some((*device_id, dir));
                    }
                }
                Direction::Top => {
                    if (rect.bottom() - current_rect.y).abs() < self.edge_threshold * 2
                        && rect.x <= x && x <= rect.right()
                    {
                        return Some((*device_id, dir));
                    }
                }
            }
        }

        None
    }

    pub fn translate_coordinates(
        &self,
        x: i32,
        y: i32,
        from_device: DeviceId,
        to_device: DeviceId,
        direction: Direction,
    ) -> Option<(i32, i32)> {
        let from_rect = self.screen_bounds.iter()
            .find(|(id, _)| *id == from_device)?.1;
        let to_rect = self.screen_bounds.iter()
            .find(|(id, _)| *id == to_device)?.1;

        match direction {
            Direction::Right => {
                let rel_y = y - from_rect.y;
                Some((to_rect.x, to_rect.y + rel_y))
            }
            Direction::Left => {
                let rel_y = y - from_rect.y;
                Some((to_rect.right(), to_rect.y + rel_y))
            }
            Direction::Bottom => {
                let rel_x = x - from_rect.x;
                Some((to_rect.x + rel_x, to_rect.y))
            }
            Direction::Top => {
                let rel_x = x - from_rect.x;
                Some((to_rect.x + rel_x, to_rect.bottom()))
            }
        }
    }
}
