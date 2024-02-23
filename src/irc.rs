use crate::error::MessageParseError;
use crate::tags;
use crate::tags::{Badge, Tags, UserType};
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;

lazy_static! {
    // Parses twitch messages into four components:
    // @<tags> :<source(<nick>!<host)> <command> :<parameters>
    // "@tag=0;tag1=;tag2=0,300374282 :azrajah!azrajah@azrajah.tmi.twitch.tv PRIVMSG #channel :hello world"
    static ref MESSAGE_PATTERN: Regex = Regex::new(r"^(?:@(?<tags>\S+)\s)?(?:\:(?<source>\S+)\s)?(?<command>[^:]+[^:\s])(?:\s\:(?<parameters>.*))?$").unwrap();
}

#[derive(Debug, PartialEq, Eq)]
pub struct Source {
    nick: Option<String>,
    host: String,
}

#[derive(Debug)]
pub struct UserContext {
    pub username: String,
    pub user_type: UserType,
    pub is_mod: bool,
    pub is_returning_chatter: bool,
    pub is_first_message: bool,
    pub is_broadcaster: bool,
    pub is_subscriber: bool,
    pub is_turbo: bool,
    pub user_id: String,
    pub badges: Vec<Badge>,
}

#[derive(Debug)]
pub enum IRCMessage {
    Ping(String),
    Notice {
        source: Source,
        message: String,
    },
    Part {
        source: Source,
        message: String,
    },
    Privmsg {
        tags: HashMap<String, String>,
        user_context: UserContext,
        source: Source,
        message: String,
    },
    Numbered {
        number: u32,
        source: Source,
        message: String,
    },
    Unknown {
        command: String,
    },
}

fn parse_source(source_component: &str) -> Source {
    if source_component.contains('!') {
        let mut parts = source_component.split('!');
        let nick = parts.next().unwrap().to_string();
        let host = parts.next().unwrap().to_string();
        return Source {
            nick: Some(nick),
            host,
        };
    }

    Source {
        nick: None,
        host: source_component.to_string(),
    }
}

