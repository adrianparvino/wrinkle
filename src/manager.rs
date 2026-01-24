use std::sync::Arc;

use arc_swap::{ArcSwap, ArcSwapOption};
use tokio::sync::{mpsc, oneshot};
use windows::{
    Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, MSG, SetProcessDPIAware, TranslateMessage,
    },
    core::BOOL,
};

use crate::{
    config::{Config, Hotkey},
    instance::MinecraftInstance,
    projector::Projector,
};
use crate::{
    instance::MinecraftInstanceListener,
    keylogger::{KeyEvent, KeyLogger},
};

pub struct Manager {
    pub instance: Arc<ArcSwapOption<MinecraftInstance>>,
    pub projector: Projector,
    pub key_channel: mpsc::Receiver<KeyEvent>,
    pub config: Arc<ArcSwap<Config>>,
    pub state: Option<Hotkey>,
}

impl Manager {
    pub async fn spawn(config: Arc<ArcSwap<Config>>) -> Self {
        unsafe {
            SetProcessDPIAware().unwrap();
        }

        let (tx, rx) = mpsc::channel(100);
        let (projector_tx, projector) = oneshot::channel();
        let instance = Arc::new(ArcSwapOption::empty());

        std::thread::spawn({
            let config = config.clone();
            let instance = instance.clone();
            move || {
                MinecraftInstanceListener::spawn({
                    let instance = instance.clone();
                    Box::new(move |hwnd| {
                        let found_instance = MinecraftInstance::new(hwnd);

                        let lpmi = found_instance.get_monitor_info();
                        found_instance.set_window_pos(
                            lpmi.rcMonitor.left,
                            lpmi.rcMonitor.top,
                            lpmi.rcMonitor.right - lpmi.rcMonitor.left,
                            lpmi.rcMonitor.bottom - lpmi.rcMonitor.top,
                        );

                        instance.store(Some(Arc::new(found_instance)));
                    })
                });
                projector_tx
                    .send(Projector::spawn(instance, config))
                    .unwrap();
                let _ = KeyLogger::spawn(tx);

                let mut msg = MSG::default();
                unsafe {
                    loop {
                        match GetMessageW(&raw mut msg, None, 0, 0) {
                            BOOL(-1) => {}
                            BOOL(0) => {
                                break;
                            }
                            BOOL(_) => {
                                let _ = TranslateMessage(&raw const msg);
                                DispatchMessageW(&raw const msg);
                            }
                        }
                    }
                }
            }
        });

        let projector = projector.await.unwrap();

        Manager {
            key_channel: rx,
            projector,
            instance,
            state: None,
            config,
        }
    }

    fn update_state(&mut self, hotkey: Hotkey) {
        let config = *self.config.as_ref().load_full();
        let Some(instance) = self.instance.load_full() else {
            return;
        };

        if self.state == Some(hotkey) {
            let lpmi = instance.get_monitor_info();

            instance.set_window_pos(
                lpmi.rcMonitor.left,
                lpmi.rcMonitor.top,
                lpmi.rcMonitor.right - lpmi.rcMonitor.left,
                lpmi.rcMonitor.bottom - lpmi.rcMonitor.top,
            );

            self.state = None;
        } else if hotkey == Hotkey::Tall {
            let lpmi = instance.get_monitor_info();

            let center_x = lpmi.rcMonitor.left.midpoint(lpmi.rcMonitor.right);
            let center_y = lpmi.rcMonitor.top.midpoint(lpmi.rcMonitor.bottom);

            let Config {
                tall_width,
                tall_height,
                ..
            } = config;

            instance.set_window_pos(
                center_x - tall_width / 2,
                center_y - tall_height / 2,
                tall_width,
                tall_height,
            );

            self.state = Some(Hotkey::Tall);
        } else if hotkey == Hotkey::Thin {
            let lpmi = instance.get_monitor_info();

            let center_x = lpmi.rcMonitor.left.midpoint(lpmi.rcMonitor.right);
            let center_y = lpmi.rcMonitor.top.midpoint(lpmi.rcMonitor.bottom);

            let Config {
                thin_width,
                thin_height,
                ..
            } = config;

            instance.set_window_pos(
                center_x - thin_width / 2,
                center_y - thin_height / 2,
                thin_width,
                thin_height,
            );

            self.state = Some(Hotkey::Thin);
        } else if hotkey == Hotkey::Wide {
            let lpmi = instance.get_monitor_info();

            let center_x = lpmi.rcMonitor.left.midpoint(lpmi.rcMonitor.right);
            let center_y = lpmi.rcMonitor.top.midpoint(lpmi.rcMonitor.bottom);

            let Config {
                wide_width,
                wide_height,
                ..
            } = config;

            instance.set_window_pos(
                center_x - wide_width / 2,
                center_y - wide_height / 2,
                wide_width,
                wide_height,
            );

            self.state = Some(Hotkey::Wide);
        }

        self.projector
            .set_visibility(self.state == Some(Hotkey::Tall));
    }

    pub async fn run(&mut self, tx: mpsc::Sender<KeyEvent>) {
        while let Some(ev) = self.key_channel.recv().await {
            let _ = tx.try_send(ev);

            let Some(instance) = self.instance.load_full() else {
                continue;
            };
            if !instance.is_foreground() {
                continue;
            }

            let config = **self.config.as_ref().load();
            if let Some(tall) = config.tall
                && tall.test(ev)
            {
                self.update_state(Hotkey::Tall);
            } else if let Some(thin) = config.thin
                && thin.test(ev)
            {
                self.update_state(Hotkey::Thin);
            } else if let Some(wide) = config.wide
                && wide.test(ev)
            {
                self.update_state(Hotkey::Wide);
            }
        }
    }
}
