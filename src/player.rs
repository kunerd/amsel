use std::time::Duration;

use iced::{
    Alignment, Element, Length, Task,
    widget::{button, column, container, horizontal_space, row, slider, text},
};
use player_core::{Backend, Video};

#[derive(Debug, Clone)]
pub enum Message {
    PausePressed,
    PlayPressed,
    PlayheadMoved(f32),
    PlaybackStarted(Video, Duration, Duration),
    VideoPaused,
}

pub struct Player {
    video: Video,
    cur_pos: Duration,
    duration: Duration,
    state: State,
}

enum State {
    Loading,
    Playing,
    Pause,
}

impl Player {
    pub fn new(video: Video, backend: Backend) -> (Self, Task<Message>) {
        (
            Self {
                video: video.clone(),
                cur_pos: Duration::from_secs(0),
                duration: video.duration.to_std().unwrap(),
                state: State::Loading,
            },
            Task::perform(backend.load_and_play(video.id.clone()), move |duration| {
                Message::PlaybackStarted(video, Duration::from_secs(0), duration.unwrap())
            }),
        )
    }

    pub fn update(&mut self, message: Message, backend: Backend) -> Task<Message> {
        match message {
            Message::PlayheadMoved(pos) => {
                let new_pos = pos * self.duration.as_secs_f32();
                self.cur_pos = Duration::from_secs_f32(new_pos);

                Task::perform(backend.seek_to(Duration::from_secs_f32(new_pos)), |_| {}).discard()
            }
            Message::PlaybackStarted(video, cur_pos, duration) => {
                self.video = video;

                self.cur_pos = cur_pos;
                self.duration = duration;

                self.state = State::Playing;

                Task::none()
            }
            Message::PausePressed => Task::perform(backend.pause(), |_| Message::VideoPaused),
            Message::VideoPaused => {
                self.state = State::Pause;

                Task::none()
            }
            Message::PlayPressed => {
                let video = self.video.clone();
                let cur_pos = self.cur_pos;
                let duration = self.duration;

                Task::perform(backend.play(), move |_| {
                    Message::PlaybackStarted(video, cur_pos, duration)
                })
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let normalized_pos = self.cur_pos.as_secs_f32() / self.duration.as_secs_f32();

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

        let controls: Element<_> = match self.state {
            State::Loading => text("Loading...").into(),
            State::Playing => button("Pause").on_press(Message::PausePressed).into(),
            State::Pause => button("Play").on_press(Message::PlayPressed).into(),
        };

        container(
            row![
                text(&self.video.title).width(Length::FillPortion(1)),
                column![
                    container(controls).center_x(Length::Fill),
                    row![
                        text(format_time(&self.cur_pos)),
                        slider(0.0..=1.0, normalized_pos, Message::PlayheadMoved).step(0.01),
                        text(format_time(&self.duration))
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center)
                ]
                .spacing(10)
                .width(Length::FillPortion(1)),
                horizontal_space()
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding(10)
        .into()
    }

    pub fn set_cur_pos(&mut self, pos: Duration) {
        self.cur_pos = pos;
    }
}
