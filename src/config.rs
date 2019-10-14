use url::Url;

#[derive(Clone, Debug, Builder)]
pub struct ClientConfig {
    #[builder(default = r#"Url::parse("wss://irc-ws.chat.twitch.tv:443").unwrap()"#)]
    pub url: Url,

    pub username: String,

    pub token: String,

    #[builder(default = "false")]
    pub cap_membership: bool,

    #[builder(default = "false")]
    pub cap_commands: bool,

    #[builder(default = "true")]
    pub cap_tags: bool,
}
