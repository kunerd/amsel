use std::time::Duration;

use iced::widget::{
    button, column, container, horizontal_rule, horizontal_space, row, scrollable, slider, text,
    text_input,
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
    PlayPressed,
    PausePressed,
    Backend(backend::Event),
    PlaybackStarted(Video, Duration, Duration),
    PlayheadMoved(f32),
    PlaybackPositionChanged(Duration),
    VideoPaused,
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
    Loading(Video),
    Playing(Video, Duration, Duration),
    Pause(Video, Duration, Duration),
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
                let Some(video) = &self.videos.get(index).cloned() else {
                    return Task::none();
                };

                let Backend::Started(backend) = &self.backend else {
                    return Task::none();
                };

                self.playing = State::Loading(video.clone());

                let video = video.clone();
                Task::perform(
                    backend.clone().load_and_play(video.id.clone()),
                    move |duration| {
                        Message::PlaybackStarted(video, Duration::from_secs(0), duration.unwrap())
                    },
                )
            }
            Message::PlayheadMoved(pos) => {
                let State::Playing(_, cur_pos, length) = &mut self.playing else {
                    return Task::none();
                };
                let Backend::Started(backend) = &self.backend else {
                    return Task::none();
                };

                let new_pos = pos * length.as_secs_f32();
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
            },
            Message::PlaybackStarted(video, cur_pos, duration) => {
                self.playing = State::Playing(video, cur_pos, duration);

                Task::none()
            }
            Message::PlaybackPositionChanged(_pos) => {
                let State::Playing(_, _cur_pos, _) = &mut self.playing else {
                    return Task::none();
                };

                // *cur_pos = pos;

                Task::none()
            }
            Message::PausePressed => {
                let Backend::Started(backend) = &self.backend else {
                    return Task::none();
                };

                Task::perform(backend.clone().pause(), |_| Message::VideoPaused)
            }
            Message::VideoPaused => {
                let State::Playing(video, cur_pos, duration) = &self.playing else {
                    return Task::none();
                };

                self.playing = State::Pause(video.clone(), *cur_pos, *duration);

                Task::none()
            }
            Message::PlayPressed => {
                let State::Pause(video, cur_pos, duration) = &self.playing else {
                    return Task::none();
                };

                let Backend::Started(backend) = &self.backend else {
                    return Task::none();
                };

                let video = video.clone();
                let cur_pos = *cur_pos;
                let duration = *duration;
                Task::perform(backend.clone().play(), move |_| {
                    Message::PlaybackStarted(video, cur_pos, duration)
                })
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
                        button(
                            row![
                                text(&v.title),
                                horizontal_space(),
                                text!(
                                    "{:02}:{:02}:{:02}",
                                    v.duration.num_hours(),
                                    v.duration.num_minutes() % 60,
                                    v.duration.num_seconds() % 60
                                )
                            ]
                            .align_y(Alignment::Center),
                        )
                        .on_press(Message::VideoSelected(i))
                        .width(Length::Fill)
                        .style(button::secondary)
                        .into()
                    }))
                    .spacing(5),
                )
                .spacing(5);

                container(list).center(Length::Fill)
            }
        };

        let player = {
            match &self.playing {
                State::Idle => container(text("Choose a file to start playback.")),
                State::Loading(video) => container(row![
                    text(&video.title).width(Length::FillPortion(1)),
                    container(text("Loading...")).center_x(Length::FillPortion(1)),
                    horizontal_space()
                ])
                .padding(10),
                State::Playing(video, cur_pos, duration) => {
                    let normalized_pos = cur_pos.as_secs_f32() / duration.as_secs_f32();
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
                        text(&video.title).width(Length::FillPortion(1)),
                        column![
                            container(button("Pause").on_press(Message::PausePressed))
                                .center_x(Length::Fill),
                            row![
                                text(format_time(cur_pos)),
                                slider(0.0..=1.0, normalized_pos, Message::PlayheadMoved)
                                    .step(0.01),
                                text(format_time(duration))
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
                State::Pause(video, cur_pos, duration) => {
                    let normalized_pos = cur_pos.as_secs_f32() / duration.as_secs_f32() * 100.0;
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
                        text(&video.title).width(Length::FillPortion(1)),
                        column![
                            container(button("Play").on_press(Message::PlayPressed))
                                .center_x(Length::Fill),
                            row![
                                text(format_time(cur_pos)),
                                slider(0.0..=100.0, normalized_pos, Message::PlayheadMoved),
                                text(format_time(duration))
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