pub fn parse_message(line: &str) -> Result<IRCMessage, MessageParseError> {
    let caps = MESSAGE_PATTERN.captures(line).unwrap();

    let tags = caps
        .name("tags")
        .map(|m| tags::parse_tags(m.as_str()))
        .transpose()?
        .unwrap_or(HashMap::new());
    let source = caps.name("source").map(|m| parse_source(m.as_str()));
    let command = caps.name("command").unwrap().as_str();
    let parameters = caps.name("parameters").map(|m| m.as_str());

    if command.starts_with("PING") {
        let ping_message = parameters.unwrap();
        return Ok(IRCMessage::Ping(ping_message.to_string()));
    } else if command.starts_with("PRIVMSG") {
        let badges = tags.try_get_badges()?;

        let user_context = UserContext {
            username: tags
                .get("display-name")
                .ok_or(MessageParseError::MissingTag("display-name".to_owned()))?
                .to_owned(),
            user_id: tags
                .get("user-id")
                .ok_or(MessageParseError::MissingTag("user-id".to_owned()))?
                .to_owned(),
            user_type: tags.try_get_user_type()?,
            is_turbo: tags.try_get_bool("turbo")?,
            is_subscriber: tags.try_get_bool("subscriber")?,
            is_mod: tags.try_get_bool("mod")?,
            is_first_message: tags.try_get_bool("first-msg")?,
            is_returning_chatter: tags.try_get_bool("returning-chatter")?,
            is_broadcaster: badges.iter().any(|b| matches!(b, Badge::Broadcaster(_))),
            badges,
        };

        return Ok(IRCMessage::Privmsg {
            message: parameters.unwrap().to_string(),
            source: source.unwrap(),
            user_context,
            tags,
        });
    } else if command.starts_with("NOTICE") {
        return Ok(IRCMessage::Notice {
            message: parameters.unwrap().to_string(),
            source: source.unwrap(),
        });
    } else if command.starts_with("PART") {
        return Ok(IRCMessage::Part {
            message: parameters.unwrap().to_string(),
            source: source.unwrap(),
        });
    } else if let Ok(number) = command.split(' ').next().unwrap_or("").parse::<u32>() {
        return Ok(IRCMessage::Numbered {
            number,
            message: parameters.unwrap().to_string(),
            source: source.unwrap(),
        });
    }

    Ok(IRCMessage::Unknown {
        command: command.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_source() {
        let expected = Source {
            nick: Some(String::from("bytebot_0x0f")),
            host: String::from("bytebot_0x0f@bytebot_0x0f.tmi.twitch.tv"),
        };

        let actual = parse_source("bytebot_0x0f!bytebot_0x0f@bytebot_0x0f.tmi.twitch.tv");

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_is_not_mod() {
        let actual = parse_message(
            "@user-type=;user-id=1;badges=;mod=0;returning-chatter=0;first-msg=1;turbo=0;subscriber=0;display-name=abc \
            :abc!abc@abc.tmi.twitch.tv PRIVMSG #xyz :HeyGuys"
        )
            .unwrap();

        assert!(matches!(actual, IRCMessage::Privmsg { .. }));
        if let IRCMessage::Privmsg { user_context, .. } = actual {
            assert_eq!(user_context.is_mod, false);
        }
    }

    #[test]
    fn test_is_mod() {
        let actual = parse_message(
            "@user-type=;user-id=1;badges=broadcaster/1;mod=1;returning-chatter=0;first-msg=1;turbo=0;subscriber=0;display-name=abc \
            :abc!abc@abc.tmi.twitch.tv PRIVMSG #xyz :HeyGuys"
        )
            .unwrap();

        assert!(matches!(actual, IRCMessage::Privmsg { .. }));
        if let IRCMessage::Privmsg { user_context, .. } = actual {
            assert_eq!(user_context.is_mod, true);
        }
    }

    #[test]
    fn test_is_not_broadcaster() {
        let actual = parse_message(
            "@user-type=;user-id=1;badges=;mod=0;returning-chatter=1;first-msg=0;turbo=0;subscriber=0;display-name=abc \
            :abc!abc@abc.tmi.twitch.tv PRIVMSG #xyz :HeyGuys"
        )
            .unwrap();

        assert!(matches!(actual, IRCMessage::Privmsg { .. }));
        if let IRCMessage::Privmsg { user_context, .. } = actual {
            assert_eq!(user_context.is_broadcaster, false);
        }
    }

    #[test]
    fn test_is_broadcaster() {
        let actual = parse_message(
            "@user-type=;user-id=1;badges=broadcaster/1;mod=0;returning-chatter=1;first-msg=0;turbo=0;subscriber=0;display-name=abc \
            :abc!abc@abc.tmi.twitch.tv PRIVMSG #xyz :HeyGuys"
        )
            .unwrap();

        assert!(matches!(actual, IRCMessage::Privmsg { .. }));
        if let IRCMessage::Privmsg { user_context, .. } = actual {
            assert_eq!(user_context.is_broadcaster, true);
        }
    }

    #[test]
    fn test_user_details() {
        let actual = parse_message(
            "@user-type=;user-id=1;badges=;mod=0;returning-chatter=1;first-msg=0;turbo=1;subscriber=1;display-name=abc \
            :abc!abc@abc.tmi.twitch.tv PRIVMSG #xyz :HeyGuys"
        )
            .unwrap();

        assert!(matches!(actual, IRCMessage::Privmsg { .. }));
        if let IRCMessage::Privmsg { user_context, .. } = actual {
            assert_eq!(user_context.username, "abc".to_string());
            assert_eq!(user_context.user_id, "1".to_string());
            assert_eq!(user_context.is_returning_chatter, true);
            assert_eq!(user_context.is_first_message, false);
            assert_eq!(user_context.is_turbo, true);
            assert_eq!(user_context.is_subscriber, true);
        }
    }
}
