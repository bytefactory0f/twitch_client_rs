use thiserror::Error;

#[derive(Debug, Error)]
pub enum MessageParseError {
    //// Parsing
    #[error("no value for tag: {0}")]
    MissingTag(String),

    #[error("tag is invalid: {0}")]
    InvalidTag(String),

    #[error("badge is invalid: {0}")]
    InvalidBadge(String),

    #[error("badge version is invalid: {0}")]
    InvalidBadgeVersion(String),

    #[error("value for tag {0} could not be converted to boolean: {1}")]
    InvalidBoolValue(String, String),

    #[error("value for tag {0} could not be converted to u32: {1}")]
    InvalidIntValue(String, String),

    #[error("emote is invalid: {0}")]
    MalformedEmote(String),

    #[error("user type {0} is invalid")]
    InvalidUserType(String),
}

#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("websocket is not connected")]
    WebsocketNotConnected,

    #[error("error opening websocket connection: {0}")]
    WebsocketConnectionError(tokio_tungstenite::tungstenite::Error),

    #[error("error sending message: {0}")]
    SendMessageFailure(tokio_tungstenite::tungstenite::Error),

    #[error("error receiving message: {0}")]
    ReceiveMessageFailure(tokio_tungstenite::tungstenite::Error),
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("connection error: {0}")]
    ConnectionError(ConnectionError),

    #[error("error handling twitch message: {0}")]
    MessageParseError(MessageParseError),

    #[error("error refreshing access token: {0}")]
    RefreshAccessTokenError(reqwest::Error),
}

impl From<ConnectionError> for Error {
    fn from(value: ConnectionError) -> Self {
        Self::ConnectionError(value)
    }
}
