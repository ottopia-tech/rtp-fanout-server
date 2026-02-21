use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    
    #[serde(default = "default_max_sessions")]
    pub max_sessions: usize,
    
    #[serde(default = "default_max_fanout_per_session")]
    pub max_fanout_per_session: usize,
    
    #[serde(default = "default_buffer_size")]
    pub buffer_size: usize,
    
    #[serde(default = "default_session_timeout_secs")]
    pub session_timeout_secs: u64,
    
    #[serde(default = "default_enable_metrics")]
    pub enable_metrics: bool,
    
    #[serde(default = "default_metrics_bind_address")]
    pub metrics_bind_address: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: default_bind_address(),
            max_sessions: default_max_sessions(),
            max_fanout_per_session: default_max_fanout_per_session(),
            buffer_size: default_buffer_size(),
            session_timeout_secs: default_session_timeout_secs(),
            enable_metrics: default_enable_metrics(),
            metrics_bind_address: default_metrics_bind_address(),
        }
    }
}

impl ServerConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let config = config::Config::builder()
            .add_source(config::File::with_name("config/server").required(false))
            .add_source(config::Environment::with_prefix("RTP_FANOUT").separator("__"))
            .build()?;
        
        Ok(config.try_deserialize()?)
    }
}

fn default_bind_address() -> String {
    "0.0.0.0:5004".to_string()
}

fn default_max_sessions() -> usize {
    10000
}

fn default_max_fanout_per_session() -> usize {
    1000
}

fn default_buffer_size() -> usize {
    65536
}

fn default_session_timeout_secs() -> u64 {
    300
}

fn default_enable_metrics() -> bool {
    true
}

fn default_metrics_bind_address() -> String {
    "0.0.0.0:9090".to_string()
}
