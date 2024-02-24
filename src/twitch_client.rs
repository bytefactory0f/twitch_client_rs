use futures_util::{SinkExt, StreamExt};
use std::collections::VecDeque;
use std::fmt;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};

use crate::auth;
use crate::credentials::Credentials;
use crate::error::{ConnectionError, Error, MessageParseError};
use crate::irc;
use crate::irc::IRCMessage;

// Defines extra capabilies for the chat bot
pub enum Capability {
    Commands,
    Memberships,
    Tags,
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Commands => write!(f, "twitch.tv/commands"),
            Self::Memberships => write!(f, "twitch.tv/membership"),
            Self::Tags => write!(f, "twitch.tv/tags"),
        }
    }
}

pub struct TwitchClient {
    credentials: Credentials,
    // Stores the access token retrieved from Credentials
    access_token: String,
    nick: String,
    url: url::Url,
    message_buffer: VecDeque<Result<IRCMessage, MessageParseError>>,
    ws_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    auto_pong: bool,
}

impl TwitchClient {
    pub fn new(credentials: Credentials, nick: String, auto_pong: bool) -> Self {
        TwitchClient {
            nick,
            credentials,
            message_buffer: VecDeque::new(),
            access_token: String::new(),
            url: url::Url::parse("wss://irc-ws.chat.twitch.tv:443").unwrap(),
            ws_stream: None,
            auto_pong,
        }
    }

    pub async fn update_access_token(&mut self) -> Result<(), Error> {
        self.access_token = auth::refresh_access_token(&self.credentials)
            .await
            .map_err(Error::RefreshAccessTokenError)?;
        Ok(())
    }

    pub async fn connect(&mut self) -> Result<(), Error> {
        let (ws_stream, _) = connect_async(&self.url)
            .await
            .map_err(ConnectionError::WebsocketConnectionError)?;

        self.ws_stream = Some(ws_stream);
        Ok(())
    }

    async fn send(&mut self, message: Message) -> Result<(), Error> {
        self.ws_stream
            .as_mut()
            .ok_or(ConnectionError::WebsocketNotConnected)?
            .send(message)
            .await
            .map_err(ConnectionError::SendMessageFailure)?;
        Ok(())
    }

    pub async fn authenticate(&mut self) -> Result<(), Error> {
        self.pass().await?;
        self.nick().await?;
        Ok(())
    }

    pub async fn cap_req(&mut self, capabilities: &[Capability]) -> Result<(), Error> {
        let cap_str = capabilities
            .iter()
            .map(Capability::to_string)
            .collect::<Vec<_>>()
            .join(" ");
        self.send(Message::Text(format!("CAP REQ :{}", cap_str)))
            .await?;
        Ok(())
    }

    pub async fn pass(&mut self) -> Result<(), Error> {
        self.send(Message::Text(format!("PASS oauth:{}", self.access_token)))
            .await?;
        Ok(())
    }

    pub async fn nick(&mut self) -> Result<(), Error> {
        self.send(Message::Text(format!("NICK {}", self.nick)))
            .await?;
        Ok(())
    }

    pub async fn join(&mut self, channel_name: &str) -> Result<(), Error> {
        self.send(Message::Text(format!("JOIN #{channel_name}")))
            .await?;
        Ok(())
    }

    pub async fn pong(&mut self, ping_message: &str) -> Result<(), Error> {
        self.send(Message::Text(format!("PONG :{ping_message}")))
            .await?;
        Ok(())
    }

    pub async fn part(&mut self, channel_name: &str) -> Result<(), Error> {
        self.send(Message::Text(format!("PART #{channel_name}")))
            .await?;
        Ok(())
    }

    pub async fn privmsg(&mut self, channel_name: &str, message: &str) -> Result<(), Error> {
        self.send(Message::Text(format!("PRIVMSG #{channel_name} :{message}")))
            .await?;
        Ok(())
    }

    async fn get_next_message(&mut self) -> Option<Result<IRCMessage, Error>> {
        if !self.message_buffer.is_empty() {
            return Some(
                self.message_buffer
                    .pop_front()?
                    .map_err(Error::MessageParseError),
            );
        }

        let stream = match self
            .ws_stream
            .as_mut()
            .ok_or(ConnectionError::WebsocketNotConnected)
        {
            Ok(s) => s,
            Err(e) => return Some(Err(Error::from(e))),
        };

        match stream
            .next()
            .await?
            .map_err(ConnectionError::ReceiveMessageFailure)
        {
            Ok(message) => {
                if message.is_text() {
                    let text = match message
                        .to_text()
                        .map_err(ConnectionError::ReceiveMessageFailure)
                    {
                        Ok(t) => t,
                        Err(e) => return Some(Err(Error::from(e))),
                    };

                    for line in text.lines() {
                        self.message_buffer.push_back(irc::parse_message(line));
                    }

                    return Some(
                        self.message_buffer
                            .pop_front()?
                            .map_err(Error::MessageParseError),
                    );
                }

                None // Should return Some(Err())
            }
            Err(e) => Some(Err(Error::from(e))),
        }
    }

    pub async fn next(&mut self) -> Option<Result<IRCMessage, Error>> {
        loop {
            let message = self.get_next_message().await?;

            if self.auto_pong {
                if let Ok(IRCMessage::Ping(msg)) = message {
                    if let Err(e) = self.pong(msg.as_str()).await {
                        return Some(Err(e));
                    }
                    continue;
                }
            }

            return Some(message);
        }
    }
}
