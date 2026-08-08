use crate::device::{self, resample, AudioError};
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackOutcome {
    Played,
    Interrupted,
}

struct Queued {
    id: u64,
    owner: u64,
    resampled: Vec<f32>,
    done: Option<Sender<PlaybackOutcome>>,
}

pub struct PlaybackQueue {
    queue: VecDeque<Queued>,
    current: Option<Queued>,
    pos: usize,
    limit: usize,
}

impl PlaybackQueue {
    pub fn new(limit: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            current: None,
            pos: 0,
            limit: limit.max(1),
        }
    }

    pub fn len(&self) -> usize {
        self.queue.len() + usize::from(self.current.is_some())
    }

    pub fn is_empty(&self) -> bool {
        self.is_idle()
    }

    pub fn is_idle(&self) -> bool {
        self.current.is_none() && self.queue.is_empty()
    }

    fn clear(&mut self, outcome: PlaybackOutcome) {
        if let Some(queued) = self.current.take() {
            if let Some(done) = queued.done {
                let _ = done.send(outcome);
            }
        }
        for queued in self.queue.drain(..) {
            if let Some(done) = queued.done {
                let _ = done.send(outcome);
            }
        }
        self.pos = 0;
    }

    fn advance(&mut self) {
        self.current = self.queue.pop_front();
        self.pos = 0;
    }

    pub fn push(
        &mut self,
        id: u64,
        owner: u64,
        resampled: Vec<f32>,
        interrupt: bool,
    ) -> Result<Receiver<PlaybackOutcome>, AudioError> {
        if interrupt {
            self.clear(PlaybackOutcome::Interrupted);
        }
        if self.len() >= self.limit {
            return Err(AudioError::QueueFull);
        }
        let (tx, rx) = channel();
        self.queue.push_back(Queued {
            id,
            owner,
            resampled,
            done: Some(tx),
        });
        if self.current.is_none() {
            self.advance();
        }
        Ok(rx)
    }

    pub fn current(&self) -> Option<(u64, u64)> {
        self.current
            .as_ref()
            .map(|queued| (queued.id, queued.owner))
    }

    pub fn stop(&mut self) {
        self.clear(PlaybackOutcome::Interrupted);
    }

    pub fn interrupt_current(&mut self) {
        if let Some(current) = self.current.take() {
            if let Some(done) = current.done {
                let _ = done.send(PlaybackOutcome::Interrupted);
            }
        }
        self.pos = 0;
        if self.current.is_none() {
            self.advance();
        }
    }

    pub fn next_frame(&mut self) -> Option<f32> {
        loop {
            if let Some(current) = &mut self.current {
                if self.pos < current.resampled.len() {
                    let value = current.resampled[self.pos];
                    self.pos += 1;
                    return Some(value);
                }
                if let Some(done) = current.done.take() {
                    let _ = done.send(PlaybackOutcome::Played);
                }
                self.current = None;
                self.advance();
                continue;
            }
            return None;
        }
    }
}

pub struct PlaybackController {
    queue: Arc<Mutex<PlaybackQueue>>,
    _stream: cpal::Stream,
    device_rate: u32,
}

impl PlaybackController {
    pub fn open(device: Option<&str>, queue_limit: usize) -> Result<Self, AudioError> {
        let queue = Arc::new(Mutex::new(PlaybackQueue::new(queue_limit)));
        let callback_queue = Arc::clone(&queue);
        let (stream, config, _info) = device::open_stream(device, move |data, cfg| {
            let channels = cfg.channels as usize;
            let mut guard = callback_queue.lock();
            for frame in 0..data.len() / channels {
                let value = guard.next_frame().unwrap_or(0.0);
                for channel in 0..channels {
                    data[frame * channels + channel] = value;
                }
            }
        })?;
        let device_rate = config.sample_rate.0;
        Ok(Self {
            queue,
            _stream: stream,
            device_rate,
        })
    }

    pub fn device_rate(&self) -> u32 {
        self.device_rate
    }

    pub fn push(
        &self,
        id: u64,
        owner: u64,
        samples: Vec<f32>,
        rate: u32,
        interrupt: bool,
    ) -> Result<Receiver<PlaybackOutcome>, AudioError> {
        let resampled = resample(&samples, rate, self.device_rate)?;
        let mut guard = self.queue.lock();
        guard.push(id, owner, resampled, interrupt)
    }

    pub fn current(&self) -> Option<(u64, u64)> {
        self.queue.lock().current()
    }

    pub fn stop(&self) {
        self.queue.lock().stop();
    }

    pub fn interrupt_current(&self) {
        self.queue.lock().interrupt_current();
    }

    pub fn is_idle(&self) -> bool {
        self.queue.lock().is_idle()
    }

    pub fn queue_len(&self) -> usize {
        self.queue.lock().len()
    }
}

enum Command {
    Push {
        id: u64,
        owner: u64,
        samples: Vec<f32>,
        rate: u32,
        interrupt: bool,
        reply: std::sync::mpsc::SyncSender<Result<Receiver<PlaybackOutcome>, String>>,
    },
    Current {
        reply: std::sync::mpsc::SyncSender<Option<(u64, u64)>>,
    },
    InterruptCurrent,
    Stop,
}

