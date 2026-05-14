use std::{borrow::Cow, collections::BTreeMap, fs, str::Split};

use crate::vec::Vec;
use crate::web_server::Body;
use crate::web_server::{Request, RequestMethod, Response, StatusCode, request::Parameters};

pub struct Route {
    method: RequestMethod,
    path_pattern: PathPattern,
    response_type: ResponseType,
}

pub struct PathPattern {
    components: Vec<PathComponent>,
}

pub enum PathComponent {
    Literal(String),
    Variable(String),
}

pub struct ErrorRoute {
    status_code: StatusCode,
    response_type: ResponseType,
}

pub enum ResponseType {
    File(Cow<'static, str>),
    Function(Box<RequestProcessorFn>),
}

type RequestProcessorFn =
    dyn Fn(&Request, Parameters) -> Result<Response, StatusCode> + Send + Sync;

impl Route {
    fn new(method: RequestMethod, path_pattern: PathPattern, response_type: ResponseType) -> Self {
        Route {
            method,
            path_pattern,
            response_type,
        }
    }

    pub fn file(method: RequestMethod, path_pattern: &str, file_path: &'static str) -> Self {
        Route::new(
            method,
            PathPattern::new(path_pattern),
            ResponseType::new_file(file_path),
        )
    }

    pub fn func<F>(method: RequestMethod, path_pattern: &str, function: F) -> Self
    where
        F: Fn(&Request, Parameters) -> Result<Response, StatusCode> + Send + Sync + 'static,
    {
        Route::new(
            method,
            PathPattern::new(path_pattern),
            ResponseType::new_func(function),
        )
    }

    pub fn data_form<T: TryFrom<Parameters>, F>(
        method: RequestMethod,
        path_pattern: &str,
        function: F,
    ) -> Self
    where
        F: Fn(&Request, Parameters, T) -> Result<Response, StatusCode> + Send + Sync + 'static,
    {
        let user_function = Box::new(function);

        Route::new(
            method,
            PathPattern::new(path_pattern),
            ResponseType::new_func(
                move |request: &Request, path_params: Parameters| match &request.body {
                    Body::FormData(params) => match T::try_from(params.clone()) {
                        Ok(model) => user_function(request, path_params, model),
                        Err(_) => Err(StatusCode::BadRequest),
                    },
                    _ => Err(StatusCode::UnsupportedMediaType),
                },
            ),
        )
    }

    pub fn matches_path(&self, path: &str) -> bool {
        self.path_pattern.matches(path)
    }

    pub fn matches_method(&self, method: RequestMethod) -> bool {
        self.method == method
    }

    pub fn to_response(&self, request: &Request) -> Result<Response, StatusCode> {
        let path_params = self.path_pattern.get_path_params(&request.resource.path);
        self.response_type.to_response(request, path_params)
    }
}

impl PathPattern {
    pub fn new(pattern: &str) -> Self {
        assert!(
            pattern.starts_with("/"),
            "Path pattern {} does not start with /",
            pattern
        );
        PathPattern {
            components: Self::parse_components(pattern),
        }
    }

    fn split_components(path: &str) -> Split<'_, char> {
        assert!(
            path.starts_with('/'),
            "Path {} doesn't start with / - was not validated properly",
            path
        );
        path[1..].split('/')
    }

    fn parse_components(pattern: &str) -> Vec<PathComponent> {
        if pattern == "/" {
            return Vec::new();
        }

        let mut components = Vec::new();
        for component in Self::split_components(pattern) {
            match component.chars().nth(0) {
                None => panic!("Path pattern {} has empty component", pattern),
                Some('{') => {
                    assert!(
                        component.ends_with('}'),
                        "Path pattern {} has no closing brace in component {}",
                        pattern,
                        component
                    );
                    let var_name = &component[1..component.chars().count() - 1];
                    components.push(PathComponent::Variable(String::from(var_name)));
                }
                Some(_) => {
                    components.push(PathComponent::Literal(String::from(component)));
                }
            }
        }
        components
    }

    fn matches(&self, path: &str) -> bool {
        if !path.starts_with('/') {
            return false;
        }

        let components = Self::split_components(path).collect::<Vec<&str>>();

        if self.components.len() != components.len() {
            return false;
        }

        self.components
            .iter()
            .zip(components)
            .all(|(mine, theirs)| match (mine, theirs) {
                (PathComponent::Variable(_), value) => !value.is_empty(),
                (PathComponent::Literal(a), b) => a == b,
            })
    }

    fn get_path_params(&self, path: &str) -> Parameters {
        let mut params = BTreeMap::new();
        let components = Self::split_components(path);

        for (mine, theirs) in self.components.iter().zip(components) {
            match (mine, theirs) {
                (PathComponent::Variable(key), value) => {
                    params.insert(key.clone(), String::from(value));
                }
                (PathComponent::Literal(a), b) => assert!(
                    *a == *b,
                    "PathPattern.get_path_params(path) called with invalid path. Check PathPattern.matches(path) first"
                ),
            };
        }

        params
    }
}

impl ErrorRoute {
    fn new(status_code: StatusCode, response_type: ResponseType) -> Self {
        ErrorRoute {
            status_code,
            response_type,
        }
    }

    pub fn file(status_code: StatusCode, file_path: &'static str) -> Self {
        Self::new(status_code, ResponseType::new_file(file_path))
    }

    pub fn file_dynamic(status_code: StatusCode, file_path: String) -> Self {
        Self::new(status_code, ResponseType::new_file_dynamic(file_path))
    }

    pub fn function<F>(status_code: StatusCode, function: F) -> Self
    where
        F: Fn(&Request, Parameters) -> Result<Response, StatusCode> + Send + Sync + 'static,
    {
        Self::new(status_code, ResponseType::new_func(function))
    }

    pub fn matches(&self, status_code: StatusCode) -> bool {
        self.status_code == status_code
    }

    pub fn to_response(&self, request: &Request) -> Result<Response, StatusCode> {
        self.response_type.to_response(request, BTreeMap::new())
    }
}

impl ResponseType {
    pub fn new_file(path: &'static str) -> ResponseType {
        ResponseType::File(Cow::Borrowed(path))
    }

    pub fn new_file_dynamic(path: String) -> ResponseType {
        ResponseType::File(Cow::Owned(path))
    }

    pub fn new_func<F>(function: F) -> ResponseType
    where
        F: Fn(&Request, Parameters) -> Result<Response, StatusCode> + Send + Sync + 'static,
    {
        ResponseType::Function(Box::new(function))
    }

    pub fn to_response(
        &self,
        request: &Request,
        path_params: Parameters,
    ) -> Result<Response, StatusCode> {
        match self {
            ResponseType::File(path) => self.file_response(path),
            ResponseType::Function(f) => f(request, path_params),
        }
    }

    fn file_response(&self, path: &str) -> Result<Response, StatusCode> {
        match fs::read_to_string(path) {
            Ok(response_body) => Ok(Response::new(StatusCode::Ok, response_body)),
            Err(e) => {
                eprintln!("Error: Cannot read file {}, error: {:?}", path, e);
                Err(StatusCode::NotFound)
            }
        }
    }
}
