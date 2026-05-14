use std::collections::BTreeMap;

use crate::vec::Vec;

#[derive(Debug, PartialEq)]
pub enum Json {
    Object(BTreeMap<String, Json>),
    String(String),
    Double(f64),
    Boolean(bool),
    Array(Vec<Json>),
    Null,
}

pub struct JsonParser {
    buf: Vec<char>,
    i: usize,
    line: u32,
    beginning_line_i: usize,
    nesting: u32,
}

#[derive(Debug)]
pub struct ParseFailure {
    msg: String,
    line: u32,
    char: usize,
}

type Result<T> = std::result::Result<T, ParseFailure>;

impl Json {
    pub fn parse(json_str: &str) -> Result<Json> {
        let mut parser = JsonParser::new(json_str);
        let result = parser.parse_object();
        if result.is_ok() {
            assert!(
                parser.nesting == 0,
                "Bad state, successful result while still at nesting level {}",
                parser.nesting
            );
        }
        result
    }
}

impl JsonParser {
    pub fn new(json_str: &str) -> Self {
        JsonParser {
            buf: Vec::from_iter(json_str.chars()),
            i: 0,
            line: 1,
            beginning_line_i: 0,
            nesting: 0,
        }
    }

    fn drop_whitespace(&mut self) -> Result<()> {
        loop {
            let c = self.peek()?;
            if !c.is_whitespace() {
                break;
            }
            if c == '\n' {
                self.line += 1;
                self.beginning_line_i = self.i + 1;
            }

            self.eat_any();
        }
        Ok(())
    }

    #[inline]
    fn eat_any(&mut self) {
        self.i += 1;
    }

    fn eat(&mut self, expected: char) -> Result<()> {
        self.drop_whitespace()?;

        if self.peek()? == expected {
            self.eat_any();
            Ok(())
        } else {
            Err(ParseFailure::from(
                format!("Expected '{}', found '{}'", expected, self.peek()?),
                self,
            ))
        }
    }

    fn eat_literal(&mut self, expected_literal: &str) -> Result<()> {
        for expected in expected_literal.chars() {
            let actual = self.pop()?;
            if expected != actual {
                return Err(ParseFailure::from(
                    format!("Expected literal '{}', found '{}'", expected, actual),
                    self,
                ));
            }
        }
        Ok(())
    }

    fn peek(&mut self) -> Result<char> {
        if self.i < self.buf.len() {
            Ok(self.buf[self.i])
        } else {
            Err(ParseFailure::from(
                format!("Expected more input, still nested {} levels", self.nesting),
                self,
            ))
        }
    }

    fn pop(&mut self) -> Result<char> {
        let result = self.peek()?;
        self.eat_any();
        Ok(result)
    }

    fn parse_string(&mut self) -> Result<Json> {
        self.eat('"')?;

        let mut key = String::new();
        loop {
            let c = match self.pop()? {
                '\\' => self.pop()?,
                '"' => break,
                c => c,
            };
            key.push(c);
        }

        Ok(Json::String(key))
    }

    fn parse_boolean_true(&mut self) -> Result<Json> {
        self.eat_literal("true")?;
        Ok(Json::Boolean(true))
    }

    fn parse_boolean_false(&mut self) -> Result<Json> {
        self.eat_literal("false")?;
        Ok(Json::Boolean(false))
    }

    fn parse_null(&mut self) -> Result<Json> {
        self.eat_literal("null")?;
        Ok(Json::Null)
    }

    fn parse_scientific_notation(&mut self, dec_str: &mut String) -> Result<()> {
        dec_str.push('e');
        self.eat_any();

        let c = self.peek()?;
        if c == '+' || c == '-' {
            dec_str.push(c);
            self.eat_any();
        }

        loop {
            let c = self.peek()?;
            if c.is_ascii_digit() {
                dec_str.push(c);
                self.eat_any();
            } else {
                break;
            }
        }

        Ok(())
    }

