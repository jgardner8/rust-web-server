use derive_try_from_parameters::TryFromParameters;
use http_server::web_server::{
    self, Body, ErrorRoute, FromJson, Json, Parameters, Request,
    RequestMethod::{Get, Post},
    Response, Route, StatusCode,
};

#[derive(TryFromParameters)]
struct Greeting {
    say: String,
    to: String,
    times: u8,
}

#[derive(Debug)]
struct User {
    id: u32,
    name: String,
    preferences: Preferences,
}

#[derive(Debug)]
struct Preferences {
    dark_mode: bool,
}

impl FromJson for User {
    fn from_json(json: Json) -> Option<Self> {
        match json {
            Json::Object(mut map) => match (
                map.remove("id").and_then(u32::from_json),
                map.remove("name").and_then(String::from_json),
                map.remove("preferences").and_then(Preferences::from_json),
            ) {
                (Some(id), Some(name), Some(preferences)) => Some(Self {
                    id,
                    name,
                    preferences,
                }),
                _ => None,
            },
            _ => None,
        }
    }
}

impl FromJson for Preferences {
    fn from_json(json: Json) -> Option<Self> {
        match json {
            Json::Object(mut map) => match map.remove("dark_mode").and_then(bool::from_json) {
                Some(dark_mode) => Some(Self { dark_mode }),
                None => None,
            },
            _ => None,
        }
    }
}

fn route_greeting_result(
    _request: &Request,
    _path_params: Parameters,
    greeting: Greeting,
) -> Result<Response, StatusCode> {
    let body = if greeting.times <= 3 {
        format!("I will say {} to {}", greeting.say, greeting.to)
    } else {
        "I'm not a spam robot!".to_string()
    };
    Ok(Response::ok(body))
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

fn route_post_user(
    _request: &Request,
    _path_params: Parameters,
    user: User,
) -> Result<Response, StatusCode> {
    Ok(Response::ok(format!("body = {:?}", user)))
}

fn main() {
    web_server::bind_and_listen_forever(
        "127.0.0.1:8080",
        Box::new([
            Route::file(Get, "/", "html/index.html"),
            Route::file(Get, "/index.html", "html/index.html"),
            Route::file(Get, "/greeting_form", "html/form.html"),
            Route::data_form(Post, "/greeting_form", route_greeting_result),
            Route::data_query(Get, "/greeting_query_params", route_greeting_result),
            Route::func(Get, "/query_params", route_query_params),
            Route::func(Get, "/user/me", route_get_me),
            Route::func(Get, "/user/{id}", route_get_user),
            Route::data_json(Post, "/user", route_post_user),
        ]),
        Box::new([ErrorRoute::file(
            StatusCode::NotFound,
            "html/404_not_found.html",
        )]),
    );
}
