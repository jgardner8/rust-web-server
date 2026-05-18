# Rust Web Server / Framework

Rust web server and framework with easy API and JSON parser. 

**Not tested seriously in production - for learning purposes.**

## Features

* Built from basics - custom `ThreadPool`, `Arc`, `Vec`, `RequestParser` and `JsonParser` are used with Rust's `TCPListener`. All HTTP protocol implementation is here.
* Parses JSON, query strings and form submissions into user's model struct using derive macros. Validation and parsing completely handled at framework level.

## Usage

See [main.rs](https://github.com/jgardner8/rust-web-server/blob/master/src/main.rs) for a complete example, including the below content.

### Routing

`"{var}"` in a route is used as a variable. It will be passed through to request handler in `path_params: Parameters` with key `var`

```rust
fn main() {
    web_server::bind_and_listen_forever(
        "127.0.0.1:8080",
        Box::new([
            Route::file(Get, "/", "html/index.html"),
            Route::func(Get, "/user/{id}", route_get_user),
            Route::data_form(Post, "/greeting_form", route_greeting_result),
            Route::data_query(Get, "/greeting_query_params", route_greeting_result),
            Route::data_json(Post, "/user", route_post_user),
        ]),
        Box::new([
            ErrorRoute::file(StatusCode::NotFound, "html/404.html")
        ]),
    );
}
```

### Model structs

It's nice to have structs passed directly into your request handlers rather than `Json` or `Parameters` objects. 

To receive these, the struct requires a `FromJson` or `TryFrom<Parameters>` impl and a `Route::data_*` route type must be used. See below for details.

#### JSON

Define models relevant to your application and use `#[derive(FromJsonObject)]` or a manual `FromJson` trait impl to make it parsable. Receive it in your request handler using a `Route::data_json` route type.

```rust
#[derive(FromJsonObject)]
struct User {
    id: u32,
    name: String,
    preferences: Preferences,
}

#[derive(FromJsonObject)]
struct Preferences {
    dark_mode: bool,
    trash: Vec<Json>,
}
```

#### Parameters (HTML form submission or query strings)

Define models relevant to your application and use `#[derive(TryFromParameters)]` or a manual `TryFrom<Parameters>` trait impl to make it parsable. Receive it in your request handler using a `Route::data_form` (html form submission) or `Route::data_query` (query string) route type.

```rust
#[derive(TryFromParameters)]
struct Greeting {
    say: String,
    to: String,
    times: u8,
}
```

### Request Handling

Routes:
* `Route::file(Get, "/", "html/index.html")`: Respond to request with file contents
* `Route::func(Get, "/", route_base_func)` Respond to request with result of `route_base_func`
* `Route::data_form(Post, "/", route_data_func)` Respond to request with result of `route_data_func` - receives extra model param from form submission
* `Route::data_query(Get, "/", route_data_func)` Respond to request with result of `route_data_func` - receives extra model param from query string
* `Route::data_json(Post, "/", route_data_func)` Respond to request with result of `route_data_func` - receives extra model param from json request body

Request handler functions:
* `route_base_func:	Fn(&Request, PathParameters) -> Result<Response, StatusCode> + Send + Sync`
* `route_data_func<T>: Fn(&Request, PathParameters, T) -> Result<Response, StatusCode> + Send + Sync`
	* Used when you want to receive your own model struct `T`.

Request handler input:
```rust
pub struct Request {
    pub method: RequestMethod,
    pub resource: Resource,
    pub headers: Parameters,
    pub body: Body,
}

pub enum RequestMethod {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Patch,
    Unknown,
}

pub type Parameters = BTreeMap<String, String>;

pub struct Resource {
    pub path: String,
    pub query_params: Parameters,
}

pub enum Body {
    Text(String),
    FormData(Parameters),
    JsonData(Json),
}
```

#### Responses:
```rust
// Basic response
Ok(Response::new(StatusCode::Ok, body))

// Error response - this will be handled by `error_routes` if such a route exists
Err(StatusCode::NotFound)

// Templated response
Response::render_template(StatusCode::Ok, "html/greeting_result.html", &[("say", &greeting.say), ("to", &greeting.to)])
```

## Credits

* `Arc` and `Vec` are from [The Rustonomicon](https://doc.rust-lang.org/nightly/nomicon/)
* `ThreadPool` and `WebServer` were based on the final project of [The Rust Programming Language](https://doc.rust-lang.org/nightly/book/ch21-00-final-project-a-web-server.html)
