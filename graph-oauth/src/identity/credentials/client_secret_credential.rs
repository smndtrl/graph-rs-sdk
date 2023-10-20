use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use async_trait::async_trait;
use http::{HeaderMap, HeaderName, HeaderValue};
use url::Url;
use uuid::Uuid;

use graph_error::{AuthExecutionError, AuthorizationFailure, IdentityResult};
use graph_extensions::cache::{InMemoryTokenStore, TokenCacheStore};

use crate::auth::{OAuthParameter, OAuthSerializer};
use crate::identity::{
    credentials::app_config::AppConfig, Authority, AzureCloudInstance,
    ClientCredentialsAuthorizationUrlParameterBuilder, ConfidentialClientApplication,
    ForceTokenRefresh, Token, TokenCredentialExecutor,
};

credential_builder!(
    ClientSecretCredentialBuilder,
    ConfidentialClientApplication<ClientSecretCredential>
);

/// Client Credentials flow using a client secret.
///
/// The OAuth 2.0 client credentials grant flow permits a web service (confidential client)
/// to use its own credentials, instead of impersonating a user, to authenticate when calling
/// another web service. The grant specified in RFC 6749, sometimes called two-legged OAuth,
/// can be used to access web-hosted resources by using the identity of an application.
/// This type is commonly used for server-to-server interactions that must run in the background,
/// without immediate interaction with a user, and is often referred to as daemons or service accounts.
///
/// See [Microsoft identity platform and the OAuth 2.0 client credentials flow](https://docs.microsoft.com/en-us/azure/active-directory/develop/v2-oauth2-client-creds-grant-flow)
#[derive(Clone)]
pub struct ClientSecretCredential {
    pub(crate) app_config: AppConfig,
    /// Required
    /// The application secret that you created in the app registration portal for your app.
    /// Don't use the application secret in a native app or single page app because a
    /// client_secret can't be reliably stored on devices or web pages. It's required for web
    /// apps and web APIs, which can store the client_secret securely on the server side. Like
    /// all parameters here, the client secret must be URL-encoded before being sent. This step
    /// is done by the SDK. For more information on URI encoding, see the URI Generic Syntax
    /// specification. The Basic auth pattern of instead providing credentials in the Authorization
    /// header, per RFC 6749 is also supported.
    pub(crate) client_secret: String,
    /// The value passed for the scope parameter in this request should be the resource
    /// identifier (application ID URI) of the resource you want, affixed with the .default
    /// suffix. For the Microsoft Graph example, the value is https://graph.microsoft.com/.default.
    /// Default is https://graph.microsoft.com/.default.
    pub(crate) scope: Vec<String>,
    serializer: OAuthSerializer,
    token_cache: InMemoryTokenStore<Token>,
}

impl Debug for ClientSecretCredential {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientSecretCredential")
            .field("app_config", &self.app_config)
            .field("scope", &self.scope)
            .finish()
    }
}

impl ClientSecretCredential {
    pub fn new<T: AsRef<str>>(client_id: T, client_secret: T) -> ClientSecretCredential {
        ClientSecretCredential {
            app_config: AppConfig::new_with_client_id(client_id),
            client_secret: client_secret.as_ref().to_owned(),
            scope: vec!["https://graph.microsoft.com/.default".into()],
            serializer: OAuthSerializer::new(),
            token_cache: InMemoryTokenStore::new(),
        }
    }

    pub fn new_with_tenant<T: AsRef<str>>(
        tenant_id: T,
        client_id: T,
        client_secret: T,
    ) -> ClientSecretCredential {
        ClientSecretCredential {
            app_config: AppConfig::new_with_tenant_and_client_id(tenant_id, client_id),
            client_secret: client_secret.as_ref().to_owned(),
            scope: vec!["https://graph.microsoft.com/.default".into()],
            serializer: OAuthSerializer::new(),
            token_cache: InMemoryTokenStore::new(),
        }
    }

    pub fn authorization_url_builder<T: AsRef<str>>(
        client_id: T,
    ) -> ClientCredentialsAuthorizationUrlParameterBuilder {
        ClientCredentialsAuthorizationUrlParameterBuilder::new(client_id)
    }
}

#[async_trait]
impl TokenCacheStore for ClientSecretCredential {
    type Token = Token;

    fn get_token_silent(&mut self) -> Result<Self::Token, AuthExecutionError> {
        let cache_id = self.app_config.cache_id.to_string();
        if let Some(token) = self.token_cache.get(cache_id.as_str()) {
            if token.is_expired_sub(time::Duration::minutes(5)) {
                let response = self.execute()?;
                let msal_token: Token = response.json()?;
                self.token_cache.store(cache_id, msal_token.clone());
                Ok(msal_token)
            } else {
                Ok(token)
            }
        } else {
            let response = self.execute()?;
            let msal_token: Token = response.json()?;
            self.token_cache.store(cache_id, msal_token.clone());
            Ok(msal_token)
        }
    }

