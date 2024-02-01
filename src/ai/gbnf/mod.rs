extern crate proc_macro;

use auto_impl::auto_impl;
use itertools::Itertools;

mod events;

// Actual GBNF rule itself. Holds rule text for dedup.
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct GbnfRule {
    name: String,
    text: String,
}

impl GbnfRule {
    pub fn new(token: String, rule_text: String) -> GbnfRule {
        GbnfRule {
            name: token,
            text: rule_text,
        }
    }

    pub fn single(token: String, rule_text: String) -> Vec<GbnfRule> {
        vec![GbnfRule::new(token, rule_text)]
    }
}

//token() returns the gbnf identifier for the rule.
//rule() returns the rule itself.
#[auto_impl(&, Box)]
pub trait TokenAndRule {
    fn rules(&self) -> Vec<GbnfRule>;
    fn token(&self) -> String;
}

pub enum GbnfToken {
    Space,
}

impl GbnfToken {
    pub(self) const SPACE: &'static str = r#"[ \t\n]*"#;
}

impl TokenAndRule for GbnfToken {
    fn rules(&self) -> Vec<GbnfRule> {
        match self {
            Self::Space => GbnfRule::single(self.token(), Self::SPACE.to_string()),
        }
    }

    fn token(&self) -> String {
        match self {
            Self::Space => "ws".to_string(),
        }
    }
}

#[derive(Debug)]
pub enum GbnfPrimitive {
    String,
    Boolean,
    Number,
}

impl GbnfPrimitive {
    pub(self) const STRING: &'static str = r#""\""   ([^"]*)   "\""#;
    pub(self) const BOOLEAN: &'static str = r#""true" | "false""#;
    pub(self) const NUMBER: &'static str = r#"[0-9]+   "."?   [0-9]*"#;
}

impl TokenAndRule for GbnfPrimitive {
    /// Output the raw GBNF rule of this primitive.
    fn rules(&self) -> Vec<GbnfRule> {
        let rule_text = match self {
            Self::Boolean => Self::BOOLEAN,
            Self::Number => Self::NUMBER,
            Self::String => Self::STRING,
        };

        GbnfRule::single(self.token(), rule_text.to_string())
    }

    /// Output the token name of the GBNF rule (to refer to in other
    /// rules).
    fn token(&self) -> String {
        String::from(match self {
            Self::Boolean => "boolean",
            Self::Number => "number",
            Self::String => "string",
        })
    }
}

#[derive(Debug)]
pub enum FieldType {
    /// A single property on the type, e.g. myField: i32
    Primitive(GbnfPrimitive),

    /// A complex property, with its own properties.
    Complex(GbnfType),

    /// A list/vec of primitive types.
    PrimitiveList(GbnfPrimitive),

    /// A list/vec of complex types.
    ComplexList(GbnfType),

    /// A single property field, but with limited values allowed,
    /// constrained by the primitive type.
    Limited(GbnfPrimitive),
}

#[derive(Debug)]
pub struct GbnfField {
    pub field_name: String,
    pub field_type: FieldType,
}

#[derive(Debug)]
pub struct GbnfType {
    pub name: String,
    pub fields: Vec<GbnfField>,
}

impl GbnfField {
    fn list_rule(field_type: &(impl TokenAndRule + ?Sized)) -> String {
        r#""[]" | "["   {SPACE}   {TYPE_NAME}   (","   {SPACE}   {TYPE_NAME})*   "]""#
            .replace("{LIST_NAME}", "")
            .replace("{SPACE}", &GbnfToken::Space.token())
            .replace("{TYPE_NAME}", &field_type.token())
    }

    fn list_rules<T: TokenAndRule>(&self, f: &T) -> Vec<GbnfRule> {
        // Create two rules: one for the list and on for its actual type.
        let list_rule = GbnfRule::new(self.token(), Self::list_rule(f));

        let mut rules = vec![list_rule];
        rules.append(&mut f.rules());
        rules
    }
}

impl TokenAndRule for GbnfField {
    fn token(&self) -> String {
        match &self.field_type {
            FieldType::Primitive(f) => f.token(),
            FieldType::PrimitiveList(f) => format!("{}_List", f.token()),
            FieldType::Complex(f) => f.token(),
            FieldType::ComplexList(f) => format!("{}_List", f.token()),
            FieldType::Limited(f) => f.token(),
            _ => "".to_string(),
        }
    }

    fn rules(&self) -> Vec<GbnfRule> {
        match &self.field_type {
            FieldType::ComplexList(f) => self.list_rules(f),
            FieldType::Complex(f) => f.rules(),
            FieldType::PrimitiveList(f) => self.list_rules(f),
            FieldType::Primitive(f) => f.rules(),
            FieldType::Limited(f) => f.rules(),

        }
    }
}

impl TokenAndRule for GbnfType {
    fn rules(&self) -> Vec<GbnfRule> {
        // This will output the full set of rules for the complex type.
        // Deduplication handled later.
        let mut rule = String::new();

        rule.push_str(r#""{  "#);

        let field_rules_text = self
            .fields
            .iter()
            .map(|field| {
                let mut text = String::new();
                text.push_str(&GbnfToken::Space.token());
                text.push_str("   ");
                text.push_str(&format!(
                    r#""\"{}\":"   {}  {}"#,
                    field.field_name,
                    GbnfToken::Space.token(),
                    field.token(),
                ));
                text
            })
            .join(r#"   ","   "#);

        rule.push_str(&field_rules_text);
        rule.push_str(r#"   "}""#);

        GbnfRule::single(self.token(), rule)
    }

    fn token(&self) -> String {
        self.name.clone()
    }
}

pub fn create_gbnf(gbnf_type: GbnfType) -> String {
    let mut rules = vec![GbnfRule::new("root".to_string(), gbnf_type.name.clone())];

    rules.append(&mut gbnf_type.rules());

    for field in gbnf_type.fields {
        rules.append(&mut field.rules());
    }

    rules
        .into_iter()
        .unique()
        .map(|rule| format!("{} ::= {}", rule.name, rule.text))
        .join("\n")
}
