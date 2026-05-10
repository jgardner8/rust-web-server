#[derive(Copy, Clone, PartialEq)]
pub enum StatusCode {
    // Ref: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Status
    Ok = 200,
    Created = 201,
    MovedPermanently = 301,
    Found = 302,
    BadRequest = 400,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    LengthRequired = 411,
    ContentTooLarge = 413,
    URITooLong = 414,
    TooManyRequests = 429,
    RequestHeaderFieldsTooLarge = 431,
    InternalServerError = 500,
    NotImplemented = 501,
    BadGateway = 502,
    GatewayTimeout = 504,
    HttpVersionNotSupported = 505,
}

pub struct Response {
    pub status_code: StatusCode,
    body: String,
}

impl StatusCode {
    pub fn reason_phrase(&self) -> &'static str {
        match *self as u16 {
            200 => "OK",
            201 => "Created",
            301 => "Moved Permanently",
            302 => "Found",
            400 => "Bad Request",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            411 => "Length Required",
            413 => "Content Too Large",
            414 => "URI Too Long",
            429 => "Too Many Requests",
            431 => "Request Header Fields Too Large",
            500 => "Internal Server Error",
            501 => "Not Implemented",
            502 => "Bad Gateway",
            504 => "Gateway Timeout",
            505 => "HTTP Version Not Supported",
            _ => "",
        }
    }

    pub fn encode_http_str(&self) -> String {
        format!("HTTP/1.1 {} {}", *self as u16, self.reason_phrase())
    }
}

impl Response {
    pub fn new(status_code: StatusCode, body: String) -> Self {
        Response {
            status_code,
            body,
        }
    }

    pub fn encode_http_str(&self) -> String {
        format!(
            concat!(
                "{}\r\n",
                "Content-Length: {}\r\n",
                "Content-Type: text/html; charset=utf-8\r\n",
                "Connection: Closed\r\n",
                "\r\n",
                "{}"
            ),
            self.status_code.encode_http_str(),
            self.body.len(),
            self.body
        )
    }

    pub fn to_log(&self) -> String {
        self.status_code.encode_http_str()
    }
}

impl From<StatusCode> for Response {
    fn from(status_code: StatusCode) -> Self {
        let body = String::from(status_code.reason_phrase());
        Response { status_code, body }
    }
}
