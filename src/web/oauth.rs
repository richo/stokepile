use oauth2::basic::BasicClient;
use oauth2::prelude::*;
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, Scope, TokenUrl};
use oauth2::TokenResponse;

use rocket::http::RawStr;
use rocket::form::{FromFormField, FromParam};

use failure::Error;

use crate::messages::Oauth2Provider;

use std::env;
use url::Url;

#[derive(Debug)]
enum GoogleProperty {
    Drive,
    Youtube,
}

lazy_static! {
    pub static ref DROPBOX_CONFIG: Oauth2Config = {
        info!("Initializing Dropbox oauth config");
        Oauth2Config::dropbox()
    };
    pub static ref YOUTUBE_CONFIG: Oauth2Config = {
        info!("Initializing Youtube oauth config");
        Oauth2Config::google(GoogleProperty::Youtube)
    };
    pub static ref GOOGLE_DRIVE_CONFIG: Oauth2Config = {
        info!("Initializing Youtube oauth config");
        Oauth2Config::google(GoogleProperty::Drive)
    };
    pub static ref VIMEO_CONFIG: Oauth2Config = {
        info!("Initializing Vimeo oauth config");
        Oauth2Config::vimeo()
    };

    static ref BASE_URL: Url = Url::parse(
        &env::var("STOKEPILE_BASE_URL")
            .expect("Missing the STOKEPILE_BASE_URL environment variable."),
    )
    .expect("Invalid STOKEPILE_BASE_URL");
}

#[derive(Clone, RedactedDebug)]
pub struct Oauth2Config {
    pub client_id: ClientId,
    #[redacted]
    pub client_secret: ClientSecret,
    pub auth_url: AuthUrl,
    pub token_url: TokenUrl,
    pub scopes: &'static [&'static str],
    pub redirect_url: RedirectUrl,
}

impl Oauth2Config {
    /// Creates a Oauth2Config configured for Dropbox, panicing on many types of failure, since
    /// they are all unrecoverable.
    fn dropbox() -> Oauth2Config {
        let client_id = ClientId::new(
            env::var("STOKEPILE_DROPBOX_APP_KEY")
                .expect("Missing the STOKEPILE_DROPBOX_APP_KEY environment variable."),
        );
        let client_secret = ClientSecret::new(
            env::var("STOKEPILE_DROPBOX_APP_SECRET")
                .expect("Missing the STOKEPILE_DROPBOX_APP_SECRET environment variable."),
        );
        let auth_url = AuthUrl::new(
            Url::parse("https://www.dropbox.com/oauth2/authorize")
                .expect("Invalid authorization endpoint URL"),
        );
        let token_url = TokenUrl::new(
            Url::parse("https://www.dropbox.com/oauth2/token").expect("Invalid token endpoint URL"),
        );
        let redirect_url = RedirectUrl::new(
            BASE_URL
                .join("/integration/finish?provider=dropbox")
                .expect("Invalid redirect URL"),
        );
        let scopes = &[];
        Oauth2Config {
            client_id,
            client_secret,
            auth_url,
            token_url,
            scopes,
            redirect_url,
        }
    }
    fn google(property: GoogleProperty) -> Oauth2Config {
        let client_id = ClientId::new(
            env::var("STOKEPILE_GOOGLE_APP_KEY")
                .expect("Missing the STOKEPILE_GOOGLE_APP_KEY environment variable."),
        );
        let client_secret = ClientSecret::new(
            env::var("STOKEPILE_GOOGLE_APP_SECRET")
                .expect("Missing the STOKEPILE_GOOGLE_APP_SECRET environment variable."),
        );
        let auth_url = AuthUrl::new(
            Url::parse("https://accounts.google.com/o/oauth2/v2/auth?access_type=offline")
                .expect("Invalid authorization endpoint URL"),
        );
        let token_url = TokenUrl::new(
            Url::parse("https://www.googleapis.com/oauth2/v4/token")
                .expect("Invalid token endpoint URL"),
        );
        let redirect_url = RedirectUrl::new(
            BASE_URL
                .join(match property {
                    GoogleProperty::Youtube => "/integration/finish?provider=youtube",
                    GoogleProperty::Drive=> "/integration/finish?provider=drive",
                })
                .expect("Invalid redirect URL"),
        );

        let scopes = match property {
            GoogleProperty::Youtube => &["https://www.googleapis.com/auth/youtube.upload"],
            GoogleProperty::Drive => &["https://www.googleapis.com/auth/drive.file"],
        };
        Oauth2Config {
            client_id,
            client_secret,
            auth_url,
            token_url,
            scopes,
            redirect_url,
        }
    }
    fn vimeo() -> Oauth2Config {
        let client_id = ClientId::new(
            env::var("STOKEPILE_VIMEO_APP_KEY")
                .expect("Missing the STOKEPILE_VIMEO_APP_KEY environment variable."),
        );
        let client_secret = ClientSecret::new(
            env::var("STOKEPILE_VIMEO_APP_SECRET")
                .expect("Missing the STOKEPILE_VIMEO_APP_SECRET environment variable."),
        );
        let auth_url = AuthUrl::new(
            Url::parse("https://api.vimeo.com/oauth/authorize")
                .expect("Invalid authorization endpoint URL"),
        );
        let token_url = TokenUrl::new(
            Url::parse("https://api.vimeo.com/oauth/access_token")
                .expect("Invalid token endpoint URL"),
        );
        let redirect_url = RedirectUrl::new(
            BASE_URL
                .join("/integration/finish?provider=vimeo")
                .expect("Invalid redirect URL"),
        );
        let scopes = &["public", "upload"];
        Oauth2Config {
            client_id,
            client_secret,
            auth_url,
            token_url,
            scopes,
            redirect_url,
        }
    }
    fn client(&self) -> BasicClient {
        let Oauth2Config {
            client_id,
            client_secret,
            auth_url,
            token_url,
            scopes,
            redirect_url,
        } = self;
        let client = BasicClient::new(
            client_id.clone(),
            Some(client_secret.clone()),
            auth_url.clone(),
            Some(token_url.clone()),
        )
        .set_redirect_url(redirect_url.clone());
        scopes.iter().fold(client, |client, scope| {
            client.add_scope(Scope::new(scope.to_string()))
        })
    }
}

