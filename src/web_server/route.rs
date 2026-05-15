use std::{borrow::Cow, collections::BTreeMap, fs, str::Split};

use crate::vec::Vec;
use crate::web_server::{Body, FromJson};
use crate::web_server::{Request, RequestMethod, Response, StatusCode, request::Parameters};

pub struct Route {
    method: RequestMethod,
    path_pattern: PathPattern,
    response_type: ResponseType,
}

pub struct ErrorRoute {
    status_code: StatusCode,
    response_type: ResponseType,
}

struct PathPattern {
    components: Vec<PathComponent>,
}

enum PathComponent {
    Literal(String),
    Variable(String),
}

enum ResponseType {
    File(Cow<'static, str>),
    Function(Box<RequestProcessorFn>),
}

pub type PathParameters<'a, 'b> = BTreeMap<&'a str, &'b str>;

type RequestProcessorFn =
    dyn Fn(&Request, PathParameters) -> Result<Response, StatusCode> + Send + Sync;

impl Route {
    fn new(method: RequestMethod, path_pattern: &str, response_type: ResponseType) -> Self {
        Route {
            method,
            path_pattern: PathPattern::new(path_pattern),
            response_type,
        }
    }

    pub fn file(method: RequestMethod, path_pattern: &str, file_path: &'static str) -> Self {
        Route::new(
            method,
            path_pattern,
            ResponseType::File(Cow::Borrowed(file_path)),
        )
    }

    pub fn file_dynamic(method: RequestMethod, path_pattern: &str, file_path: String) -> Self {
        Route::new(
            method,
            path_pattern,
            ResponseType::File(Cow::Owned(file_path)),
        )
    }

    fn func_boxed(
        method: RequestMethod,
        path_pattern: &str,
        function: Box<RequestProcessorFn>,
    ) -> Self {
        Route::new(method, path_pattern, ResponseType::Function(function))
    }

    pub fn func<F>(method: RequestMethod, path_pattern: &str, function: F) -> Self
    where
        F: Fn(&Request, PathParameters) -> Result<Response, StatusCode> + Send + Sync + 'static,
    {
        Route::func_boxed(method, path_pattern, Box::new(function))
    }

    pub fn data_form<T: TryFrom<Parameters>, F>(
        method: RequestMethod,
        path_pattern: &str,
        function: F,
    ) -> Self
    where
        F: Fn(&Request, PathParameters, T) -> Result<Response, StatusCode> + Send + Sync + 'static,
    {
        let wrapper: Box<RequestProcessorFn> =
            Box::new(move |request, path_params| match request.body {
                Body::FormData(ref params) => match T::try_from(params.clone()) {
                    Ok(model) => function(request, path_params, model),
                    Err(_) => Err(StatusCode::BadRequest),
                },
                _ => Err(StatusCode::UnsupportedMediaType),
            });

        Route::func_boxed(method, path_pattern, wrapper)
    }

    pub fn data_query<T: TryFrom<Parameters>, F>(
        method: RequestMethod,
        path_pattern: &str,
        function: F,
    ) -> Self
    where
        F: Fn(&Request, PathParameters, T) -> Result<Response, StatusCode> + Send + Sync + 'static,
    {
        let wrapper: Box<RequestProcessorFn> = Box::new(move |request, path_params| {
            let data = request.resource.query_params.clone();
            match T::try_from(data) {
                Ok(model) => function(request, path_params, model),
                Err(_) => Err(StatusCode::BadRequest),
            }
        });

        Route::func_boxed(method, path_pattern, wrapper)
    }

    pub fn data_json<T: FromJson, F>(method: RequestMethod, path_pattern: &str, function: F) -> Self
    where
        F: Fn(&Request, PathParameters, T) -> Result<Response, StatusCode> + Send + Sync + 'static,
    {
        let wrapper: Box<RequestProcessorFn> =
            Box::new(move |request, path_params| match request.body {
                Body::JsonData(ref json) => match T::from_json(json.clone()).ok_or(()) {
                    Ok(model) => function(request, path_params, model),
                    Err(_) => Err(StatusCode::BadRequest),
                },
                _ => Err(StatusCode::UnsupportedMediaType),
            });

        Route::func_boxed(method, path_pattern, wrapper)
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

impl ErrorRoute {
    fn new(status_code: StatusCode, response_type: ResponseType) -> Self {
        ErrorRoute {
            status_code,
            response_type,
        }
    }

    pub fn file(status_code: StatusCode, file_path: &'static str) -> Self {
        Self::new(status_code, ResponseType::File(Cow::Borrowed(file_path)))
    }

    pub fn file_dynamic(status_code: StatusCode, file_path: String) -> Self {
        Self::new(status_code, ResponseType::File(Cow::Owned(file_path)))
    }

    pub fn func<F>(status_code: StatusCode, function: F) -> Self
    where
        F: Fn(&Request, PathParameters) -> Result<Response, StatusCode> + Send + Sync + 'static,
    {
        Self::new(status_code, ResponseType::Function(Box::new(function)))
    }

    pub fn matches(&self, status_code: StatusCode) -> bool {
        self.status_code == status_code
    }

    pub fn to_response(&self, request: &Request) -> Result<Response, StatusCode> {
        let mut response = self.response_type.to_response(request, BTreeMap::new())?;
        response.status_code = self.status_code;
        Ok(response)
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

        let path_components = Self::split_components(path).collect::<Vec<&str>>();

        if self.components.len() != path_components.len() {
            return false;
        }

        self.components
            .iter()
            .zip(path_components)
            .all(|(mine, theirs)| match (mine, theirs) {
                (PathComponent::Variable(_), value) => !value.is_empty(),
                (PathComponent::Literal(a), b) => a == b,
            })
    }

    fn get_path_params<'a, 'b>(&'a self, path: &'b str) -> PathParameters<'a, 'b> {
        let mut params = BTreeMap::new();
        let path_components = Self::split_components(path);

        for (mine, theirs) in self.components.iter().zip(path_components) {
            match (mine, theirs) {
                (PathComponent::Variable(key), value) => {
                    params.insert(key.as_str(), value);
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

impl ResponseType {
    fn to_response(
        &self,
        request: &Request,
        path_params: PathParameters,
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
