use std::io::Write;

use crate::cli::Config;
use chrono::DateTime;
use chrono::Utc;
use cli_42::results::user::UserElement;
use cli_42::results::*;
use cli_42::token::TokenInfo;
use cli_42::Mode;
use cli_42::Session;
use cli_42::SessionError;
use directories::BaseDirs;
use url::Url;

pub enum Command {
    Id,
    Me,
    Email,
    Login,
    CorrectionPoint,
    Wallet,
    Blackhole,
}

#[derive(Debug)]
pub struct Program {
    session: Session,
    pub token: Option<TokenInfo>,
    pub config: Config,
}

impl Program {
    pub async fn new(config: Config) -> Result<Self, SessionError> {
        if !(check_if_config_file_exists()) {
            // create new file
            create_config_toml()?;
            if let Ok(result) = check_config_toml() {
                if !result {
                    return Err(SessionError::ConfigFileNotFound);
                }
            }
        }
        let program = Program {
            session: Session::new(Some(Mode::Credentials)).await?,
            token: None,
            config,
        };
        Ok(program)
    }

    pub async fn call(&mut self, url: &str) -> Result<String, SessionError> {
        let res = self.session.call(url).await?;
        Ok(res)
    }

    pub async fn run_program(&mut self, command: Command) -> Result<(), SessionError> {
        let tmp = self.get_user_with_login().await?;
        let user = self.get_user_info_with_id(tmp.id).await?;
        match command {
            Command::Id => self.id(&tmp).await?,
            Command::Me => self.me(&user).await?,
            Command::Email => self.email(&user).await?,
            Command::Login => self.login(&user).await?,
            Command::CorrectionPoint => self.correction_point(&user).await?,
            Command::Wallet => self.wallet(&user).await?,
            Command::Blackhole => self.blackhole(&user).await?,
        }
        Ok(())
    }
}

impl Program {
    async fn get_user_with_login(&mut self) -> Result<user::UserElement, SessionError> {
        let url = "https://api.intra.42.fr/v2/users";
        let url = Url::parse_with_params(
            url,
            &[
                ("client_id", self.session.get_client_id()),
                ("filter[login]", self.session.get_login()),
            ],
        )?;

        let res = self.call(url.as_str()).await?;
        let user: user::User = serde_json::from_str(res.as_str())?;
        Ok(user[0].clone())
    }

    async fn get_user_info_with_id(&mut self, id: i64) -> Result<me::Me, SessionError> {
        let url = format!("https://api.intra.42.fr/v2/users/{}", id);
        let url = Url::parse_with_params(&url, &[("client_id", self.session.get_client_id())])?;

        let res = self.call(url.as_str()).await?;
        let me: me::Me = serde_json::from_str(res.as_str())?;
        Ok(me)
    }

    async fn me(&mut self, user: &me::Me) -> Result<(), SessionError> {
        let title = if user.titles.is_empty() {
            ""
        } else {
            user.titles[0].name.split(' ').next().unwrap_or("")
        };
        println!("{} | {} {}", user.displayname, title, user.login);
        self.wallet(user).await?;
        self.correction_point(user).await?;
        println!("{:20}{}", "Cursus", user.cursus_users[1].cursus.name);
        println!(
            "{:20}{}",
            "Grade",
            user.cursus_users[1]
                .grade
                .as_ref()
                .unwrap_or(&"".to_string())
        );
        self.blackhole(user).await?;
        Ok(())
    }

    async fn email(&mut self, user: &me::Me) -> Result<(), SessionError> {
        println!("{:20}{}", "Email", user.email);
        Ok(())
    }

    async fn wallet(&mut self, user: &me::Me) -> Result<(), SessionError> {
        println!("{:20}{}", "Wallet", user.wallet);
        Ok(())
    }

    async fn id(&mut self, tmp: &UserElement) -> Result<(), SessionError> {
        println!("{:20}{}", "ID", tmp.id);
        Ok(())
    }

    async fn login(&mut self, user: &me::Me) -> Result<(), SessionError> {
        println!("{:20}{}", "Login", user.login);
        Ok(())
    }

    async fn correction_point(&mut self, user: &me::Me) -> Result<(), SessionError> {
        println!("{:20}{}", "Correction point", user.correction_point);
        Ok(())
    }

    async fn blackhole(&mut self, user: &me::Me) -> Result<(), SessionError> {
        let utc = Utc::now();
        let utc2 = user.cursus_users[1]
            .blackholed_at
            .as_ref()
            .unwrap_or(&"".to_string())
            .parse::<DateTime<Utc>>()?;
        println!(
            "{:20}{}",
            "Blackhole",
            utc2.signed_duration_since(utc).num_days()
        );
        Ok(())
    }
}

fn check_if_config_file_exists() -> bool {
    if let Some(dir) = BaseDirs::new() {
        let path = dir.config_dir().join("config.toml");
        if !(path.exists()) {
            return false;
        }
    }
    true
}

fn create_config_toml() -> Result<(), SessionError> {
    use std::fs::File;
    use std::io::stdin;

    println!("Browse to: https://profile.intra.42.fr/oauth/applications/new");
    println!("Create new Application");
    println!("Set redirect_url to \"http://localhost:8080\"");

    if let Some(dir) = BaseDirs::new() {
        let path = dir.config_dir().join("config.toml");
        let mut file = File::create(path)?;
        let stdin = stdin();
        let mut line = String::new();

        println!("Enter client id: ");
        stdin.read_line(&mut line)?;
        writeln!(&mut file, "client_id=\"{}\"", line.trim())?;
        line.clear();
        println!("Enter client secret: ");
        stdin.read_line(&mut line)?;
        writeln!(&mut file, "client_secret=\"{}\"", line.trim())?;
        line.clear();
        println!("Enter intra login: ");
        stdin.read_line(&mut line)?;
        writeln!(&mut file, "login=\"{}\"", line.trim())?;
        Ok(())
    } else {
        Err(SessionError::BaseDirsNewError)
    }
}

fn check_config_toml() -> Result<bool, SessionError> {
    use std::io::ErrorKind;

    if let Some(dir) = BaseDirs::new() {
        let path = dir.config_dir().join("config.toml");
        let tmp = std::fs::read_to_string(path);
        match tmp {
            Ok(content) => {
                let config: Session = toml::from_str(&content)?;
                if !(check_client(&config)) {
                    return Ok(false);
                }
            }
            Err(e) => match e.kind() {
                ErrorKind::NotFound => {
                    eprintln!("config.toml not found");
                    return Ok(false);
                }
                ErrorKind::PermissionDenied => {
                    eprintln!("config.toml not readable");
                    return Ok(false);
                }
                _ => {
                    eprintln!("config.toml error.");
                    eprintln!("something went wrong.");
                }
            },
        }
    }
    Ok(true)
}

fn check_client(session: &Session) -> bool {
    let client_id = session.get_client_id();
    let client_secret = session.get_client_secret();
    if client_id.is_empty() || client_secret.is_empty() {
        return false;
    }
    if client_id.len() > 256 || client_secret.len() > 256 {
        return false;
    }
    true
}
