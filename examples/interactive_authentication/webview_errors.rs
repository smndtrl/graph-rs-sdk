use anyhow::Error;
use graph_error::WebViewExecutionError;
use graph_oauth::oauth::AuthorizationCodeCredential;

async fn customize_webview(tenant_id: &str, client_id: &str, scope: Vec<&str>, redirect_uri: &str) {
    let mut credential_builder_result =
        AuthorizationCodeCredential::authorization_url_builder(client_id)
            .with_tenant(tenant_id)
            .with_scope(scope)
            .with_redirect_uri(redirect_uri)
            .with_interactive_authentication(None);

    if let Ok(credential_builder) = credential_builder_result {
        // ...
    } else if let Err(err) = credential_builder_result {
        match err {
            // Issues with the redirect uri such as specifying localhost
            // but not providing a port in the WebViewOptions.
            WebViewExecutionError::InvalidRedirectUri(uri) => {}
            // The user closed the webview window without logging in.
            WebViewExecutionError::WindowClosedRequested => {}
            // The user navigated to a url that was not the login url
            // or a redirect url specified. Requires that WebViewOptions
            // has the enforcement of invalid navigation enabled.
            WebViewExecutionError::WindowClosedOnInvalidNavigation => {}
            // The webview exited because of a timeout defined in the WebViewOptions.
            WebViewExecutionError::WindowClosedOnTimeoutReached => {}
            // The host or domain provided or set for login is invalid.
            // This could be an internal error and most likely will never happen.
            WebViewExecutionError::InvalidStartUri { reason } => {}
            // The webview was successfully redirected but the url did not
            // contain a query or fragment. The query or fragment of the url
            // is where the auth code would be returned to the app.
            WebViewExecutionError::RedirectUriMissingQueryOrFragment(_) => {}
            // Serde serialization error when attempting to serialize
            // the query or fragment of the url to a AuthorizationQueryResponse
            WebViewExecutionError::SerdeError(_) => {}
            // Error from AuthorizationCodeCredential Authorization Url Builder: AuthCodeAuthorizationUrlParameters
            // This most likely came from an invalid parameter or missing parameter
            // passed to the client used for building the url. See graph_rs_sdk::oauth::AuthCodeAuthorizationUrlParameters
            WebViewExecutionError::AuthorizationError(authorization_failure) => {}
            WebViewExecutionError::RecvError(_) => {}
            WebViewExecutionError::AuthExecutionError(_) => {}
        }
    }
}
