#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("usage: {0}")]
    Usage(String),
    #[error("configuration: {0}")]
    Config(String),
    #[error("language: {0}")]
    Language(String),
    #[error("synthesis: {0}")]
    Synthesis(String),
    #[error("audio: {0}")]
    Audio(String),
    #[error("queue full: {0}")]
    QueueFull(String),
    #[error("daemon: {0}")]
    Daemon(String),
}

impl Error {
    pub fn exit_code(&self) -> i32 {
        match self {
            Error::Usage(_) => 1,
            Error::Config(_) => 2,
            Error::Language(_) => 3,
            Error::Synthesis(_) => 4,
            Error::Audio(_) => 5,
            Error::QueueFull(_) => 7,
            Error::Daemon(_) => 6,
        }
    }
}

impl From<master_voice_linguistics::LingError> for Error {
    fn from(value: master_voice_linguistics::LingError) -> Self {
        Error::Language(value.to_string())
    }
}

impl From<master_voice_audio::AudioError> for Error {
    fn from(value: master_voice_audio::AudioError) -> Self {
        Error::Audio(value.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Daemon(value.to_string())
    }
}
