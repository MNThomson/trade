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

    use crate::{
        admin::{AddStockToUserRequest, CreateStockRequest},
        db::DB,
        router,
        telemetry::tracing_init,
        types::{AppState, StockId, TokenResponse},
        user::{LoginRequest, RegisterRequest},
    };

    #[tokio::test]
    async fn integration() {
        tracing_init(env!("CARGO_PKG_NAME"), env!("GIT_HASH"));

        let app = App::init().await;

        // Vanguard Register
        let sc = app
            .clone()
            .register(RegisterRequest {
                user_name: String::from("VanguardETF"),
                password: String::from("Vang@123"),
                name: String::from("Vanguard Corp."),
            })
            .await
            .unwrap();
        assert_eq!(sc, 201);

        // Vanguard username already taken
        let sc = app
            .clone()
            .register(RegisterRequest {
                user_name: String::from("VanguardETF"),
                password: String::from("Comp@124"),
                name: String::from("Vanguard Ltd."),
            })
            .await
            .unwrap_err();
        assert_eq!(sc, 409);

        // Vanguard Incorrect Password Login
        let sc = app
            .clone()
            .login(LoginRequest {
                user_name: String::from("VanguardETF"),
                password: String::from("Vang@1234"),
            })
            .await
            .unwrap_err();
        assert_eq!(sc, 401);

        // Vanguard Login
        let (sc, resp) = app
            .clone()
            .login(LoginRequest {
                user_name: String::from("VanguardETF"),
                password: String::from("Vang@123"),
            })
            .await
            .unwrap();
        let vanguard_token = resp.token;
        assert_eq!(sc, 200);
        assert!(vanguard_token.len() > 10);

        // Create Google Stock
        let (sc, resp) = app
            .clone()
            .create_stock(&vanguard_token, CreateStockRequest {
                stock_name: String::from("Google"),
            })
            .await
            .unwrap();
        let google_stock_id = resp.stock_id;
        assert_eq!(sc, 200);

        // Add 550 Google Stock to Vanguard
        let sc = app
            .clone()
            .add_stock_to_user(&vanguard_token, AddStockToUserRequest {
                stock_id: google_stock_id,
                quantity: 550,
            })
            .await
            .unwrap();
        assert_eq!(sc, 200);

        // Create Apple Stock
        let (sc, resp) = app
            .clone()
            .create_stock(&vanguard_token, CreateStockRequest {
                stock_name: String::from("Apple"),
            })
            .await
            .unwrap();
        let apple_stock_id = resp.stock_id;
        assert_eq!(sc, 200);

        // Add 350 Apple Stock to Vanguard
        let sc = app
            .clone()
            .add_stock_to_user(&vanguard_token, AddStockToUserRequest {
                stock_id: apple_stock_id,
                quantity: 350,
            })
            .await
            .unwrap();
        assert_eq!(sc, 200);
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
            token: &String,
            request: Builder,
            payload: Option<B>,
        ) -> Result<(StatusCode, R), StatusCode> {
            let request = request.header("token", token);
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

        async fn register(self, payload: RegisterRequest) -> Result<StatusCode, StatusCode> {
            let (sc, _resp) = self
                .request::<_, Option<i64>>(
                    &"".to_string(),
                    Request::builder()
                        .uri("/authentication/register")
                        .method("POST"),
                    Some(payload),
                )
                .await?;

            Ok(sc)
        }

        async fn login(
            self,
            payload: LoginRequest,
        ) -> Result<(StatusCode, TokenResponse), StatusCode> {
            let resp = self
                .request::<_, TokenResponse>(
                    &"".to_string(),
                    Request::builder()
                        .uri("/authentication/login")
                        .method("POST"),
                    Some(payload),
                )
                .await?;

            Ok(resp)
        }

        async fn create_stock(
            self,
            token: &String,
            payload: CreateStockRequest,
        ) -> Result<(StatusCode, StockId), StatusCode> {
            let resp = self
                .request::<_, StockId>(
                    token,
                    Request::builder().uri("/setup/createStock").method("POST"),
                    Some(payload),
                )
                .await?;

            Ok(resp)
        }

        async fn add_stock_to_user(
            self,
            token: &String,
            payload: AddStockToUserRequest,
        ) -> Result<StatusCode, StatusCode> {
            let (sc, _resp) = self
                .request::<_, Option<i64>>(
                    token,
                    Request::builder()
                        .uri("/setup/addStockToUser")
                        .method("POST"),
                    Some(payload),
                )
                .await?;

            Ok(sc)
        }
    }
}
