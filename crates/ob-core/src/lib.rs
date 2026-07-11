pub mod device;
pub mod event;
pub mod layout;
pub mod protocol;
pub mod screen;
pub mod window;

pub use device::{DeviceId, DeviceInfo, DeviceRole};
pub use event::{InputEvent, KeyEvent, MouseButton, MouseEvent, ScrollEvent};
pub use layout::{LayoutConfig, ScreenTopology};
pub use screen::ScreenRect;
pub use protocol::{Message, MessageType};
pub use screen::{ScreenId, ScreenInfo};
pub use window::{TransferState, WindowInfo, WindowId};
