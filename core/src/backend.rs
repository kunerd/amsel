use std::time::Duration;

use futures::{
    SinkExt,
    channel::mpsc::{self},
};
use rodio::{Decoder, Source};
use stream_download::{
    Settings, StreamDownload,
    process::{ProcessStreamParams, YtDlpCommand},
    storage::temp::TempStorageProvider,
};
use youtube_dl::YoutubeDl;

#[derive(Debug, Clone)]
pub struct Backend(mpsc::Sender<Command>);

impl Backend {
    pub async fn load_and_play(mut self, id: String) -> Option<Duration> {
        let format = "m4a";
        let url = format!("https://www.youtube.com/watch?v={id}");
        let output = YoutubeDl::new(&url)
            .format(format)
            .extract_audio(true)
            .run_async()
            .await
            .expect("meta data")
            .into_single_video()
            .expect("to extract video metadata");

        let size = output.filesize.expect("file size") as u64;

        let cmd = YtDlpCommand::new(url).extract_audio(true).format(format);
        let reader = StreamDownload::new_process(
            ProcessStreamParams::new(cmd).unwrap().content_length(size),
            TempStorageProvider::new(),
            // Disable cancel_on_drop to ensure no error messages from the process are lost.
            Settings::default().cancel_on_drop(false),
        )
        .await
        .unwrap();

        let reader = reader;
        let decoder = tokio::task::spawn_blocking(|| rodio::Decoder::new(reader).unwrap())
            .await
            .unwrap();
        let duration = decoder.total_duration();

        self.0.send(Command::PlayStream(decoder)).await.unwrap();

        duration
    }

    pub async fn seek_to(mut self, pos: Duration) -> Duration {
        self.0.send(Command::Seek(pos.clone())).await.unwrap();

        pos
    }

    pub async fn play(mut self) {
        self.0.send(Command::Play).await.unwrap();
    }

    pub async fn pause(mut self) {
        self.0.send(Command::Pause).await.unwrap();
    }
}

pub enum Command {
    PlayStream(Decoder<StreamDownload<TempStorageProvider>>),
    Play,
    Pause,
    Seek(Duration),
}

#[derive(Debug, Clone)]
pub enum Event {
    Started(Backend),
    PlaybackPosition(Duration),
}

pub fn start() -> impl futures::Stream<Item = Event> {
    let (event_tx, event_rx) = mpsc::channel(100);

    std::thread::spawn(|| run(event_tx));

    event_rx
}

enum State {
    Starting,
    Running(rodio::Sink, rodio::OutputStream, mpsc::Receiver<Command>),
}

fn run(mut sender: mpsc::Sender<Event>) {
    let mut state = State::Starting;

    loop {
        match state {
            State::Starting => {
                let (command_tx, command_rx) = mpsc::channel(100);

                let (stream, handle) = rodio::OutputStream::try_default().unwrap();
                let sink = rodio::Sink::try_new(&handle).unwrap();
                let backend = Backend(command_tx);

                sender.try_send(Event::Started(backend)).unwrap();
                state = State::Running(sink, stream, command_rx);
            }
            State::Running(ref sink, ref _stream, ref mut command_rx) => {
                match command_rx.try_next() {
                    Ok(Some(command)) => match command {
                        Command::PlayStream(decoder) => {
                            sink.clear();
                            sink.append(decoder);
                            sink.play();
                        }
                        Command::Play => {
                            sink.play();
                        }
                        Command::Pause => {
                            sink.pause();
                        }
                        Command::Seek(pos) => {
                            sink.try_seek(pos).unwrap();
                        }
                    },
                    Ok(None) => {
                        dbg!("No one is interested anymore.");
                        return;
                    }
                    Err(_err) => {
                        let _ = sender.try_send(Event::PlaybackPosition(sink.get_pos()));
                        std::thread::sleep(Duration::from_millis(20));
                    }
                }
            }
        }
    }
}
