
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BuildLogMsg {
    pub system: String,
    pub identity: String,
    pub attempt_id: String,
    pub line_number: u64,
    pub output: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BuildLogStart {
    pub system: String,
    pub identity: String,
    pub attempt_id: String,
}
