use askama::Template;
use axum::{
    extract::Path,
    response::{Html, IntoResponse, Response},
};
use hyper::StatusCode;
use uuid::Uuid;

pub async fn casta_index(Path(uuid): Path<Uuid>) -> impl IntoResponse {
    let template = CastaTemplate { uuid };
    HtmlTemplate(template)
}

#[derive(Template)]
#[template(path = "casta.html")]
struct CastaTemplate {
    uuid: Uuid,
}

/// A wrapper type that we'll use to encapsulate HTML parsed by askama into valid HTML for axum to serve.
struct HtmlTemplate<T>(T);

/// Allows us to convert Askama HTML templates into valid HTML for axum to serve in the response.
impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        // Attempt to render the template with askama
        match self.0.render() {
            // If we're able to successfully parse and aggregate the template, serve it
            Ok(html) => Html(html).into_response(),
            // If we're not, return an error or some bit of fallback HTML
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {}", err),
            )
                .into_response(),
        }
    }
}