    fn parse_double(&mut self) -> Result<Json> {
        let mut dec_str = String::new();

        match self.peek()? {
            '-' => {
                self.eat_any();
                dec_str.push('-');
            }
            '+' => self.eat_any(),
            _ => {}
        };

        loop {
            let c = self.peek()?;
            if c == '.' || c.is_ascii_digit() {
                dec_str.push(c);
                self.eat_any();
            } else {
                break;
            }
        }

        if self.peek()?.eq_ignore_ascii_case(&'e') {
            self.parse_scientific_notation(&mut dec_str)?;
        }

        match dec_str.parse::<f64>() {
            Ok(double) => Ok(Json::Double(double)),
            Err(_) => panic!("Failed to parse \"{}\" as double", dec_str),
        }
    }

    fn parse_key(&mut self) -> Result<String> {
        match self.parse_string() {
            Ok(Json::String(s)) => Ok(s),
            Ok(json) => panic!("Bad state, parsed {:?} instead of Json::String", json),
            Err(e) => Err(e),
        }
    }

    fn parse_value(&mut self) -> Result<Json> {
        match self.peek() {
            Ok('{') => self.parse_object(),
            Ok('"') => self.parse_string(),
            Ok('[') => self.parse_array(),
            Ok('t') => self.parse_boolean_true(),
            Ok('f') => self.parse_boolean_false(),
            Ok('n') => self.parse_null(),
            Ok('-') => self.parse_double(),
            Ok(digit) if digit.is_numeric() => self.parse_double(),
            Ok(c) => Err(ParseFailure::from(
                format!("Expected JSON value, found '{}'", c),
                self,
            )),
            Err(e) => Err(e),
        }
    }

    fn parse_array(&mut self) -> Result<Json> {
        let mut values = Vec::new();

        self.nesting += 1;
        self.eat('[')?;
        self.drop_whitespace()?;

        if self.peek()? != ']' {
            loop {
                self.drop_whitespace()?;
                let value = self.parse_value()?;

                values.push(value);

                self.drop_whitespace()?;
                if self.peek()? != ',' {
                    break;
                }
                self.eat(',')?;
            }
        }

        self.eat(']')?;
        self.nesting -= 1;

        Ok(Json::Array(values))
    }

    fn parse_object(&mut self) -> Result<Json> {
        let mut map = BTreeMap::new();

        self.nesting += 1;
        self.eat('{')?;
        self.drop_whitespace()?;

        if self.peek()? != '}' {
            loop {
                let key = &self.parse_key()?;

                self.eat(':')?;
                self.drop_whitespace()?;

                let value = self.parse_value()?;

                if map.insert(key.clone(), value).is_some() {
                    return Err(ParseFailure::from(
                        format!("Key \"{}\" is set on object more than once", key),
                        self,
                    ));
                }

                self.drop_whitespace()?;
                if self.peek()? != ',' {
                    break;
                }
                self.eat(',')?;
            }
        }

        self.eat('}')?;
        self.nesting -= 1;

        Ok(Json::Object(map))
    }
}

impl ParseFailure {
    pub fn from(msg: String, parser: &JsonParser) -> ParseFailure {
        ParseFailure {
            msg,
            line: parser.line,
            char: parser.i - parser.beginning_line_i,
        }
    }

    pub fn to_log(&self) -> String {
        format!(
            "JSON parse failure (L{}:{}) - {}",
            self.line, self.char, self.msg
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assertables::*;

    #[test]
    fn empty() {
        let json_str = "{}";

        let json = Json::parse(json_str);
        assert_eq!(json.unwrap(), Json::Object(BTreeMap::new()))
    }

    #[test]
    fn string() {
        let json_str = r#"{
			"key1": "string"
		}"#;

        let json = Json::parse(json_str);
        assert_eq!(
            json.unwrap(),
            Json::Object(BTreeMap::from([(
                "key1".to_string(),
                Json::String("string".to_string())
            )]))
        );
    }

    #[test]
    fn escaped() {
        let json_str = r#"{"key1": "
			\"test\""}"#;

        let json = Json::parse(&json_str);
        assert_eq!(
            json.unwrap(),
            Json::Object(BTreeMap::from([(
                "key1".to_string(),
                Json::String("\n\t\t\t\"test\"".to_string())
            )]))
        );
    }

