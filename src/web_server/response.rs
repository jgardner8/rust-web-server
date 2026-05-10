pub struct StatusLine {
    pub code: u16,
}

pub struct Response {
    pub status_line: StatusLine,
    body: String,
}

impl StatusLine {
    pub fn new(code: u16) -> Self {
        StatusLine { code }
    }

    pub fn reason_phrase(&self) -> &'static str {
        // Ref: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Status
        match self.code {
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
        format!("HTTP/1.1 {} {}", self.code, self.reason_phrase())
    }
}

impl Response {
    pub fn new(status_code: u16, body: String) -> Self {
        Response {
            status_line: StatusLine::new(status_code),
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
            self.status_line.encode_http_str(),
            self.body.len(),
            self.body
        )
    }

    pub fn to_log(&self) -> String {
        self.status_line.encode_http_str()
    }
}

impl From<StatusLine> for Response {
    fn from(status_line: StatusLine) -> Self {
        let body = String::from(status_line.reason_phrase());
        Response { status_line, body }
    }
}
