use std::time::Duration;

use futures::{SinkExt, channel::mpsc};
use rodio::Source;
use stream_download::{
    Settings, StreamDownload,
    process::{ProcessStreamParams, YtDlpCommand},
    storage::temp::TempStorageProvider,
};
use youtube_dl::YoutubeDl;

#[derive(Debug, Clone)]
pub struct Backend(mpsc::Sender<Command>);

impl Backend {
    pub async fn play(mut self, id: String) -> u64 {
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

        // let reader_handle = reader.handle();
        let reader = Box::new(reader);

        self.0.send(Command::Play(reader)).await.unwrap();

        size
    }

    pub async fn seek_to(mut self, pos: Duration) -> Duration {
        self.0.send(Command::Seek(pos.clone())).await;

        pos
    }
}

pub enum Command {
    Play(Box<StreamDownload<TempStorageProvider>>),
    Seek(Duration),
}

#[derive(Debug, Clone)]
pub enum Event {
    Started(Backend),
    PlaybackPosition(Duration),
    PlaybackDuration(Option<Duration>),
}

pub fn start() -> impl futures::Stream<Item = Event> {
    let (event_tx, event_rx) = mpsc::channel(100);

    // let runner =
    //     stream::once(async { tokio::task::spawn(run(event_tx)).await }).map(|_| unreachable!());
    tokio::task::spawn_blocking(|| run(event_tx));

    // stream::select(event_rx, runner)
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

                let _ = sender.try_send(Event::Started(backend));
                state = State::Running(sink, stream, command_rx);
            }
            State::Running(ref sink, ref _stream, ref mut command_rx) => {
                match command_rx.try_next() {
                    Ok(Some(command)) => match command {
                        Command::Play(reader) => {
                            sink.clear();
                            let decoder = rodio::Decoder::new(reader).unwrap();
                            sender.try_send(Event::PlaybackDuration(decoder.total_duration()));
                            sink.append(decoder);
                            sink.play();
                        }
                        Command::Seek(pos) => {
                            sink.try_seek(pos);
                        }
                    },
                    Ok(None) => break,
                    Err(_err) => {
                        sender.try_send(Event::PlaybackPosition(sink.get_pos()));
                        std::thread::sleep(Duration::from_millis(20));
                    }
                }
            }
        }
    }
}