pub struct PlaybackThread {
    tx: std::sync::mpsc::Sender<Command>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl PlaybackThread {
    pub fn spawn(device: Option<String>, queue_limit: usize) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<Command>();
        let handle = std::thread::Builder::new()
            .name("master-voice-playback".to_string())
            .spawn(move || {
                let mut controller = PlaybackController::open(device.as_deref(), queue_limit).ok();
                while let Ok(command) = rx.recv() {
                    match command {
                        Command::Push {
                            id,
                            owner,
                            samples,
                            rate,
                            interrupt,
                            reply,
                        } => {
                            if controller.is_none() {
                                controller =
                                    PlaybackController::open(device.as_deref(), queue_limit).ok();
                            }
                            let result = match &controller {
                                Some(controller) => controller
                                    .push(id, owner, samples, rate, interrupt)
                                    .map_err(|e| e.to_string()),
                                None => Err("no audio output device available".to_string()),
                            };
                            let _ = reply.send(result);
                        }
                        Command::Current { reply } => {
                            let current = controller.as_ref().and_then(|c| c.current());
                            let _ = reply.send(current);
                        }
                        Command::InterruptCurrent => {
                            if let Some(controller) = &controller {
                                controller.interrupt_current();
                            }
                        }
                        Command::Stop => break,
                    }
                }
                if let Some(controller) = &controller {
                    controller.stop();
                }
            })
            .expect("spawn playback thread");
        Self {
            tx,
            handle: Some(handle),
        }
    }

    pub fn push(
        &self,
        id: u64,
        owner: u64,
        samples: Vec<f32>,
        rate: u32,
        interrupt: bool,
    ) -> Result<Receiver<PlaybackOutcome>, AudioError> {
        let (reply_tx, reply_rx) = std::sync::mpsc::sync_channel(1);
        self.tx
            .send(Command::Push {
                id,
                owner,
                samples,
                rate,
                interrupt,
                reply: reply_tx,
            })
            .map_err(|_| AudioError::Stream("playback thread stopped".into()))?;
        reply_rx
            .recv()
            .map_err(|_| AudioError::Stream("playback thread stopped".into()))?
            .map_err(|message| {
                if message.contains("no audio output device") {
                    AudioError::NoDevice
                } else if message.contains("queue is full") {
                    AudioError::QueueFull
                } else {
                    AudioError::Stream(message)
                }
            })
    }

    pub fn current(&self) -> Option<(u64, u64)> {
        let (reply_tx, reply_rx) = std::sync::mpsc::sync_channel(1);
        if self.tx.send(Command::Current { reply: reply_tx }).is_err() {
            return None;
        }
        reply_rx.recv().unwrap_or(None)
    }

    pub fn interrupt_current(&self) {
        let _ = self.tx.send(Command::InterruptCurrent);
    }

    pub fn stop(&mut self) {
        let _ = self.tx.send(Command::Stop);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait_outcome(rx: Receiver<PlaybackOutcome>) -> PlaybackOutcome {
        rx.recv().unwrap_or(PlaybackOutcome::Interrupted)
    }

    #[test]
    fn plays_samples_in_order() {
        let mut queue = PlaybackQueue::new(4);
        queue.push(1, 1, vec![1.0, 2.0, 3.0], false).unwrap();
        assert_eq!(queue.next_frame(), Some(1.0));
        assert_eq!(queue.next_frame(), Some(2.0));
        assert_eq!(queue.next_frame(), Some(3.0));
        assert_eq!(queue.next_frame(), None);
        assert!(queue.is_idle());
    }

    #[test]
    fn queues_second_utterance() {
        let mut queue = PlaybackQueue::new(4);
        queue.push(1, 1, vec![1.0], false).unwrap();
        queue.push(2, 1, vec![2.0], false).unwrap();
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.next_frame(), Some(1.0));
        assert_eq!(queue.next_frame(), Some(2.0));
        assert_eq!(queue.next_frame(), None);
    }

    #[test]
    fn interrupt_replaces_and_notifies() {
        let mut queue = PlaybackQueue::new(4);
        let first = queue.push(1, 1, vec![1.0; 100], false).unwrap();
        let second = queue.push(2, 1, vec![2.0; 100], false).unwrap();
        let third = queue.push(3, 1, vec![3.0; 100], true).unwrap();
        assert_eq!(wait_outcome(first), PlaybackOutcome::Interrupted);
        assert_eq!(wait_outcome(second), PlaybackOutcome::Interrupted);
        assert_eq!(queue.next_frame(), Some(3.0));
        assert_eq!(queue.len(), 1);
        queue.stop();
        assert_eq!(wait_outcome(third), PlaybackOutcome::Interrupted);
    }

    #[test]
    fn reports_played_on_completion() {
        let mut queue = PlaybackQueue::new(4);
        let rx = queue.push(1, 1, vec![0.5; 10], false).unwrap();
        for _ in 0..10 {
            assert!(queue.next_frame().is_some());
        }
        assert_eq!(queue.next_frame(), None);
        assert_eq!(wait_outcome(rx), PlaybackOutcome::Played);
    }

    #[test]
    fn bounded_queue_rejects_overflow() {
        let mut queue = PlaybackQueue::new(2);
        queue.push(1, 1, vec![1.0; 10], false).unwrap();
        queue.push(2, 1, vec![2.0; 10], false).unwrap();
        assert!(matches!(
            queue.push(3, 1, vec![3.0; 10], false),
            Err(AudioError::QueueFull)
        ));
    }

    #[test]
    fn empty_queue_returns_silence() {
        let mut queue = PlaybackQueue::new(2);
        assert_eq!(queue.next_frame(), None);
        assert!(queue.is_idle());
    }
}
