#[derive(Clone)]
pub struct DBOption {
    pub dir: String,
    pub db_file_name: String,
    pub replication: ReplicationOption,
    pub port: u16,
}

#[derive(Clone)]
pub struct ReplicationOption {
    pub role: String,
    pub master_replid: String,
    pub master_repl_offset: u64,
    pub replica_of: Option<String>,
}
