mod player;
use player::Player;

use std::time::Duration;

use iced::widget::{
    button, column, container, horizontal_rule, horizontal_space, row, scrollable, text, text_input,
};
use iced::{Alignment, Element, Length, Subscription, Task, Theme};
use player_core::{Error, Video, backend};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title(App::title)
        // .font(icon::FONT)
        .subscription(App::subscription)
        .theme(App::theme)
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    SearchChanged(String),
    SearchCooled,
    VideosListed(Result<Vec<Video>, Error>),
    VideoSelected(usize),
    Backend(backend::Event),
    Player(player::Message),
}

struct App {
    search: String,
    search_temperature: usize,
    is_searching: bool,

    videos: Vec<Video>,
    player: Option<Player>,

    backend: Backend,
}

enum Backend {
    Starting,
    Started(player_core::Backend),
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                search: String::new(),
                search_temperature: 0,
                is_searching: false,

                videos: Vec::new(),
                player: None,
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

                let (player, task) = Player::new(video.clone(), backend.clone());
                self.player = Some(player);

                task.map(Message::Player)
            }
            Message::Backend(event) => match event {
                backend::Event::Started(backend) => {
                    self.backend = Backend::Started(backend);
                    Task::none()
                }
                backend::Event::PlaybackPosition(pos) => {
                    let Some(player) = &mut self.player else {
                        return Task::none();
                    };

                    player.set_cur_pos(pos);

                    Task::none()
                }
            },
            Message::Player(message) => {
                let Some(player) = &mut self.player else {
                    return Task::none();
                };

                let Backend::Started(backend) = &self.backend else {
                    return Task::none();
                };

                player.update(message, backend.clone()).map(Message::Player)
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

        let player: Element<_> = match &self.player {
            Some(player) => player.view().map(Message::Player).into(),
            None => container(text("Choose a file to start playback.")).into(),
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
