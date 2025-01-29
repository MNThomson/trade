#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
        routing::RouterIntoService,
    };
    use http::request::Builder;
    use serde::{Deserialize, Serialize, de};
    use tower::{Service, ServiceExt};

    use crate::{db::DB, router, types::AppState};

    #[tokio::test]
    async fn integration() {
        let app = App::init().await;
    }

    #[derive(Serialize, Deserialize)]
    struct ApiResponseWrapper<T> {
        success: bool,
        data: T,
    }

    #[derive(Clone)]
    struct App {
        app: RouterIntoService<Body>,
    }

    impl App {
        async fn init() -> Self {
            let state = AppState {
                db: DB::init().await.unwrap(),
            };

            App {
                app: router(state).await.into_service(),
            }
        }

        async fn request<B: Serialize, R: for<'a> de::Deserialize<'a>>(
            mut self,
            request: Builder,
            payload: Option<B>,
        ) -> Result<(StatusCode, R), StatusCode> {
            let request = if let Some(ref p) = payload {
                request
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&p).unwrap()))
            } else {
                request.body(Body::empty())
            }
            .unwrap();

            let response = ServiceExt::<Request<Body>>::ready(&mut self.app)
                .await
                .unwrap()
                .call(request)
                .await
                .unwrap();

            let (_parts, rawbody) = response.into_parts();
            let bytes = axum::body::to_bytes(rawbody, usize::MAX).await.unwrap();
            let obj: ApiResponseWrapper<R> = serde_json::from_slice(&bytes).map_err(|e| {
                dbg!(e);
                _parts.status
            })?;

            Ok((_parts.status, obj.data))
        }
    }
}
