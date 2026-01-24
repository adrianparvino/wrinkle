use std::sync::{Arc, Weak};

use arc_swap::ArcSwap;
use iced::alignment::Vertical;
use iced::widget::{Column, button, row, text};
use iced::{Element, Length, Size, Subscription};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::config::{Config, Hotkey};
use crate::keylogger::KeyEvent;
use crate::manager::Manager;

#[derive(Debug)]
struct Window {
    config: Arc<ArcSwap<Config>>,
    changing: Option<Hotkey>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Message {
    Change(Hotkey),
    KeyEvent(KeyEvent),
}

impl Window {
    fn new() -> Self {
        let config = Arc::new(ArcSwap::from_pointee(Config::load_from_file()));

        Self {
            config,
            changing: None,
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::KeyEvent(ev) => {
                let Some(changing) = self.changing else {
                    return;
                };
                let config = self.config.load_full().set_hotkey(
                    changing,
                    Some(crate::keylogger::KeyFilter {
                        char: ev.char,
                        modifiers: Some(ev.modifiers),
                    }),
                );
                config.save_to_file().unwrap();
                self.config.store(Arc::new(config));
                self.changing = None;
            }
            Message::Change(hotkey) => {
                self.changing = Some(hotkey);
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let config = *self.config.load_full();

        let hotkeys = Column::with_children(
            [
                ("Tall", Hotkey::Tall),
                ("Thin", Hotkey::Thin),
                ("Wide", Hotkey::Wide),
            ]
            .into_iter()
            .map(|(name, hotkey)| {
                row![
                    text(name).width(Length::Fill),
                    button(
                        (if Some(hotkey) == self.changing {
                            text!("...")
                        } else {
                            config
                                .get_hotkey(hotkey)
                                .map(|hotkey| text!("{}", hotkey))
                                .unwrap_or(text!("Unset"))
                        })
                        .center(),
                    )
                    .width(Length::Fixed(100.0))
                    .on_press(Message::Change(hotkey))
                ]
                .width(Length::Fill)
                .align_y(Vertical::Center)
                .into()
            }),
        );

        hotkeys.spacing(6).padding(16).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::run_with(Weak::into_raw(Arc::downgrade(&self.config)), |config| {
            let config = unsafe { Weak::from_raw(*config).upgrade().unwrap() };
            let (tx, rx) = mpsc::channel(100);

            tokio::task::spawn(async {
                let mut manager = Manager::spawn(config).await;
                manager.run(tx).await;
            });

            ReceiverStream::new(rx)
        })
        .map(|ev| Message::KeyEvent(ev))
    }
}

pub fn spawn() {
    iced::application(Window::new, Window::update, Window::view)
        .subscription(Window::subscription)
        .window_size(Size {
            width: 480.0,
            height: 360.0,
        })
        .run()
        .unwrap();
}
