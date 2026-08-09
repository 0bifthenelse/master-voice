use crate::device::{self, resample, AudioError};
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

const DECLICK_SAMPLES: usize = 128;

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
    last_sample: Option<f32>,
    last_utterance: Option<u64>,
    ramp_out_start: Option<f32>,
    ramp_out_pos: usize,
    ramp_in_pos: Option<usize>,
}

impl PlaybackQueue {
    pub fn new(limit: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            current: None,
            pos: 0,
            limit: limit.max(1),
            last_sample: None,
            last_utterance: None,
            ramp_out_start: None,
            ramp_out_pos: 0,
            ramp_in_pos: None,
        }
    }

    pub fn len(&self) -> usize {
        self.queue.len() + usize::from(self.current.is_some())
    }

    pub fn is_empty(&self) -> bool {
        self.is_idle()
    }

    pub fn is_idle(&self) -> bool {
        self.current.is_none() && self.queue.is_empty() && self.ramp_out_start.is_none()
    }

    fn begin_ramp_out(&mut self) {
        if self.current.is_some() {
            if let Some(sample) = self.last_sample.filter(|sample| sample.is_finite()) {
                self.ramp_out_start = Some(sample);
                self.ramp_out_pos = 0;
            }
        }
    }

    fn clear(&mut self, outcome: PlaybackOutcome) {
        self.begin_ramp_out();
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
        self.ramp_in_pos = None;
        self.last_utterance = None;
    }

    fn advance(&mut self) {
        self.current = self.queue.pop_front();
        self.pos = 0;
        if let Some(current) = &self.current {
            if self.last_utterance != Some(current.id) {
                self.ramp_in_pos = Some(0);
            }
            self.last_utterance = Some(current.id);
        }
    }

    pub fn push(
        &mut self,
        id: u64,
        owner: u64,
        resampled: Vec<f32>,
        interrupt: bool,
    ) -> Result<Receiver<PlaybackOutcome>, AudioError> {
        device::validate_normalized_pcm(&resampled)?;
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
        self.begin_ramp_out();
        if let Some(current) = self.current.take() {
            if let Some(done) = current.done {
                let _ = done.send(PlaybackOutcome::Interrupted);
            }
        }
        self.pos = 0;
        self.ramp_in_pos = None;
        self.last_utterance = None;
        self.advance();
    }

    /// Interrupt the current item and every queued item of this utterance
    /// (an utterance is many queue items once streaming). Other utterances
    /// stay queued; their FIFO position is preserved.
    pub fn interrupt_utterance(&mut self, id: u64) {
        if self.current.as_ref().is_some_and(|q| q.id == id) {
            self.begin_ramp_out();
            if let Some(current) = self.current.take() {
                if let Some(done) = current.done {
                    let _ = done.send(PlaybackOutcome::Interrupted);
                }
            }
            self.pos = 0;
            self.ramp_in_pos = None;
            self.last_utterance = None;
        }
        let mut kept = VecDeque::new();
        for mut queued in self.queue.drain(..) {
            if queued.id == id {
                if let Some(done) = queued.done.take() {
                    let _ = done.send(PlaybackOutcome::Interrupted);
                }
            } else {
                kept.push_back(queued);
            }
        }
        self.queue = kept;
        if self.current.is_none() {
            self.advance();
        }
    }

    pub fn next_frame(&mut self) -> Option<f32> {
        if let Some(start) = self.ramp_out_start {
            let gain = 1.0 - self.ramp_out_pos as f32 / (DECLICK_SAMPLES.saturating_sub(1)) as f32;
            let value = start * gain;
            self.ramp_out_pos += 1;
            if self.ramp_out_pos == DECLICK_SAMPLES {
                self.ramp_out_start = None;
                self.ramp_out_pos = 0;
                self.last_sample = self.current.as_ref().map(|_| 0.0);
            }
            return Some(value);
        }

        loop {
            if let Some(current) = &mut self.current {
                if self.pos < current.resampled.len() {
                    let mut value = current.resampled[self.pos];
                    self.pos += 1;
                    if let Some(ramp_pos) = self.ramp_in_pos {
                        let gain = ramp_pos as f32 / (DECLICK_SAMPLES.saturating_sub(1)) as f32;
                        value *= gain;
                        let next = ramp_pos + 1;
                        self.ramp_in_pos = (next < DECLICK_SAMPLES).then_some(next);
                    }
                    self.last_sample = Some(value);
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

    pub fn interrupt_utterance(&self, id: u64) {
        self.queue.lock().interrupt_utterance(id);
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
        reply: std::sync::mpsc::SyncSender<Result<Receiver<PlaybackOutcome>, AudioError>>,
    },
    Current {
        reply: std::sync::mpsc::SyncSender<Option<(u64, u64)>>,
    },
    InterruptCurrent,
    InterruptUtterance {
        id: u64,
    },
    QueueLen {
        reply: std::sync::mpsc::SyncSender<usize>,
    },
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
                                Some(controller) => {
                                    controller.push(id, owner, samples, rate, interrupt)
                                }
                                None => Err(AudioError::NoDevice),
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
                        Command::InterruptUtterance { id } => {
                            if let Some(controller) = &controller {
                                controller.interrupt_utterance(id);
                            }
                        }
                        Command::QueueLen { reply } => {
                            let len = controller.as_ref().map(|c| c.queue_len()).unwrap_or(0);
                            let _ = reply.send(len);
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

    pub fn interrupt_utterance(&self, id: u64) {
        let _ = self.tx.send(Command::InterruptUtterance { id });
    }

    /// Number of items in the queue (current + queued).
    pub fn queue_len(&self) -> usize {
        let (reply_tx, reply_rx) = std::sync::mpsc::sync_channel(1);
        if self.tx.send(Command::QueueLen { reply: reply_tx }).is_err() {
            return 0;
        }
        reply_rx.recv().unwrap_or(0)
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
    fn new_utterance_ramps_in_and_preserves_sample_order() {
        let mut queue = PlaybackQueue::new(4);
        queue.push(1, 1, vec![0.5; DECLICK_SAMPLES], false).unwrap();
        let samples: Vec<f32> = (0..DECLICK_SAMPLES)
            .map(|_| queue.next_frame().unwrap())
            .collect();
        assert_eq!(samples[0], 0.0);
        assert_eq!(samples[DECLICK_SAMPLES - 1], 0.5);
        assert!(samples.windows(2).all(|pair| pair[0] <= pair[1]));
        assert_eq!(queue.next_frame(), None);
        assert!(queue.is_idle());
    }

    #[test]
    fn successive_items_with_same_utterance_do_not_ramp_again() {
        let mut queue = PlaybackQueue::new(4);
        queue
            .push(1, 1, vec![0.25; DECLICK_SAMPLES], false)
            .unwrap();
        queue.push(1, 1, vec![0.5], false).unwrap();
        for _ in 0..DECLICK_SAMPLES {
            queue.next_frame().unwrap();
        }
        assert_eq!(queue.next_frame(), Some(0.5));
        assert_eq!(queue.next_frame(), None);
    }

    #[test]
    fn interrupt_replaces_with_declicked_transition_and_notifies() {
        let mut queue = PlaybackQueue::new(4);
        let first = queue.push(1, 1, vec![0.8; 256], false).unwrap();
        for _ in 0..DECLICK_SAMPLES {
            queue.next_frame().unwrap();
        }
        let second = queue.push(2, 1, vec![0.2; 100], false).unwrap();
        let third = queue.push(3, 1, vec![0.3; 256], true).unwrap();
        assert_eq!(wait_outcome(first), PlaybackOutcome::Interrupted);
        assert_eq!(wait_outcome(second), PlaybackOutcome::Interrupted);
        let ramp: Vec<f32> = (0..DECLICK_SAMPLES)
            .map(|_| queue.next_frame().unwrap())
            .collect();
        assert_eq!(ramp[0], 0.8);
        assert_eq!(ramp[DECLICK_SAMPLES - 1], 0.0);
        assert!(ramp.windows(2).all(|pair| pair[0] >= pair[1]));
        assert_eq!(queue.next_frame(), Some(0.0));
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
        queue.push(1, 1, vec![0.1; 10], false).unwrap();
        queue.push(2, 1, vec![0.2; 10], false).unwrap();
        assert!(matches!(
            queue.push(3, 1, vec![0.3; 10], false),
            Err(AudioError::QueueFull)
        ));
    }

    #[test]
    fn interrupt_utterance_kills_only_its_own_items() {
        let mut queue = PlaybackQueue::new(8);
        let a = queue.push(7, 1, vec![0.1; 10], false).unwrap();
        let b = queue.push(7, 1, vec![0.2; 10], false).unwrap();
        let c = queue.push(7, 1, vec![0.3; 10], false).unwrap();
        let d = queue.push(8, 1, vec![0.4; 10], false).unwrap();
        queue.interrupt_utterance(7);
        assert_eq!(wait_outcome(a), PlaybackOutcome::Interrupted);
        assert_eq!(wait_outcome(b), PlaybackOutcome::Interrupted);
        assert_eq!(wait_outcome(c), PlaybackOutcome::Interrupted);
        assert_eq!(queue.len(), 1, "only the id-8 item survives");
        assert_eq!(queue.next_frame(), Some(0.0));
        queue.stop();
        assert_eq!(wait_outcome(d), PlaybackOutcome::Interrupted);
    }

    #[test]
    fn invalid_interrupting_push_does_not_mutate_queue() {
        let mut queue = PlaybackQueue::new(4);
        let first = queue.push(1, 1, vec![0.25; 4], false).unwrap();
        assert!(matches!(
            queue.push(2, 1, vec![0.0, f32::NAN], true),
            Err(AudioError::InvalidSamples { index: 1 })
        ));
        assert_eq!(queue.current(), Some((1, 1)));
        assert_eq!(queue.next_frame(), Some(0.0));
        queue.stop();
        assert_eq!(wait_outcome(first), PlaybackOutcome::Interrupted);
    }

    #[test]
    fn stop_emits_fixed_ramp_to_zero() {
        let mut queue = PlaybackQueue::new(2);
        let outcome = queue.push(1, 1, vec![0.75; 256], false).unwrap();
        for _ in 0..DECLICK_SAMPLES {
            queue.next_frame().unwrap();
        }
        queue.stop();
        assert_eq!(wait_outcome(outcome), PlaybackOutcome::Interrupted);
        let ramp: Vec<f32> = (0..DECLICK_SAMPLES)
            .map(|_| queue.next_frame().unwrap())
            .collect();
        assert_eq!(ramp[0], 0.75);
        assert_eq!(ramp[DECLICK_SAMPLES - 1], 0.0);
        assert!(ramp.windows(2).all(|pair| pair[0] >= pair[1]));
        assert_eq!(queue.next_frame(), None);
        assert!(queue.is_idle());
    }

    #[test]
    fn empty_queue_returns_silence() {
        let mut queue = PlaybackQueue::new(2);
        assert_eq!(queue.next_frame(), None);
        assert!(queue.is_idle());
    }
}
