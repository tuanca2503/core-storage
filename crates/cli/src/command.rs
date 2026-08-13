use std::collections::HashMap;
#[derive(Debug, Clone)]
pub struct Context {
    pub command: String,
    pub options: HashMap<String, Option<String>>,
    pub global_options: Vec<String>,
    pub params: Vec<String>,
}

impl Context {
    pub fn prm(&self, index: usize) -> Option<String> {
        self.params.get(index).cloned()
    }

    pub fn prm_count(&self) -> usize {
        self.params.len()
    }

    pub fn has_option(&self, name: &str) -> bool {
        self.options.contains_key(name)
    }

    pub fn option(&self, name: &str) -> Option<String> {
        self.options
            .get(name)
            .and_then(|v| v.clone())
    }
}

pub struct Command {
    pub name: &'static str,
    pub min_arguments: usize,
    pub max_arguments: usize,
    pub flags: &'static [(&'static str, &'static str)],
    pub handler: fn(Context) -> Result,
}

pub enum Result {
    Text(String),
    Table(Vec<Vec<String>>),
    Error(String),
}
