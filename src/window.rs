use std::str::FromStr;
use std::sync::{Arc, Weak};

use arc_swap::ArcSwap;
use iced::alignment::Vertical;
use iced::widget::{Column, Row, button, column, container, row, space, text, text_input};
use iced::{Background, Element, Length, Size, Subscription};

use crate::config::{self, Config, Hotkey};
use crate::keylogger::KeyEvent;
use crate::manager::Manager;

#[derive(Debug)]
struct Window {
    config: Arc<ArcSwap<Config>>,
    colors: [String; 2],
    changing: Option<Hotkey>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Message {
    Change(Hotkey),
    SetColor(usize, String),
    KeyEvent(KeyEvent),
    Save,
}

impl Window {
    fn new() -> Self {
        let config = Arc::new(ArcSwap::from_pointee(Config::load_from_file()));
        let colors = config
            .load_full()
            .colors
            .map(|color| iced::Color::from(color).to_string());

        Self {
            config,
            colors,
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
                self.config.store(Arc::new(config));
                self.changing = None;
            }
            Message::Change(hotkey) => {
                self.changing = Some(hotkey);
            }
            Message::SetColor(i, color) => {
                self.colors[i] = color;

                if let Ok(color) = iced::Color::from_str(&self.colors[i]) {
                    self.config.rcu(|config| {
                        let mut config = **config;
                        config.colors[i] = config::Color::from(color);
                        config
                    });
                }
            }
            Message::Save => {
                self.config.load_full().save_to_file().unwrap();
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let config = *self.config.load_full();

        let hotkeys = Column::with_children(
            [
                ("Thin", Hotkey::Thin),
                ("Tall", Hotkey::Tall),
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
        )
        .spacing(6);

        let colors = Row::with_children((0..self.colors.len()).map(|i| {
            row![
                text_input("Color", &self.colors[i])
                    .width(Length::Fill)
                    .on_input(move |color| Message::SetColor(i, color)),
                container("")
                    .height(Length::Fill)
                    .width(40)
                    .style(move |theme| container::Style {
                        background: Some(Background::from(iced::Color::from(config.colors[i]))),
                        ..container::rounded_box(theme)
                    })
            ]
            .spacing(6)
            .into()
        }))
        .height(24)
        .spacing(12);

        let save = row![
            space().width(Length::Fill),
            button(text!("Save").center())
                .width(100)
                .on_press(Message::Save)
        ];

        column![hotkeys, colors, space().height(Length::Fill), save]
            .spacing(6)
            .padding(16)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::run_with(Weak::into_raw(Arc::downgrade(&self.config)), |config| {
            let config = unsafe { Weak::from_raw(*config).upgrade().unwrap() };
            iced::stream::channel(100, async |tx| {
                let mut manager = Manager::spawn(config).await;
                manager.run(tx).await;
            })
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
