use crossterm::event::KeyEvent;

use super::Action;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InputEvent {
    Key(KeyEvent),
    Resize { columns: u16, rows: u16 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppEvent {
    Input(InputEvent),
    Action(Box<Action>),
    VideoTick,
}
