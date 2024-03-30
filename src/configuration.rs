use config::Config;
use secrecy::{ExposeSecret, Secret};

#[derive(serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application_port: u16,
}

#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    host: String,
    username: String,
    pub database_name: String,
    pub password: Secret<String>,
    port: u16,
}

impl DatabaseSettings {
    pub fn connection_string(&self, sslmode: &str) -> Secret<String> {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}/{}?sslmode={}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database_name,
            sslmode
        ))
    }

    pub fn connection_string_without_db(&self, sslmode: &str) -> Secret<String> {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}/postgres?sslmode={}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            sslmode
        ))
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    // Initialise our configuration reader
    let conf = Config::builder()
        .set_default("default", "")?
        // Add configuration values from a file named `configuration`.
        // It will look for any top-level file with an extension
        // that `config` knows how to parse: yaml, json, etc.
        .add_source(config::File::with_name("configuration"))
        .build()
        .unwrap();

    // Try to convert the configuration values it read into
    // our Settings type
    let settings = conf.try_deserialize::<Settings>();

    settings
}
