use std::collections::BTreeMap;

use derive_try_from::TryFromParameters;
use http_server::web_server::Parameters;

#[derive(TryFromParameters, Debug, PartialEq)]
struct Foo {
    bar: String,
    baz: u32,
    qux: bool,
}

#[test]
fn default() {
    let params: Parameters = BTreeMap::from([
        ("bar".to_string(), "bar".to_string()),
        ("baz".to_string(), "312".to_string()),
        ("qux".to_string(), "true".to_string()),
    ]);
    let foo = Foo::try_from(params).unwrap();
    assert_eq!(
        foo,
        Foo {
            bar: "bar".to_string(),
            baz: 312,
            qux: true,
        }
    )
}
