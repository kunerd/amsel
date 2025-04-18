use std::time::Duration;

use iced::widget::{column, container, text, text_input};
use iced::{Element, Length, Task, Theme};
use player_core::Video;

// #[derive(Debug, Deserialize)]
// struct ListResponse {
//     items: Vec<Video>,
// }

// #[derive(Debug, Deserialize)]
// struct Video {
//     id: HashMap<String, String>,
// }

fn main() -> iced::Result {
    iced::application(Player::new, Player::update, Player::view)
        .title(Player::title)
        // .font(icon::FONT)
        // .subscription(Icebreaker::subscription)
        .theme(Player::theme)
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    SearchChanged(String),
    SearchCooled,
    VideosListed(Vec<Video>),
}

struct Player {
    search: String,
    search_temperature: usize,
    is_searching: bool,
}

impl Player {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                search: String::new(),
                search_temperature: 0,
                is_searching: false,
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
            Message::VideosListed(videos) => {
                self.is_searching = false;
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
            container(text("Not implemented, yet!")).center(Length::Fill)
        };

        column![search, content].into()
    }

    fn theme(&self) -> Theme {
        Theme::TokyoNightStorm
    }
}
