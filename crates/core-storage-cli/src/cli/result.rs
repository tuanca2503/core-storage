pub enum CommandResult {
    Text(String),
    Table(Vec<Vec<String>>),
    List(Vec<String>),
    Error(String),
}