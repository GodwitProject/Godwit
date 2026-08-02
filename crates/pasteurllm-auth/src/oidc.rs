use openidconnect::{
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata, CoreResponseType},
    reqwest::async_http_client,
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, RedirectUrl, Scope,
};
use pasteurllm_core::{OidcProviderConfig, PasteurError};
use url::Url;

pub struct OidcClient {
    inner: CoreClient,
    provider_id: String,
}

impl OidcClient {
    pub async fn new(config: &OidcProviderConfig) -> Result<Self, PasteurError> {
        let issuer_url = IssuerUrl::new(config.issuer_url.clone())
            .map_err(|e| PasteurError::Config(e.to_string()))?;
        let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, async_http_client)
            .await
            .map_err(|e| PasteurError::Auth(e.to_string()))?;
        let client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(config.client_id.clone()),
            Some(ClientSecret::new(config.client_secret.clone())),
        )
        .set_redirect_uri(
            RedirectUrl::new(config.redirect_uri.clone())
                .map_err(|e| PasteurError::Config(e.to_string()))?,
        );
        Ok(Self {
            inner: client,
            provider_id: config.id.clone(),
        })
    }

    pub fn authorize_url(&self, scopes: Vec<String>) -> (Url, CsrfToken, Nonce) {
        let mut request = self
            .inner
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            );
        for scope in scopes {
            request = request.add_scope(Scope::new(scope));
        }
        request.url()
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        _csrf: &str,
        nonce: &str,
    ) -> Result<(String, String, Option<String>), PasteurError> {
        let token_response = self
            .inner
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .request_async(async_http_client)
            .await
            .map_err(|e| PasteurError::Auth(e.to_string()))?;
        let id_token = token_response
            .extra_fields()
            .id_token()
            .ok_or_else(|| PasteurError::Auth("missing id_token".to_string()))?;
        let nonce = Nonce::new(nonce.to_string());
        let claims = id_token
            .claims(&self.inner.id_token_verifier(), &nonce)
            .map_err(|e| PasteurError::Auth(e.to_string()))?;
        let email = claims
            .email()
            .map(|e| e.as_str().to_string())
            .ok_or_else(|| PasteurError::Auth("missing email".to_string()))?;
        let name = claims.name().and_then(|n| n.get(None)).map(|s| s.to_string());
        let subject = claims.subject().to_string();
        Ok((email, subject, name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_issuer_fails() {
        let config = OidcProviderConfig {
            id: "bad".to_string(),
            issuer_url: "not-a-url".to_string(),
            client_id: "x".to_string(),
            client_secret: "y".to_string(),
            redirect_uri: "http://localhost/callback".to_string(),
        };
        // Discovery cannot run in unit test; test URL parsing error path.
        assert!(IssuerUrl::new(config.issuer_url.clone()).is_err());
    }
}
