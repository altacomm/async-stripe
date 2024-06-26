//! Web Hooks
//! =========
//!
//! Reference: <https://stripe.com/docs/webhooks/test>
//!
//! This example shows how to manage web hooks.
//! To trigger it, you can use the stripe cli.
//!
//! TLDR;
//! ```
//! stripe listen --forward-to localhost:4242/stripe_webhooks
//! stripe trigger checkout.session.completed
//! ```

use std::net::SocketAddr;

use axum::{
    extract::FromRequest,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Error, Router,
};
use stripe::{Event, EventObject, EventType};

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[tokio::main]
async fn main() {
    // build our application with a route
    let app = Router::new().route("/stripe_webhooks", post(handle_webhook));

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([127, 0, 0, 1], 4242));
    println!("listening on {}", addr);
    axum::Server::bind(&addr).serve(app.into_make_service()).await.unwrap();
}

struct StripeEvent(Event);

#[async_trait::async_trait]
impl<S, B> FromRequest<S, B> for StripeEvent
where
    String: FromRequest<S, B>,
    B: Send + 'static,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        let signature = if let Some(sig) = req.headers().get("stripe-signature") {
            sig.to_owned()
        } else {
            return Err(StatusCode::BAD_REQUEST.into_response());
        };

        let payload =
            String::from_request(req, state).await.map_err(IntoResponse::into_response)?;

        Ok(Self(
            stripe::Webhook::construct_event(&payload, signature.to_str().unwrap(), "whsec_xxxxx")
                .map_err(|_| StatusCode::BAD_REQUEST.into_response())?,
        ))
    }
}

#[axum::debug_handler]
async fn handle_webhook(StripeEvent(event): StripeEvent) {
    match event.type_ {
        EventType::CheckoutSessionCompleted => {
            if let EventObject::CheckoutSession(session) = event.data.object {
                println!("Received checkout session completed webhook with id: {:?}", session.id);
            }
        }
        EventType::AccountUpdated => {
            if let EventObject::Account(account) = event.data.object {
                println!("Received account updated webhook for account: {:?}", account.id);
            }
        }
        _ => println!("Unknown event encountered in webhook: {:?}", event.type_),
    }
}