impl Oauth2Provider {
    pub fn providers() -> &'static [Oauth2Provider] {
        static VARIANTS: &'static [Oauth2Provider] = &[
            Oauth2Provider::Dropbox,
            Oauth2Provider::YouTube,
            Oauth2Provider::GoogleDrive,
            Oauth2Provider::Vimeo,
        ];
        VARIANTS
    }

    pub fn name(&self) -> &'static str {
        match self {
            Oauth2Provider::Dropbox => "dropbox",
            Oauth2Provider::YouTube => "youtube",
            Oauth2Provider::GoogleDrive => "drive",
            Oauth2Provider::Vimeo => "vimeo",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Oauth2Provider::Dropbox => "Dropbox",
            Oauth2Provider::YouTube => "YouTube",
            Oauth2Provider::GoogleDrive => "Google Drive",
            Oauth2Provider::Vimeo => "Vimeo",
        }
    }

    pub fn client(&self) -> BasicClient {
        let config: &Oauth2Config = match self {
            Oauth2Provider::Dropbox => &*DROPBOX_CONFIG,
            Oauth2Provider::YouTube => &*YOUTUBE_CONFIG,
            Oauth2Provider::GoogleDrive => &*GOOGLE_DRIVE_CONFIG,
            Oauth2Provider::Vimeo => &*VIMEO_CONFIG,
        };

        config.client()
    }
}

impl Oauth2Provider {
    fn parse<'v>(from: &'v RawStr) -> Result<Oauth2Provider, String> {
        let decoded = from.url_decode();
        match decoded {
            Ok(ref provider) if provider == "dropbox" => Ok(Oauth2Provider::Dropbox),
            Ok(ref provider) if provider == "youtube" => Ok(Oauth2Provider::YouTube),
            Ok(ref provider) if provider == "drive" => Ok(Oauth2Provider::GoogleDrive),
            Ok(ref provider) if provider == "vimeo" => Ok(Oauth2Provider::Vimeo),
            _ => Err(format!("unknown provider {}", from)),
        }
    }
}

/// Exchange a code returned by an oauth provider, yielding an access token and maybe a refresh
/// token.
#[cfg(not(test))]
pub fn exchange_oauth_code<'a>(provider: &Oauth2Provider, code: &str) -> Result<(String, Option<String>), Error> {
    use oauth2::AuthorizationCode;

    info!("Invoked live excahnge_auth_code");
    let client = provider.client();
    client.exchange_code(AuthorizationCode::new(code.to_string()))
        .map_err(|e| e.into())
        .map(|token| {
            // TODO(richo) Can we abuse serde to do this for us without having to carry these
            // values about?
            (
                token.access_token().secret().to_string(),
                token.refresh_token().map(|v| v.secret().clone())
            )
        })
}

#[cfg(test)]
pub fn exchange_oauth_code<'a>(provider: &Oauth2Provider, code: &str) -> Result<(String, Option<String>), Error> {
    info!("Invoked test excahnge_auth_code");
    Ok(("test_access_token".into(), Some("test_refresh_token".into())))
}

impl<'v> FromFormValue<'v> for Oauth2Provider {
    type Error = String;

    fn from_form_value(form_value: &'v RawStr) -> Result<Oauth2Provider, Self::Error> {
        Self::parse(form_value)
    }
}

impl<'r> FromParam<'r> for Oauth2Provider {
    type Error = String;

    fn from_param(param: &'r RawStr) -> Result<Self, Self::Error> {
        Self::parse(param)
    }
}
