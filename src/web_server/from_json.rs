use crate::vec::Vec;
use crate::web_server::Json;

pub trait FromJson: Sized {
    fn from_json(json: Json) -> Option<Self>;
}

impl FromJson for Json {
    fn from_json(json: Json) -> Option<Self> {
        Some(json)
    }
}

impl FromJson for u32 {
    fn from_json(json: Json) -> Option<Self> {
        match json {
            Json::Double(double) => Some(double as u32),
            _ => None,
        }
    }
}

impl FromJson for String {
    fn from_json(json: Json) -> Option<Self> {
        match json {
            Json::String(string) => Some(string),
            _ => None,
        }
    }
}

impl FromJson for bool {
    fn from_json(json: Json) -> Option<Self> {
        match json {
            Json::Boolean(boolean) => Some(boolean),
            _ => None,
        }
    }
}

impl<T: FromJson> FromJson for Vec<T> {
    fn from_json(json: Json) -> Option<Self> {
        match json {
            Json::Array(vec) => {
                let mut result = Vec::with_capacity(vec.len());
                for v in vec.into_iter() {
                    match T::from_json(v) {
                        Some(t) => result.push(t),
                        None => return None,
                    };
                }
                Some(result)
            }
            _ => None,
        }
    }
}
