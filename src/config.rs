use url::Url;

/// Holds the available settings for a twitch chat client
#[derive(Clone, Debug, Builder)]
pub struct ClientConfig {
    /// The chat server, by default wss://irc-ws.chat.twitch.tv:443
    #[builder(default = r#"Url::parse("wss://irc-ws.chat.twitch.tv:443").unwrap()"#)]
    pub url: Url,

    /// Twitch username to use
    pub username: String,

    /// OAuth token to use
    pub token: String,

    /// Whether to enable membership capability (default: false)
    #[builder(default = "false")]
    pub cap_membership: bool,

    /// Whether to enable commands capability (default: true)
    #[builder(default = "true")]
    pub cap_commands: bool,

    /// Whether to enable tags capability (default: true)
    #[builder(default = "true")]
    pub cap_tags: bool,
}
