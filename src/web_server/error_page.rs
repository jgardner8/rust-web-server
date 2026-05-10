use crate::web_server::{Request, Response, StatusLine, request_pattern::ResponseType};

pub struct ErrorPage {
    status_code: u16,
    response_type: ResponseType,
}

impl ErrorPage {
    fn new(status_code: u16, response_type: ResponseType) -> Self {
        ErrorPage {
            status_code,
            response_type,
        }
    }

    pub fn file(status_code: u16, file_path: &'static str) -> Self {
        Self::new(status_code, ResponseType::new_file(file_path))
    }

    pub fn file_dynamic(status_code: u16, file_path: String) -> Self {
        Self::new(status_code, ResponseType::new_file_dynamic(file_path))
    }

    pub fn function<F>(status_code: u16, function: F) -> Self
    where
        F: Fn(&Request) -> Result<Response, StatusLine> + Send + Sync + 'static,
    {
        Self::new(status_code, ResponseType::new_func(function))
    }

    pub fn matches(&self, status_code: u16) -> bool {
        self.status_code == status_code
    }

    pub fn to_response(&self, request: &Request) -> Result<Response, StatusLine> {
        self.response_type.to_response(request)
    }
}
