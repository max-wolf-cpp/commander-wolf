#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CommandId(pub String);

impl From<&str> for CommandId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandInvocation {
    pub id: CommandId,
    pub arguments: Vec<String>,
    pub count: Option<usize>,
}

impl CommandInvocation {
    pub fn new(id: CommandId, arguments: Vec<String>, count: Option<usize>) -> Self {
        Self {
            id,
            arguments,
            count: count.map(|value| value.max(1)),
        }
    }
}
