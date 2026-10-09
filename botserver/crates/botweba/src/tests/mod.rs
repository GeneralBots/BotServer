pub(crate) use super::*;
pub(crate) use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BrowserType {
    #[default]
    Chrome,
    Firefox,
}

#[derive(Debug, Clone)]
pub struct BrowserConfig {
    pub browser_type: BrowserType,
    pub debug_port: u16,
    pub headless: bool,
    pub window_width: u32,
    pub window_height: u32,
    pub timeout: Duration,
}

#[derive(Debug, Clone)]
pub enum Locator {
    Css(String),
    XPath(String),
    Id(String),
}

#[derive(Debug, Clone)]
pub enum Action {
    Click(Locator),
    SendKeys(String),
    Pause(Duration),
}

pub struct ActionChain {
    actions: Vec<Action>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub secure: Option<bool>,
    pub http_only: Option<bool>,
}

pub struct LoginPage {
    base_url: String,
}

pub struct DashboardPage {
    base_url: String,
}

pub struct ChatPage {
    base_url: String,
    bot_name: String,
}

pub struct QueuePage {
    base_url: String,
}

pub struct BotManagementPage {
    base_url: String,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;
mod part8;
mod part9;
mod part10;
