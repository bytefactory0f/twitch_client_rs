use serde::Deserialize;
use std::error::Error;

use crate::credentials::Credentials;

#[allow(dead_code)] // Ignore unused fields in the response
#[derive(Deserialize)]
struct RefreshTokenResponse {
    access_token: String,
    expires_in: u32,
    refresh_token: String,
    scope: Vec<String>,
    token_type: String,
}

/// Makes an OAuth2 request to get a new access token from the refresh
/// token. Refresh tokens are longer lived than access tokens, so the
/// client can be configured once without having to add a new token
/// constantly.
pub async fn refresh_access_token(credentials: &Credentials) -> Result<String, Box<dyn Error>> {
    let res = reqwest::Client::new()
        .post("https://id.twitch.tv/oauth2/token")
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &credentials.refresh_token),
            ("client_id", &credentials.client_id),
            ("client_secret", &credentials.client_secret),
        ])
        .send()
        .await?
        .json::<RefreshTokenResponse>()
        .await?;

    Ok(res.access_token)
}
