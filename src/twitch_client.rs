use futures_util::{SinkExt, StreamExt};
use std::collections::VecDeque;
use std::error::Error;
use std::fmt;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};

use crate::auth;
use crate::credentials::Credentials;
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
    message_buffer: VecDeque<Result<IRCMessage, Box<dyn Error>>>,
    ws_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl TwitchClient {
    pub fn new(credentials: Credentials, nick: String) -> Result<Self, Box<dyn Error>> {
        let twitch_client = TwitchClient {
            nick,
            credentials,
            message_buffer: VecDeque::new(),
            access_token: String::new(),
            url: url::Url::parse("wss://irc-ws.chat.twitch.tv:443").unwrap(),
            ws_stream: None,
        };

        Ok(twitch_client)
    }

    pub async fn update_access_token(&mut self) -> Result<(), Box<dyn Error>> {
        self.access_token = auth::refresh_access_token(&self.credentials).await?;

        Ok(())
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        let (ws_stream, _) = connect_async(&self.url).await?;
        println!("WebSocket handshake has been successfully completed");

        self.ws_stream = Some(ws_stream);
        Ok(())
    }

    async fn send(&mut self, message: Message) -> Result<(), Box<dyn Error>> {
        self.ws_stream
            .as_mut()
            .ok_or("Websocket is not connected")?
            .send(message)
            .await?;
        Ok(())
    }

    pub async fn authenticate(&mut self) -> Result<(), Box<dyn Error>> {
        self.pass().await?;
        self.nick().await?;
        Ok(())
    }

    pub async fn cap_req(&mut self, capabilities: &[Capability]) -> Result<(), Box<dyn Error>> {
        let cap_str = capabilities
            .iter()
            .map(Capability::to_string)
            .collect::<Vec<_>>()
            .join(" ");
        self.send(Message::Text(format!("CAP REQ :{}", cap_str)))
            .await?;
        Ok(())
    }

    pub async fn pass(&mut self) -> Result<(), Box<dyn Error>> {
        self.send(Message::Text(format!("PASS oauth:{}", self.access_token)))
            .await?;
        Ok(())
    }

    pub async fn nick(&mut self) -> Result<(), Box<dyn Error>> {
        self.send(Message::Text(format!("NICK {}", self.nick)))
            .await?;
        Ok(())
    }

    pub async fn join(&mut self, channel_name: &str) -> Result<(), Box<dyn Error>> {
        self.send(Message::Text(format!("JOIN #{channel_name}")))
            .await?;
        Ok(())
    }

    pub async fn pong(&mut self, ping_message: &str) -> Result<(), Box<dyn Error>> {
        self.send(Message::Text(format!("PONG :{ping_message}")))
            .await?;
        Ok(())
    }

    pub async fn part(&mut self, channel_name: &str) -> Result<(), Box<dyn Error>> {
        self.send(Message::Text(format!("PART #{channel_name}")))
            .await?;
        Ok(())
    }

    pub async fn privmsg(
        &mut self,
        channel_name: &str,
        message: &str,
    ) -> Result<(), Box<dyn Error>> {
        self.send(Message::Text(format!("PRIVMSG #{channel_name} :{message}")))
            .await?;
        Ok(())
    }

    // Return Option<Result<..., Error>>
    pub async fn next(&mut self) -> Option<Result<IRCMessage, Box<dyn Error>>> {
        if !self.message_buffer.is_empty() {
            return self.message_buffer.pop_front();
        }

        let stream = match self.ws_stream.as_mut().ok_or("Not connected") {
            Ok(s) => s,
            Err(e) => {
                panic!("Not connected");
            }
        };

        match stream.next().await? {
            Ok(message) => {
                if message.is_text() {
                    let text = match message.to_text() {
                        Ok(t) => t,
                        Err(_) => panic!("Error reading text message"),
                    };

                    for line in text.lines() {
                        self.message_buffer.push_back(irc::parse_message(line));
                    }

                    return self.message_buffer.pop_front();
                }

                None // Should return Some(Err())
            }
            Err(e) => panic!("Error receiving from the websocket"),
        }
    }
}
