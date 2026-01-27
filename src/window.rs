use std::str::FromStr;
use std::sync::{Arc, Weak};

use arc_swap::ArcSwap;
use iced::alignment::Vertical;
use iced::widget::{Column, Row, button, column, container, row, space, text, text_input};
use iced::{Background, Element, Length, Size, Subscription};

use crate::config::resolution::Resolution;
use crate::config::{self, Config, Hotkey};
use crate::keylogger::KeyEvent;
use crate::manager::Manager;

#[derive(Debug)]
struct Window {
    old_config: Config,
    config: Arc<ArcSwap<Config>>,
    colors: [String; 2],
    thin: String,
    tall: String,
    wide: String,
    changing: Option<Hotkey>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Message {
    Change(Hotkey),
    SetColor(usize, String),
    SetResolution(Hotkey, String),
    KeyEvent(KeyEvent),
    Save,
}

impl Window {
    fn new() -> Self {
        let old_config = Config::load_from_file();
        let colors = old_config
            .colors
            .map(|color| iced::Color::from(color).to_string());
        let thin = old_config.thin.to_string();
        let tall = old_config.tall.to_string();
        let wide = old_config.wide.to_string();

        let config = Arc::new(ArcSwap::from_pointee(old_config));

        Self {
            old_config,
            config,
            colors,
            thin,
            tall,
            wide,
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
            Message::SetResolution(hotkey, resolution) => {
                match Resolution::from_str(&resolution) {
                    Ok(resolution) => {
                        self.config
                            .rcu(move |config| config.set_resolution(hotkey, resolution));
                    }
                    Err(e) => {
                        println!("{:?}", e);
                    }
                }

                match hotkey {
                    Hotkey::Tall => {
                        self.tall = resolution;
                    }
                    Hotkey::Thin => {
                        self.thin = resolution;
                    }
                    Hotkey::Wide => {
                        self.wide = resolution;
                    }
                }
            }
            Message::SetColor(i, color) => {
                self.colors[i] = color;

                if let Ok(color) = iced::Color::from_str(&self.colors[i]) {
                    self.config.rcu(|config| {
                        let mut config = **config;
                        config.colors[i] = config::color::Color::from(color);
                        config
                    });
                }
            }
            Message::Save => {
                let config = self.config.load_full();
                config.save_to_file().unwrap();
                self.old_config = *config;
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let config = *self.config.load_full();

        let hotkeys = Column::with_children(
            [
                ("Tall", &self.tall, Hotkey::Tall),
                ("Thin", &self.thin, Hotkey::Thin),
                ("Wide", &self.wide, Hotkey::Wide),
            ]
            .into_iter()
            .map(|(name, resolution, hotkey)| {
                row![
                    text(name).width(Length::Fill),
                    text_input("Resolution", resolution)
                        .width(100)
                        .on_input(move |resolution| Message::SetResolution(hotkey, resolution)),
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
                .spacing(6)
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
                .on_press_maybe((config != self.old_config).then_some(Message::Save))
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
