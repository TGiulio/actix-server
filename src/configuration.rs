use config::Config;
use secrecy::{ExposeSecret, Secret};
use sqlx::{
    postgres::{PgConnectOptions, PgSslMode},
    ConnectOptions,
};

#[derive(serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
}

#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    pub host: String,
    pub username: String,
    pub database_name: String,
    pub password: Secret<String>,
    pub port: u16,
    pub require_ssl: bool,
}

#[derive(serde::Deserialize)]
pub struct ApplicationSettings {
    pub host: String,
    pub port: u16,
}

pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Environment::Local),
            "production" => Ok(Environment::Production),
            other => Err(format!("{} is not supported as environment", other)),
        }
    }
}

impl DatabaseSettings {
    pub fn without_db(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };

        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(&self.password.expose_secret())
            .port(self.port)
            .database("postgres")
            .ssl_mode(ssl_mode)
    }

    pub fn with_db(&self) -> PgConnectOptions {
        let mut options = self.without_db().database(&self.database_name);
        options.log_statements(tracing::log::LevelFilter::Trace);
        options
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path =
        std::env::current_dir().expect("failed to determine the current working directory");
    let configuration_dir = base_path.join("configuration");

    // detect the running environment
    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("failed to parse APP_ENVIRONMENT");

    // Initialise our configuration reader
    let conf = Config::builder()
        .set_default("default", "")?
        // Add configuration values from a file named `configuration`.
        // It will look for any top-level file with an extension
        // that `config` knows how to parse: yaml, json, etc.
        .add_source(config::File::from(configuration_dir.join("base")))
        .add_source(config::File::from(
            configuration_dir.join(environment.as_str()),
        ))
        .add_source(config::Environment::with_prefix("app").separator("__"))
        .build()
        .unwrap();

    // Try to convert the configuration values it read into
    // our Settings type
    let settings = conf.try_deserialize::<Settings>();

    settings
}
