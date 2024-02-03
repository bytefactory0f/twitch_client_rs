use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;
use std::error::Error;

lazy_static! {
    // Parses twitch messages into four components:
    // @<tags> :<source(<nick>!<host)> <command> :<parameters>
    // "@tag=0;tag1=;tag2=0,300374282 :azrajah!azrajah@azrajah.tmi.twitch.tv PRIVMSG #channel :hello world"
    static ref MESSAGE_PATTERN: Regex = Regex::new(r"^(?:(?<tags>@\S+)\s)?(?:\:(?<source>\S+)\s)?(?<command>[^:]+[^:\s])(?:\s\:(?<parameters>.*))?$").unwrap();
}

#[derive(Debug, PartialEq, Eq)]
pub struct Source {
    nick: Option<String>,
    host: String,
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
        tags: Option<HashMap<String, String>>,
        is_mod: bool,
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

fn parse_tags(tags_component: &str) -> HashMap<String, String> {
    tags_component
        .split(';')
        .map(|kv| {
            let mut parts = kv.split('=');
            (
                parts.next().unwrap().to_string(),
                parts.next().unwrap_or("").to_string(),
            )
        })
        .collect::<HashMap<String, String>>()
}

pub fn parse_message(line: &str) -> Result<IRCMessage, Box<dyn Error>> {
    let caps = MESSAGE_PATTERN.captures(line).unwrap();

    let tags = caps.name("tags").map(|m| parse_tags(m.as_str()));
    let source = caps.name("source").map(|m| parse_source(m.as_str()));
    let command = caps.name("command").unwrap().as_str();
    let parameters = caps.name("parameters").map(|m| m.as_str());

    if command.starts_with("PING") {
        let ping_message = parameters.unwrap();
        return Ok(IRCMessage::Ping(ping_message.to_string()));
    } else if command.starts_with("PRIVMSG") {
        let is_mod = match tags {
            Some(ref tags) => match tags.get("mod") {
                Some(value) => value == "1",
                None => false,
            },
            None => false,
        };

        return Ok(IRCMessage::Privmsg {
            message: parameters.unwrap().to_string(),
            source: source.unwrap(),
            is_mod,
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
    fn test_parse_tag() {
        let expected: HashMap<String, String> = HashMap::from([
            ("badge-info".to_string(), "".to_string()),
            ("badges".to_string(), "".to_string()),
            ("color".to_string(), "".to_string()),
            ("display-name".to_string(), "<user>".to_string()),
            ("emote-sets".to_string(), "0,300374282".to_string()),
            ("user-id".to_string(), "12345678".to_string()),
            ("user-type".to_string(), "".to_string()),
        ]);

        let actual = parse_tags(
            "badge-info=;badges=;color=;display-name=<user>;emote-sets=0,300374282;user-id=12345678;user-type="
        );

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_source_tag() {
        let expected = Source {
            nick: Some(String::from("bytebot_0x0f")),
            host: String::from("bytebot_0x0f@bytebot_0x0f.tmi.twitch.tv"),
        };

        let actual = parse_source("bytebot_0x0f!bytebot_0x0f@bytebot_0x0f.tmi.twitch.tv");

        assert_eq!(expected, actual);
    }
}
