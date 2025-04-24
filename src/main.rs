use std::time::Duration;

use iced::widget::{
    button, column, container, horizontal_rule, horizontal_space, row, scrollable, slider, text,
    text_input, vertical_slider,
};
use iced::{Alignment, Element, Length, Subscription, Task, Theme};
use player_core::{Error, Video, backend};

fn main() -> iced::Result {
    iced::application(Player::new, Player::update, Player::view)
        .title(Player::title)
        // .font(icon::FONT)
        .subscription(Player::subscription)
        .theme(Player::theme)
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    SearchChanged(String),
    SearchCooled,
    VideosListed(Result<Vec<Video>, Error>),
    VideoSelected(usize),
    PlayheadMoved(f32),
    Backend(backend::Event),
    PlaybackStarted(Video, u64),
    PlaybackPositionChanged(Duration),
}

struct Player {
    search: String,
    search_temperature: usize,
    is_searching: bool,

    videos: Vec<Video>,
    playing: State,

    backend: Backend,
}

enum State {
    Idle,
    Playing(Video, Duration, Duration),
    Pause(Video),
}

enum Backend {
    Starting,
    Started(player_core::Backend),
}

impl Player {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                search: String::new(),
                search_temperature: 0,
                is_searching: false,

                videos: Vec::new(),
                playing: State::Idle,
                backend: Backend::Starting,
            },
            Task::none(),
        )
    }

    fn title(&self) -> String {
        "Player".to_string()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SearchChanged(search) => {
                self.search = search;
                self.search_temperature += 1;

                Task::perform(tokio::time::sleep(Duration::from_secs(1)), |_| {
                    Message::SearchCooled
                })
            }
            Message::SearchCooled => {
                self.search_temperature = self.search_temperature.saturating_sub(1);

                if self.search_temperature == 0 {
                    self.is_searching = true;

                    Task::perform(Video::search(self.search.clone()), Message::VideosListed)
                } else {
                    Task::none()
                }
            }
            Message::VideosListed(Ok(videos)) => {
                self.is_searching = false;
                self.videos = videos;

                Task::none()
            }
            Message::VideosListed(Err(err)) => {
                dbg!(err);
                Task::none()
            }
            Message::VideoSelected(index) => {
                let Some(video) = &self.videos.get(index) else {
                    return Task::none();
                };

                let Backend::Started(backend) = &self.backend else {
                    return Task::none();
                };

                let video = video.clone().clone();
                Task::perform(backend.clone().play(video.id.clone()), move |length| {
                    Message::PlaybackStarted(video, length)
                })
            }
            Message::PlayheadMoved(pos) => {
                let State::Playing(_, cur_pos, length) = &mut self.playing else {
                    return Task::none();
                };
                let Backend::Started(backend) = &self.backend else {
                    return Task::none();
                };

                dbg!(pos);
                let new_pos = pos / 100.0 * length.as_secs_f32();
                *cur_pos = Duration::from_secs_f32(new_pos);

                Task::perform(
                    backend.clone().seek_to(Duration::from_secs_f32(new_pos)),
                    Message::PlaybackPositionChanged,
                )
            }
            Message::Backend(event) => match event {
                backend::Event::Started(backend) => {
                    self.backend = Backend::Started(backend);
                    Task::none()
                }
                backend::Event::PlaybackPosition(duration) => {
                    let State::Playing(_, cur_pos, _) = &mut self.playing else {
                        return Task::none();
                    };

                    *cur_pos = duration;

                    Task::none()
                }
                backend::Event::PlaybackDuration(duration) => {
                    let State::Playing(_, _, length) = &mut self.playing else {
                        return Task::none();
                    };

                    *length = duration.expect("valid duration");

                    Task::none()
                }
            },
            Message::PlaybackStarted(video, length) => {
                self.playing =
                    State::Playing(video, Duration::from_secs(0), Duration::from_secs(1));

                Task::none()
            }
            Message::PlaybackPositionChanged(pos) => {
                let State::Playing(_, cur_pos, _) = &mut self.playing else {
                    return Task::none();
                };

                // *cur_pos = pos;

                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let search = text_input("Search videos ...", &self.search)
            .size(20)
            .padding(10)
            .on_input(Message::SearchChanged);

        let content = if self.is_searching || self.search_temperature > 0 {
            container(text("Searching...")).center(Length::Fill)
        } else {
            if self.videos.is_empty() {
                container(text("No videos found!")).center(Length::Fill)
            } else {
                let list = scrollable(
                    column(self.videos.iter().enumerate().map(|(i, v)| {
                        button(text(&v.title))
                            .on_press(Message::VideoSelected(i))
                            .width(Length::Fill)
                            .style(button::secondary)
                            .into()
                    }))
                    .spacing(5),
                );

                container(list).center(Length::Fill)
            }
        };

        let player = {
            match &self.playing {
                State::Idle => container(text("Choose a file to start playback.")),
                State::Playing(video, cur_pos, length) => {
                    let normalized_pos = cur_pos.as_secs_f32() / length.as_secs_f32() * 100.0;
                    let format_time = |time: &Duration| {
                        let secs = time.as_secs();

                        let (minutes, secs) = (secs / 60, secs % 60);

                        if minutes >= 60 {
                            let (hours, minutes) = (minutes / 60, minutes % 60);

                            format!("{}:{:0>2}:{:0>2}", hours, minutes, secs)
                        } else {
                            format!("{:0>2}:{:0>2}", minutes, secs)
                        }
                    };
                    container(row![
                        horizontal_space().width(Length::FillPortion(1)),
                        column![
                            text(&video.title),
                            row![
                                text(format_time(cur_pos)),
                                slider(0.0..=100.0, normalized_pos, Message::PlayheadMoved),
                                text(format_time(length))
                            ]
                            .spacing(10)
                            .align_y(Alignment::Center)
                        ]
                        .spacing(5)
                        .width(Length::FillPortion(1)),
                        horizontal_space()
                    ])
                    .padding(10)
                }
                State::Pause(_video) => todo!(),
            }
        };

        container(column![search, content, horizontal_rule(1), player].spacing(10))
            .padding(10)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::run(backend::start).map(Message::Backend)
    }

    fn theme(&self) -> Theme {
        Theme::TokyoNightStorm
    }
}
