use crate::{IntegrationOSError, InternalError};
use handlebars::Handlebars;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

pub trait TemplateExt {
    fn render(&self, template: &str, data: Option<&Value>) -> Result<String, IntegrationOSError>;

    fn render_as<T>(&self, template: &T, data: Option<&Value>) -> Result<T, IntegrationOSError>
    where
        T: DeserializeOwned + Serialize;
}

#[derive(Debug, Clone)]
pub struct DefaultTemplate {
    template: Handlebars<'static>,
}

impl Default for DefaultTemplate {
    fn default() -> Self {
        Self {
            template: Handlebars::new(),
        }
    }
}

impl TemplateExt for DefaultTemplate {
    fn render(&self, template: &str, data: Option<&Value>) -> Result<String, IntegrationOSError> {
        self.template
            .render_template(template, &data.as_ref())
            .map_err(|e| {
                InternalError::serialize_error(&e.to_string(), Some("HandlebarTemplate::render"))
            })
    }

    fn render_as<T>(&self, template: &T, data: Option<&Value>) -> Result<T, IntegrationOSError>
    where
        T: Serialize + for<'de> serde::Deserialize<'de>,
    {
        let str = serde_json::to_string_pretty(template).map_err(|e| {
            InternalError::serialize_error(&e.to_string(), Some("HandlebarTemplate::render_as"))
        })?;

        let rendered = self.render(&str, data)?;

        serde_json::from_str(&rendered).map_err(|e| {
            InternalError::serialize_error(&e.to_string(), Some("HandlebarTemplate::render_as"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use serde_json::Map;

    #[test]
    fn test_render_template() {
        let template = DefaultTemplate::default();
        let data = Some(Value::Object(Map::new()));
        let result = template.render("{{hello}}", data.as_ref());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");

        let mut data = Map::new();
        data.insert("hello".to_string(), Value::String("world".to_string()));
        let data = Some(Value::Object(data));

        let result = template.render("{{hello}}", data.as_ref());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "world");
    }

    #[test]
    fn test_multiple_passes_on_template() {
        let template = DefaultTemplate::default();
        let data = Some(Value::Object(Map::new()));
        let result = template.render("{{hello}} \\{{world}}", data.as_ref());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), " {{world}}");
    }

    #[test]
    fn test_render_as() {
        #[derive(Debug, Serialize, Deserialize)]
        struct Test {
            hello: String,
        }

        let template = DefaultTemplate::default();
        let data = Some(Value::Object(Map::new()));
        let result = template.render_as(
            &Test {
                hello: "world".to_string(),
            },
            data.as_ref(),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().hello, "world");

        let mut data = Map::new();
        data.insert("hello".to_string(), Value::String("world".to_string()));
        let data = Some(Value::Object(data));

        let result = template.render_as(
            &Test {
                hello: "{{hello}}".to_string(),
            },
            data.as_ref(),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().hello, "world");
    }

    #[test]
    fn test_render_as_with_escape() {
        #[derive(Debug, Serialize, Deserialize)]
        struct Test {
            hello: String,
        }

        let template = DefaultTemplate::default();
        let data = Some(Value::Object(Map::from_iter(vec![(
            "hello".to_string(),
            Value::String("world".to_string()),
        )])));

        let result = template.render_as(
            &Test {
                hello: "{{{{raw}}}}{{hello}}{{{{/raw}}}}".to_string(),
            },
            data.as_ref(),
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap().hello, "{{hello}}");
    }
}
