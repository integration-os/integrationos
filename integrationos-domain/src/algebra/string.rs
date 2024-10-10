pub trait StringExt {
    fn capitalize(&self) -> String;
    fn snake_case(&self) -> String;
    fn camel_case(&self) -> String;
    fn pascal_case(&self) -> String;
    fn kebab_case(&self) -> String {
        self.snake_case().replace('_', "-")
    }
}

impl StringExt for String {
    fn capitalize(&self) -> String {
        if let Some(first_char) = self.chars().next() {
            let capitalized = first_char
                .to_uppercase()
                .chain(self.chars().skip(1))
                .collect();
            capitalized
        } else {
            String::new()
        }
    }

    fn snake_case(&self) -> String {
        let mut snake_case = String::new();

        for (i, c) in self.chars().enumerate() {
            if c.is_ascii_uppercase() && i > 0 {
                snake_case.push('_');
                snake_case.push(c.to_ascii_lowercase());
            } else {
                snake_case.push(c.to_ascii_lowercase());
            }
        }

        snake_case
    }

    fn camel_case(&self) -> String {
        let mut camel_case = String::new();
        let mut capitalize_next = false;

        for c in self.chars() {
            if c.is_alphanumeric() {
                if capitalize_next {
                    camel_case.push(c.to_uppercase().next().unwrap());
                    capitalize_next = false;
                } else {
                    camel_case.push(c);
                }
            } else {
                capitalize_next = true;
            }
        }

        camel_case
    }

    fn pascal_case(&self) -> String {
        let mut pascal_case = String::new();
        let mut capitalize_next = true;

        for c in self.chars() {
            if c.is_alphanumeric() {
                if capitalize_next {
                    pascal_case.push(c.to_uppercase().next().unwrap());
                    capitalize_next = false;
                } else {
                    pascal_case.push(c);
                }
            } else {
                capitalize_next = true;
            }
        }

        pascal_case
    }
}

impl<'a> StringExt for &'a str {
    fn capitalize(&self) -> String {
        self.to_string().capitalize()
    }

    fn snake_case(&self) -> String {
        self.to_string().snake_case()
    }

    fn camel_case(&self) -> String {
        self.to_string().camel_case()
    }

    fn pascal_case(&self) -> String {
        self.to_string().pascal_case()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capitalize() {
        assert_eq!("".capitalize(), "");
        assert_eq!("a".capitalize(), "A");
        assert_eq!("A".capitalize(), "A");
        assert_eq!("hello".capitalize(), "Hello");
        assert_eq!("Hello".capitalize(), "Hello");
        assert_eq!(
            "really complicated and long string".capitalize(),
            "Really complicated and long string"
        );
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!("".snake_case(), "");
        assert_eq!("a".snake_case(), "a");
        assert_eq!("A".snake_case(), "a");
        assert_eq!("hello".snake_case(), "hello");
        assert_eq!("Hello".snake_case(), "hello");
        assert_eq!("HelloWorld".snake_case(), "hello_world");
        assert_eq!("HelloWorldAgain".snake_case(), "hello_world_again");
        assert_eq!(
            "ReallyComplicatedAndLongString".snake_case(),
            "really_complicated_and_long_string"
        );
    }

    #[test]
    fn test_to_camel_case() {
        assert_eq!("".camel_case(), "");
        assert_eq!("a".camel_case(), "a");
        assert_eq!("A".camel_case(), "A");
        assert_eq!("hello".camel_case(), "hello");
        assert_eq!("Hello".camel_case(), "Hello");
        assert_eq!("hello_world".camel_case(), "helloWorld");
        assert_eq!("hello_world_again".camel_case(), "helloWorldAgain");
        assert_eq!(
            "really_complicated_and_long_string".camel_case(),
            "reallyComplicatedAndLongString"
        );
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!("".pascal_case(), "");
        assert_eq!("a".pascal_case(), "A");
        assert_eq!("A".pascal_case(), "A");
        assert_eq!("hello".pascal_case(), "Hello");
        assert_eq!("Hello".pascal_case(), "Hello");
        assert_eq!("hello_world".pascal_case(), "HelloWorld");
        assert_eq!("hello_world_again".pascal_case(), "HelloWorldAgain");
        assert_eq!(
            "really_complicated_and_long_string".pascal_case(),
            "ReallyComplicatedAndLongString"
        );
    }

    #[test]
    fn test_to_kebab_case() {
        assert_eq!("".kebab_case(), "");
        assert_eq!("a".kebab_case(), "a");
        assert_eq!("A".kebab_case(), "a");
        assert_eq!("hello".kebab_case(), "hello");
        assert_eq!("Hello".kebab_case(), "hello");
        assert_eq!("hello_world".kebab_case(), "hello-world");
        assert_eq!("hello_world_again".kebab_case(), "hello-world-again");
        assert_eq!(
            "really_complicated_and_long_string".kebab_case(),
            "really-complicated-and-long-string"
        );
    }
}