    #[tracing::instrument]
    async fn get_token_silent_async(&mut self) -> Result<Self::Token, AuthExecutionError> {
        let cache_id = self.app_config.cache_id.to_string();
        if let Some(token) = self.token_cache.get(cache_id.as_str()) {
            if token.is_expired_sub(time::Duration::minutes(5)) {
                let response = self.execute_async().await?;
                let msal_token: Token = response.json().await?;
                tracing::debug!("tokenResponse={:#?}", &msal_token);
                self.token_cache.store(cache_id, msal_token.clone());
                Ok(msal_token)
            } else {
                tracing::debug!("tokenResponse={:#?}", &token);
                Ok(token.clone())
            }
        } else {
            let response = self.execute_async().await?;
            let msal_token: Token = response.json().await?;
            tracing::debug!("tokenResponse={:#?}", &msal_token);
            self.token_cache.store(cache_id, msal_token.clone());
            Ok(msal_token)
        }
    }
}

#[async_trait]
impl TokenCredentialExecutor for ClientSecretCredential {
    fn uri(&mut self) -> IdentityResult<Url> {
        let azure_cloud_instance = self.azure_cloud_instance();
        self.serializer
            .authority(&azure_cloud_instance, &self.authority());

        let uri =
            self.serializer
                .get(OAuthParameter::TokenUrl)
                .ok_or(AuthorizationFailure::msg_err(
                    "token_url for access and refresh tokens missing",
                    "Internal Error",
                ))?;
        Url::parse(uri.as_str()).map_err(AuthorizationFailure::from)
    }

    fn form_urlencode(&mut self) -> IdentityResult<HashMap<String, String>> {
        let client_id = self.app_config.client_id.to_string();
        if client_id.is_empty() || self.app_config.client_id.is_nil() {
            return AuthorizationFailure::result(OAuthParameter::ClientId);
        }

        if self.client_secret.trim().is_empty() {
            return AuthorizationFailure::result(OAuthParameter::ClientSecret);
        }

        self.serializer
            .client_id(client_id.as_str())
            .client_secret(self.client_secret.as_str())
            .grant_type("client_credentials");

        if self.scope.is_empty() {
            self.serializer
                .extend_scopes(vec!["https://graph.microsoft.com/.default".to_owned()]);
        } else {
            self.serializer.extend_scopes(&self.scope);
        }

        // Don't include ClientId and Client Secret in the fields for form url encode because
        // Client Id and Client Secret are already included as basic auth.
        self.serializer
            .as_credential_map(vec![OAuthParameter::Scope], vec![OAuthParameter::GrantType])
    }

    fn client_id(&self) -> &Uuid {
        &self.app_config.client_id
    }

    fn authority(&self) -> Authority {
        self.app_config.authority.clone()
    }

    fn azure_cloud_instance(&self) -> AzureCloudInstance {
        self.app_config.azure_cloud_instance
    }

    fn basic_auth(&self) -> Option<(String, String)> {
        Some((
            self.app_config.client_id.to_string(),
            self.client_secret.clone(),
        ))
    }

    fn app_config(&self) -> &AppConfig {
        &self.app_config
    }
}

#[derive(Clone)]
pub struct ClientSecretCredentialBuilder {
    credential: ClientSecretCredential,
}

impl ClientSecretCredentialBuilder {
    pub fn new<T: AsRef<str>>(client_id: T, client_secret: T) -> Self {
        ClientSecretCredentialBuilder {
            credential: ClientSecretCredential::new(client_id, client_secret),
        }
    }

    pub(crate) fn new_with_client_secret(
        client_secret: impl AsRef<str>,
        app_config: AppConfig,
    ) -> ClientSecretCredentialBuilder {
        Self {
            credential: ClientSecretCredential {
                app_config,
                client_secret: client_secret.as_ref().to_string(),
                scope: vec!["https://graph.microsoft.com/.default".into()],
                serializer: Default::default(),
                token_cache: InMemoryTokenStore::new(),
            },
        }
    }

    pub fn with_client_secret<T: AsRef<str>>(&mut self, client_secret: T) -> &mut Self {
        self.credential.client_secret = client_secret.as_ref().to_owned();
        self
    }

    pub fn build_client(&self) -> ConfidentialClientApplication<ClientSecretCredential> {
        ConfidentialClientApplication::credential(self.credential.clone())
    }

    pub fn credential(&self) -> ClientSecretCredential {
        self.credential.clone()
    }
}
