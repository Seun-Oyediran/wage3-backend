//! Google OpenID Connect client, built once at startup and shared through AppState.

use openidconnect::{
    ClientId, ClientSecret, EndpointMaybeSet, EndpointNotSet, EndpointSet, IssuerUrl,
    JsonWebKeySet, JsonWebKeySetUrl, RedirectUrl,
    core::{CoreClient, CoreIdTokenVerifier, CoreProviderMetadata},
    reqwest,
};

use crate::config::Config;

/// The client type after discovery + redirect URI. The generics record which
/// endpoints are known (auth URL set, token/userinfo URLs maybe set by discovery).
pub type GoogleClient = CoreClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

#[derive(Debug, Clone)]
pub struct GoogleOidc {
    pub client: GoogleClient,
    /// Used for the code exchange and fetching Google's signing keys.
    pub http: reqwest::Client,
    issuer: IssuerUrl,
    jwks_uri: JsonWebKeySetUrl,
    client_id: ClientId,
    client_secret: ClientSecret,
}

impl GoogleOidc {
    /// Fetches Google's OIDC metadata (endpoints + signing keys). Needs network at startup.
    pub async fn discover(config: &Config) -> Result<Self, String> {
        // Redirects off: following them during the code exchange is an SSRF risk.
        let http = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let issuer = IssuerUrl::new("https://accounts.google.com".to_string())
            .map_err(|e| format!("Invalid Google issuer URL: {}", e))?;

        let metadata = CoreProviderMetadata::discover_async(issuer.clone(), &http)
            .await
            .map_err(|e| format!("Google OIDC discovery failed: {}", e))?;

        let redirect_uri = RedirectUrl::new(config.google_redirect_uri.clone())
            .map_err(|e| format!("GOOGLE_REDIRECT_URI is not a valid URL: {}", e))?;

        let client_id = ClientId::new(config.google_client_id.clone());
        let client_secret = ClientSecret::new(config.google_client_secret.clone());
        let jwks_uri = metadata.jwks_uri().clone();

        let client = CoreClient::from_provider_metadata(
            metadata,
            client_id.clone(),
            Some(client_secret.clone()),
        )
        .set_redirect_uri(redirect_uri);

        Ok(Self {
            client,
            http,
            issuer,
            jwks_uri,
            client_id,
            client_secret,
        })
    }

    /// ID-token verifier built from Google's current signing keys. Google rotates
    /// its keys, so they are fetched per sign-in instead of once at startup.
    pub async fn id_token_verifier(&self) -> Result<CoreIdTokenVerifier<'static>, String> {
        let jwks = JsonWebKeySet::fetch_async(&self.jwks_uri, &self.http)
            .await
            .map_err(|e| format!("Failed to fetch Google signing keys: {}", e))?;

        Ok(CoreIdTokenVerifier::new_confidential_client(
            self.client_id.clone(),
            self.client_secret.clone(),
            self.issuer.clone(),
            jwks,
        ))
    }
}