    #[test]
    fn double() {
        let json_str = r#"{
			"key2": -2.5
		}"#;

        let json = Json::parse(json_str);
        assert_eq!(
            json.unwrap(),
            Json::Object(BTreeMap::from([("key2".to_string(), Json::Double(-2.5)),]))
        );
    }

    #[test]
    fn scientific_notation() {
        let json_str = r#"{
			"key2": [
                3.7e19,
                3.7e+19,
                3.7e-19,
                -3.7e-19
            ]
		}"#;

        let json = Json::parse(json_str);
        assert_eq!(
            json.unwrap(),
            Json::Object(BTreeMap::from([(
                "key2".to_string(),
                Json::Array(Vec::from([
                    Json::Double(3.7e19),
                    Json::Double(3.7e19),
                    Json::Double(3.7e-19),
                    Json::Double(-3.7e-19),
                ]))
            )]))
        );
    }

    #[test]
    fn boolean() {
        let json_str = r#"{
			"key3": true
		}"#;

        let json = Json::parse(json_str);
        assert_eq!(
            json.unwrap(),
            Json::Object(BTreeMap::from([("key3".to_string(), Json::Boolean(true))]))
        );
    }

    #[test]
    fn array() {
        let json_str = r#"{
			"key4":[
				1, null, true, "asd", {}, [1,2,3]
			]
		}"#;

        let json = Json::parse(json_str);
        assert_eq!(
            json.unwrap(),
            Json::Object(BTreeMap::from([(
                "key4".to_string(),
                Json::Array(Vec::from([
                    Json::Double(1.0),
                    Json::Null,
                    Json::Boolean(true),
                    Json::String("asd".to_string()),
                    Json::Object(BTreeMap::new()),
                    Json::Array(Vec::from([
                        Json::Double(1.0),
                        Json::Double(2.0),
                        Json::Double(3.0)
                    ]))
                ]))
            ),]))
        );
    }

    #[test]
    fn empty_object() {
        let json_str = r#"{
			"key5": {}
		}"#;

        let json = Json::parse(json_str);
        assert_eq!(
            json.unwrap(),
            Json::Object(BTreeMap::from([(
                "key5".to_string(),
                Json::Object(BTreeMap::new())
            ),]))
        );
    }

    #[test]
    fn object_with_multiple_fields() {
        let json_str = r#"{
			"key7.1": null,
			"key7.2": null,
			"key7.3": null
		}"#;

        let json = Json::parse(json_str);
        assert_eq!(
            json.unwrap(),
            Json::Object(BTreeMap::from([
                ("key7.1".to_string(), Json::Null),
                ("key7.2".to_string(), Json::Null),
                ("key7.3".to_string(), Json::Null),
            ]))
        );
    }

    #[test]
    fn nested_object() {
        let json_str = r#"{
			"key7": {
				"key7.1": null,
				"key7.2": -3
			}
		}"#;

        let json = Json::parse(json_str);
        assert_eq!(
            json.unwrap(),
            Json::Object(BTreeMap::from([(
                "key7".to_string(),
                Json::Object(BTreeMap::from([
                    ("key7.1".to_string(), Json::Null),
                    ("key7.2".to_string(), Json::Double(-3.0))
                ]))
            ),]))
        );
    }

    #[test]
    fn null() {
        let json_str = r#"{
			"key6": null
		}"#;

        let json = Json::parse(json_str);
        assert_eq!(
            json.unwrap(),
            Json::Object(BTreeMap::from([("key6".to_string(), Json::Null)]))
        );
    }

    #[test]
    fn complete_basic() {
        let json_str = r#"{
			"key1": "string",
			"key2": -2.5,
			"key3": true,
			"key4": [1, null, true, "asd", {}],
			"key5": {},
			"key6": null
		}"#;

        let json = Json::parse(json_str);
        assert_eq!(
            json.unwrap(),
            Json::Object(BTreeMap::from([
                ("key1".to_string(), Json::String("string".to_string())),
                ("key2".to_string(), Json::Double(-2.5)),
                ("key3".to_string(), Json::Boolean(true)),
                (
                    "key4".to_string(),
                    Json::Array(Vec::from([
                        Json::Double(1.0),
                        Json::Null,
                        Json::Boolean(true),
                        Json::String("asd".to_string()),
                        Json::Object(BTreeMap::new()),
                    ]))
                ),
                ("key5".to_string(), Json::Object(BTreeMap::new())),
                ("key6".to_string(), Json::Null)
            ]))
        );
    }

    #[test]
    fn double_key() {
        let json_str = r#"{
			"a": "b",
			"a": 3
		}"#;

        let json = &Json::parse(json_str);
        assert_contains!(json.as_ref().unwrap_err().msg, "Key \"a\"");
        assert_contains!(json.as_ref().unwrap_err().msg, "more than once")
    }

    #[test]
    fn complex() {
        let json_str = r###"{ "id":"test-0001",
"name"   :    "Messy JSON Parser Test" ,
  "version":3,"active":true,
"metadata":{
"created_at":"2026-05-13T09:45:12+10:00","updated_at":null,
"tags":[
"json","parser-test",
"",
"unicode-🔥",
"quotes-\"inside\"","slash\\/backslash\\\\test"
],
"weird keys":{ "":"empty key","   ":"spaces-only key",
"dot.key":"looks like a path",
"array[0]":"looks like an array accessor","true":"string key named true",
"null":"string key named null" } },
"users":[{"id":1,"name":"Alice",
"roles":[
"admin",
"editor","admin"],"preferences":{"theme":"dark","notifications":
{"email":true,
"sms":false,"push":null}},
"notes":"Line one\nLine two\nTabbed:\tvalue"},
{
"id":"2",
"name":"Bob \"The Builder\"",
"roles":[] ,
"preferences":{},"notes":""},
{       "id":3.14159,
"name":null,
"roles":["viewer",42,false,null,{"nestedRole":true}],
"preferences":{"numbers":[0,-0,1,-1,12345678901234567890,
0.000000000123,1.23e-10,-9.99e+99]}}],
"deeplyNested":{"level1":
{"level2":{"level3":
{"level4":{"level5":
{"value":"finally","emptyArray":[],"emptyObject":{},
"mixed":[{"a":[{"b":[{"c":"nested object in array in object in array"}]}]}]}}}}}},
"orders":[
{"orderId":"A-001",
"items":[{"sku":"ABC-123","qty":2,"price":19.99,
"discounts":[{"type":"percentage","value":10},{"type":"fixed","value":2.5}]},
{"sku":"XYZ-999","qty":0,"price":null,"discounts":[]}],
"shipping":{"address":{
"line1":"123 Example St",
"line2":"Unit \"B\"","city":"Melbourne","postcode":"3000","country":"AU"},
"instructions":"Leave at door unless raining.\nIf raining, call: +61 400 000 000"}}],
"escapedStrings":{"quote":"\"","backslash":"\\","newline":"\n",
"carriageReturn":"\r","tab":"\t","formFeed":"\f","backspace":"\b",
"unicode":"\u0048\u0065\u006c\u006c\u006f","emoji":"🧪🚀🐀"},
"matrix":[[1,2,3],
[true,false,null],["a","b",{"c":[4,5,6]}]],
"featureFlags":{
"new-parser":true,"legacy_parser":false,
"experimental.parser.v2":{"enabled":true,
"rollout":0.25,"rules":[{"country":"AU","percentage":50,
"conditions":{"minAge":18,"maxAge":null,
"segments":["beta","internal",""]}}]}},
"almostEmpty":{"array":[],"object":{},
"string":"","nullValue":null,"falseValue":false,"zero":0}}
    }
}"###;
        let json = &Json::parse(json_str);
        assert_ok!(json);
    }
}
