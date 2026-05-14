use std::collections::BTreeMap;

use http_server::web_server::Parameters;
use derive_try_from::TryFromParameters;

#[derive(TryFromParameters, Debug, PartialEq)]
struct Foo {
    bar: String,
    baz: String,
}

#[test]
fn default() {
    let params: Parameters = BTreeMap::from([
        ("bar".to_string(), "bar".to_string()),
        ("baz".to_string(), "baz".to_string()),
    ]);
    let foo = Foo::try_from(params).unwrap();
    assert_eq!(
        foo,
        Foo {
            bar: "bar".to_string(),
            baz: "baz".to_string()
        }
    )
}
