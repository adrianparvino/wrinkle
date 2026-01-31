use std::sync::Arc;

use arc_swap::{ArcSwap, ArcSwapOption};
use futures::{SinkExt, StreamExt};
use futures_channel::{mpsc, oneshot};
use windows::{
    Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, MSG, SPI_GETMOUSESPEED, SPI_SETMOUSESPEED,
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetProcessDPIAware, SystemParametersInfoW,
        TranslateMessage,
    },
    core::BOOL,
};

use crate::{
    config::{Config, Hotkey, xy::XY},
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
    mouse_speed: i32,
}

impl Drop for Manager {
    fn drop(&mut self) {
        self.slow_mouse(false);
    }
}

impl Manager {
    fn slow_mouse(&self, slow: bool) {
        unsafe {
            if slow {
                SystemParametersInfoW(
                    SPI_SETMOUSESPEED,
                    0,
                    Some(1 as *mut _),
                    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS::default(),
                )
                .unwrap();
            } else {
                SystemParametersInfoW(
                    SPI_SETMOUSESPEED,
                    0,
                    Some(self.mouse_speed as usize as *mut _),
                    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS::default(),
                )
                .unwrap();
            }
        }
    }

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
                        log::debug!("Received new minecraft instance: {:?}", hwnd);

                        let found_instance = MinecraftInstance::new(hwnd);

                        let lpmi = found_instance.get_monitor_info();
                        found_instance.set_window_pos((
                            XY::new(lpmi.rcMonitor.left, lpmi.rcMonitor.top),
                            XY::new(
                                lpmi.rcMonitor.right - lpmi.rcMonitor.left,
                                lpmi.rcMonitor.bottom - lpmi.rcMonitor.top,
                            ),
                        ));

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

        let mouse_speed = unsafe {
            let mut mouse_speed = 0i32;

            SystemParametersInfoW(
                SPI_GETMOUSESPEED,
                0,
                Some(&raw mut mouse_speed as *mut _),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS::default(),
            )
            .unwrap();

            mouse_speed
        };

        Manager {
            key_channel: rx,
            projector,
            instance,
            state: None,
            config,
            mouse_speed,
        }
    }

    fn update_state(&mut self, hotkey: Hotkey) {
        let config = *self.config.as_ref().load_full();
        let Some(instance) = self.instance.load_full() else {
            return;
        };

        let lpmi = instance.get_monitor_info();

        let center_x = lpmi.rcMonitor.left.midpoint(lpmi.rcMonitor.right);
        let center_y = lpmi.rcMonitor.top.midpoint(lpmi.rcMonitor.bottom);

        let rect = match hotkey {
            _ if self.state == Some(hotkey) => {
                log::debug!("Setting normal");

                self.state = None;

                (
                    XY::new(lpmi.rcMonitor.left, lpmi.rcMonitor.top),
                    XY::new(
                        lpmi.rcMonitor.right - lpmi.rcMonitor.left,
                        lpmi.rcMonitor.bottom - lpmi.rcMonitor.top,
                    ),
                )
            }
            Hotkey::Tall => {
                log::debug!("Setting tall");

                let Config { tall, .. } = config;

                self.state = Some(Hotkey::Tall);

                (
                    XY::new(center_x - tall.x / 2, center_y - tall.y / 2),
                    XY::new(tall.x, tall.y),
                )
            }
            Hotkey::Thin => {
                log::debug!("Setting thin");

                let Config { thin, .. } = config;

                self.state = Some(Hotkey::Thin);

                (
                    XY::new(center_x - thin.x / 2, center_y - thin.y / 2),
                    XY::new(thin.x, thin.y),
                )
            }
            Hotkey::Wide => {
                log::debug!("Setting wide");

                let Config { wide, .. } = config;

                self.state = Some(Hotkey::Wide);

                (
                    XY::new(center_x - wide.x / 2, center_y - wide.y / 2),
                    XY::new(wide.x, wide.y),
                )
            }
        };

        instance.set_window_pos(rect);

        self.projector.hotkey_hook(self.state);
        self.slow_mouse(self.state == Some(Hotkey::Tall));
    }

    pub async fn run(&mut self, mut tx: mpsc::Sender<KeyEvent>) {
        while let Some(ev) = self.key_channel.next().await {
            let _ = tx.send(ev).await;

            let Some(instance) = self.instance.load_full() else {
                continue;
            };
            if !instance.is_foreground() {
                continue;
            }

            let config = **self.config.as_ref().load();
            if let Some(tall) = config.tall_key
                && tall.test(ev)
            {
                self.update_state(Hotkey::Tall);
            } else if let Some(thin) = config.thin_key
                && thin.test(ev)
            {
                self.update_state(Hotkey::Thin);
            } else if let Some(wide) = config.wide_key
                && wide.test(ev)
            {
                self.update_state(Hotkey::Wide);
            }
        }
    }
}
