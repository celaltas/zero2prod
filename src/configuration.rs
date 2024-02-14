use config::{Config, File, FileFormat};

#[derive(serde::Deserialize, Debug)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub database_name: String,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database_name
        )
    }
    pub fn connection_string_without_db(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}",
            self.username, self.password, self.host, self.port
        )
    }
}

#[derive(serde::Deserialize, Debug)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application_port: u16,
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let mut builder = Config::builder();
    builder = builder.add_source(File::new("configuration", FileFormat::Yaml));

    match builder.build() {
        Ok(config) => config.try_deserialize::<Settings>(),
        Err(e) => Err(e),
    }
}
