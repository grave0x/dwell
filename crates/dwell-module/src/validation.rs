use dwell_core::traits::{Module, OptionType};
use dwell_core::Result;

/// Validates user-provided option values against a module's option schema.
pub struct Validator;

impl Default for Validator {
    fn default() -> Self { Self::new() }
}

impl Validator {
    pub fn new() -> Self { Validator }

    /// Validate a set of user values against a module's declared options.
    /// Returns Ok(()) if all values pass, or an error listing the first problem.
    pub fn validate(&self, module: &dyn Module, values: &serde_json::Value) -> Result<()> {
        let options = module.options();
        let obj = match values.as_object() {
            Some(o) => o,
            None => return Err("Module values must be a JSON object".into()),
        };

        for opt in &options {
            let user_value = obj.get(&opt.name);

            // Check required fields
            if opt.required && user_value.is_none() {
                return Err(dwell_core::DwellError::Validation(format!(
                    "{}: required option '{}' is missing", module.id(), opt.name
                )));
            }

            // Validate type if a value was provided
            if let Some(val) = user_value {
                self.validate_type(&opt.name, &opt.option_type, val)?;
            }
        }

        Ok(())
    }

    fn validate_type(&self, name: &str, opt_type: &OptionType, value: &serde_json::Value) -> Result<()> {
        match opt_type {
            OptionType::String => {
                if !value.is_string() {
                    return Err(dwell_core::DwellError::Validation(
                        format!("'{}': expected string, got {}", name, value.kind())
                    ));
                }
            }
            OptionType::Integer => {
                if !value.is_number() {
                    return Err(dwell_core::DwellError::Validation(
                        format!("'{}': expected integer, got {}", name, value.kind())
                    ));
                }
            }
            OptionType::Float => {
                if !value.is_number() {
                    return Err(dwell_core::DwellError::Validation(
                        format!("'{}': expected number, got {}", name, value.kind())
                    ));
                }
            }
            OptionType::Boolean => {
                if !value.is_boolean() {
                    return Err(dwell_core::DwellError::Validation(
                        format!("'{}': expected boolean, got {}", name, value.kind())
                    ));
                }
            }
            OptionType::Path => {
                if !value.is_string() {
                    return Err(dwell_core::DwellError::Validation(
                        format!("'{}': expected path (string), got {}", name, value.kind())
                    ));
                }
            }
            OptionType::List(inner) => {
                if let Some(arr) = value.as_array() {
                    for (i, item) in arr.iter().enumerate() {
                        self.validate_type(&format!("{}[{}]", name, i), inner, item)?;
                    }
                } else {
                    return Err(dwell_core::DwellError::Validation(
                        format!("'{}': expected array, got {}", name, value.kind())
                    ));
                }
            }
            OptionType::Dict(_inner) => {
                if !value.is_object() {
                    return Err(dwell_core::DwellError::Validation(
                        format!("'{}': expected object/dict, got {}", name, value.kind())
                    ));
                }
            }
            OptionType::Enum(valid) => {
                if let Some(s) = value.as_str() {
                    if !valid.contains(&s.to_string()) {
                        return Err(dwell_core::DwellError::Validation(
                            format!("'{}': invalid value '{}'. Expected one of: {}", name, s, valid.join(", "))
                        ));
                    }
                } else {
                    return Err(dwell_core::DwellError::Validation(
                        format!("'{}': expected string (enum), got {}", name, value.kind())
                    ));
                }
            }
            OptionType::SubModule => {
                if !value.is_object() {
                    return Err(dwell_core::DwellError::Validation(
                        format!("'{}': expected object for sub-module, got {}", name, value.kind())
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Helper to get a human-readable type name from serde_json::Value.
trait ValueKind {
    fn kind(&self) -> &'static str;
}

impl ValueKind for serde_json::Value {
    fn kind(&self) -> &'static str {
        match self {
            serde_json::Value::Null => "null",
            serde_json::Value::Bool(_) => "boolean",
            serde_json::Value::Number(_) => "number",
            serde_json::Value::String(_) => "string",
            serde_json::Value::Array(_) => "array",
            serde_json::Value::Object(_) => "object",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwell_core::traits::{Module, ModuleOption, OptionType};
    use dwell_core::Result;

    struct TestModule;
    impl Module for TestModule {
        fn id(&self) -> &str { "test" }
        fn description(&self) -> &str { "test module" }
        fn options(&self) -> Vec<ModuleOption> {
            vec![
                ModuleOption {
                    name: "name".into(),
                    option_type: OptionType::String,
                    default: None,
                    description: "Name".into(),
                    example: None,
                    required: true,
                },
                ModuleOption {
                    name: "count".into(),
                    option_type: OptionType::Integer,
                    default: Some(serde_json::json!(1)),
                    description: "Count".into(),
                    example: None,
                    required: false,
                },
            ]
        }
        fn generate(&self, _values: &serde_json::Value) -> Result<Vec<dwell_core::Entry>> {
            Ok(vec![])
        }
    }

    #[test]
    fn test_valid_required_field() {
        let v = Validator::new();
        let m = TestModule;
        let values = serde_json::json!({"name": "test"});
        assert!(v.validate(&m, &values).is_ok());
    }

    #[test]
    fn test_missing_required_field() {
        let v = Validator::new();
        let m = TestModule;
        let values = serde_json::json!({});
        assert!(v.validate(&m, &values).is_err());
    }

    #[test]
    fn test_wrong_type() {
        let v = Validator::new();
        let m = TestModule;
        let values = serde_json::json!({"name": 42});
        assert!(v.validate(&m, &values).is_err());
    }

    #[test]
    fn test_enum_validation() {
        let v = Validator::new();
        let values = serde_json::json!({"color": "red"});
        let opt = ModuleOption {
            name: "color".into(),
            option_type: OptionType::Enum(vec!["red".into(), "blue".into()]),
            default: None,
            description: "Color".into(),
            example: None,
            required: true,
        };
        // Create a temporary module that returns this option
        struct ColorModule;
        impl Module for ColorModule {
            fn id(&self) -> &str { "color" }
            fn description(&self) -> &str { "color" }
            fn options(&self) -> Vec<ModuleOption> {
                vec![ModuleOption {
                    name: "color".into(),
                    option_type: OptionType::Enum(vec!["red".into(), "blue".into()]),
                    default: None,
                    description: "Color".into(),
                    example: None,
                    required: true,
                }]
            }
            fn generate(&self, _values: &serde_json::Value) -> Result<Vec<dwell_core::Entry>> {
                Ok(vec![])
            }
        }
        let m = ColorModule;
        assert!(v.validate(&m, &values).is_ok(), "valid color should pass");
        let bad = serde_json::json!({"color": "green"});
        assert!(v.validate(&m, &bad).is_err(), "invalid color should fail");
    }
}
