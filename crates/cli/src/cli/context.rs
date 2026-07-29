use std::collections::HashMap;
#[derive(Debug, Clone)]
pub struct CommandContext {
    pub command: String,
    pub options: HashMap<String, Option<String>>,
    pub global_options: Vec<String>,
    pub params: Vec<String>,
}

impl CommandContext {
    pub fn prm(&self, index: usize) -> Option<String> {
        self.params.get(index).cloned()
    }

    pub fn prm_count(&self) -> usize {
        self.params.len()
    }

    pub fn has_option(&self, name: &str) -> bool {
        self.options.contains_key(name)
    }

    pub fn option(&self, name: &str) -> Option<&str> {
        self.options.get(name).and_then(|v| v.as_deref())
    }
}
