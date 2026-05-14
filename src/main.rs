use http_server::web_server::{
    Body, ErrorRoute, Parameters, Request,
    RequestMethod::{Get, Post},
    Response, Route, StatusCode, WebServer,
};

struct Greeting {
    say: String,
    to: String,
}

impl TryFrom<Parameters> for Greeting {
    type Error = StatusCode;
    fn try_from(params: Parameters) -> Result<Self, Self::Error> {
        match (params.get("say"), params.get("to")) {
            (Some(say), Some(to)) => Ok(Greeting {
                say: say.clone(),
                to: to.clone(),
            }),
            _ => Err(StatusCode::BadRequest),
        }
    }
}

fn route_greeting_form_submission(
    _request: &Request,
    _path_params: Parameters,
    greeting: Greeting,
) -> Result<Response, StatusCode> {
    Ok(Response::ok(format!(
        "I will say {} to {}",
        greeting.say, greeting.to
    )))
}

fn route_query_params(request: &Request, _path_params: Parameters) -> Result<Response, StatusCode> {
    let body = if request.resource.query_params.is_empty() {
        "Dynamic page - call me with some query parameters!".to_string()
    } else {
        format!(
            "Called {} with query parameters \"{:?}\"",
            request.resource.path, request.resource.query_params
        )
    };

    Ok(Response::ok(body))
}

fn route_get_me(request: &Request, _path_params: Parameters) -> Result<Response, StatusCode> {
    match request.headers.get("user-cookie") {
        Some(cookie) if cookie == "test" => Ok(Response::ok("Welcome user!".to_string())),
        Some(_) => Err(StatusCode::Forbidden),
        None => Err(StatusCode::Unauthorized),
    }
}

fn route_get_user(_request: &Request, path_params: Parameters) -> Result<Response, StatusCode> {
    Ok(Response::ok(format!("User {}", path_params["id"])))
}

fn route_post_user(request: &Request, path_params: Parameters) -> Result<Response, StatusCode> {
    match &request.body {
        Body::JsonData(json) => Ok(Response::ok(format!(
            "User {}, body = {:?}",
            path_params["id"], json
        ))),
        _ => Err(StatusCode::UnsupportedMediaType),
    }
}

fn main() {
    WebServer::bind_and_listen_forever(
        "127.0.0.1:8080",
        Box::new([
            Route::file(Get, "/", "html/index.html"),
            Route::file(Get, "/index.html", "html/index.html"),
            Route::file(Get, "/greeting_form", "html/form.html"),
            Route::data_form(Post, "/greeting_form", route_greeting_form_submission),
            Route::func(Get, "/query_params", route_query_params),
            Route::func(Get, "/user/me", route_get_me),
            Route::func(Get, "/user/{id}", route_get_user),
            Route::func(Post, "/user/{id}", route_post_user),
        ]),
        Box::new([ErrorRoute::file(
            StatusCode::NotFound,
            "html/404_not_found.html",
        )]),
    );
}
