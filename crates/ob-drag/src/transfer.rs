use anyhow::Result;
use ob_core::device::DeviceId;
use ob_core::window::WindowInfo;
use ob_capture::WindowCapturer;
use ob_codec::encoder::VideoEncoder;
use tokio::sync::mpsc;
use tracing::{info, warn};

pub struct WindowTransferManager {
    active_transfers: Vec<ActiveTransfer>,
    event_tx: mpsc::Sender<TransferEvent>,
}

struct ActiveTransfer {
    window: WindowInfo,
    #[allow(dead_code)]
    source_device: DeviceId,
    target_device: DeviceId,
    capturer: WindowCapturer,
    encoder: VideoEncoder,
    running: bool,
}

#[derive(Debug, Clone)]
pub enum TransferEvent {
    TransferStarted {
        window: WindowInfo,
        target_device: DeviceId,
    },
    TransferFrame {
        window_id: ob_core::window::WindowId,
        encoded_data: Vec<u8>,
        frame_number: u64,
    },
    TransferComplete {
        window: WindowInfo,
        target_device: DeviceId,
    },
    TransferError {
        error: String,
    },
}

impl WindowTransferManager {
    pub fn new(event_tx: mpsc::Sender<TransferEvent>) -> Self {
        Self {
            active_transfers: Vec::new(),
            event_tx,
        }
    }

    pub async fn start_transfer(
        &mut self,
        window: WindowInfo,
        source_device: DeviceId,
        target_device: DeviceId,
    ) -> Result<()> {
        info!(
            "Starting window transfer: '{}' from {} to {}",
            window.title, source_device, target_device
        );

        let capturer = WindowCapturer::new(window.clone());
        let encoder = VideoEncoder::new(window.width, window.height, 30);

        let transfer = ActiveTransfer {
            window: window.clone(),
            source_device,
            target_device,
            capturer,
            encoder,
            running: true,
        };

        self.active_transfers.push(transfer);

        let _ = self.event_tx.send(TransferEvent::TransferStarted {
            window: window.clone(),
            target_device,
        }).await;

        Ok(())
    }

    pub async fn process_frame(&mut self, window_id: ob_core::window::WindowId) -> Result<()> {
        for transfer in &mut self.active_transfers {
            if transfer.window.id == window_id && transfer.running {
                if let Ok(Some(frame)) = transfer.capturer.capture_frame() {
                    match transfer.encoder.encode_frame(&frame) {
                        Ok(encoded) => {
                            let _ = self.event_tx.send(TransferEvent::TransferFrame {
                                window_id,
                                encoded_data: encoded.data,
                                frame_number: encoded.frame_number,
                            }).await;
                        }
                        Err(e) => {
                            warn!("Encoding failed: {}", e);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn stop_transfer(&mut self, window_id: ob_core::window::WindowId) -> Result<()> {
        for transfer in &mut self.active_transfers {
            if transfer.window.id == window_id {
                transfer.running = false;
                transfer.capturer.stop();

                let _ = self.event_tx.send(TransferEvent::TransferComplete {
                    window: transfer.window.clone(),
                    target_device: transfer.target_device,
                }).await;

                info!("Window transfer stopped: {}", transfer.window.title);
                break;
            }
        }

        self.active_transfers.retain(|t| t.window.id != window_id);
        Ok(())
    }

    pub fn active_transfer_count(&self) -> usize {
        self.active_transfers.iter().filter(|t| t.running).count()
    }
}
