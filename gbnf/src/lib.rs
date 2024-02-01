extern crate proc_macro;

use itertools::Itertools;
use serde::de::DeserializeOwned;

pub mod prelude {
    pub use crate::gbnf_field;
    pub use crate::gbnf_field_type;
    pub use crate::AsGbnf;
    pub use crate::AsGrammar;
    pub use crate::GbnfComplex;
    pub use crate::GbnfField;
    pub use crate::GbnfFieldType;
    pub use crate::GbnfPrimitive;
    pub use crate::GbnfRule;
    pub use crate::GbnfToken;
}

// TODOs for this implementation:
//  1. Move primitive definitions (string, bool, etc) to the bottom of generated grammar.
//  2. Implement support for limited values.
//  3. Generate static strings for the gramma rules where possible.
//  4. Properly support optional types (right now they map to non-optional values).

// Converts GBNF defintions (through the types below) into the grammar
// rules.
pub trait AsGrammar {
    fn rules(&self) -> Vec<GbnfRule>;
    fn token(&self) -> String;
}

/// Trait for regular types to implement to convert themselves to a
/// GBNF value.
pub trait AsGbnf {
    fn to_gbnf() -> GbnfFieldType;
}

macro_rules! define_field_type {
    ($type:ty, $gbnf_type:expr) => {
        impl AsGbnf for $type {
            fn to_gbnf() -> GbnfFieldType {
                $gbnf_type
            }
        }
    };
}

macro_rules! define_array_blanket_impl {
    ($len:expr) => {
        impl<T> AsGbnf for [T; $len]
        where
            T: AsGbnf + DeserializeOwned,
        {
            fn to_gbnf() -> GbnfFieldType {
                use GbnfFieldType::*;
                match <T as AsGbnf>::to_gbnf() {
                    Primitive(primitive_type) => PrimitiveList(primitive_type),
                    OptionalPrimitive(primitive_type) => PrimitiveList(primitive_type),
                    Complex(complex_type) => ComplexList(complex_type),
                    OptionalComplex(complex_type) => ComplexList(complex_type),
                    Limited(_) => panic!("limited values are not yet supported"),
                    ComplexList(_) | PrimitiveList(_) => panic!("nested lists not supported"),
                }
            }
        }
    };
}

#[macro_export]
macro_rules! gbnf_field_type {
    ($type:ty) => {
        <$type as AsGbnf>::to_gbnf()
    };
}

#[macro_export]
macro_rules! gbnf_field {
    ($field_name:literal, $field_type:ty) => {
        GbnfField {
            field_name: $field_name.to_string(),
            field_type: gbnf_field_type!($field_type),
        }
    };
}

// Implemented field type mappings for common rust types.
define_field_type!(i16, GbnfFieldType::Primitive(GbnfPrimitive::Number));
define_field_type!(u16, GbnfFieldType::Primitive(GbnfPrimitive::Number));
define_field_type!(i32, GbnfFieldType::Primitive(GbnfPrimitive::Number));
define_field_type!(u32, GbnfFieldType::Primitive(GbnfPrimitive::Number));
define_field_type!(i64, GbnfFieldType::Primitive(GbnfPrimitive::Number));
define_field_type!(u64, GbnfFieldType::Primitive(GbnfPrimitive::Number));
define_field_type!(f32, GbnfFieldType::Primitive(GbnfPrimitive::Number));
define_field_type!(f64, GbnfFieldType::Primitive(GbnfPrimitive::Number));
define_field_type!(usize, GbnfFieldType::Primitive(GbnfPrimitive::Number));

define_field_type!(bool, GbnfFieldType::Primitive(GbnfPrimitive::Boolean));

define_field_type!(String, GbnfFieldType::Primitive(GbnfPrimitive::String));
define_field_type!(char, GbnfFieldType::Primitive(GbnfPrimitive::String));

// Macro-based blanket impls for arrays
define_array_blanket_impl!(1);
define_array_blanket_impl!(3);
define_array_blanket_impl!(4);
define_array_blanket_impl!(5);
define_array_blanket_impl!(6);
define_array_blanket_impl!(7);
define_array_blanket_impl!(8);
define_array_blanket_impl!(9);
define_array_blanket_impl!(10);
define_array_blanket_impl!(11);
define_array_blanket_impl!(12);
define_array_blanket_impl!(13);
define_array_blanket_impl!(14);
define_array_blanket_impl!(15);
define_array_blanket_impl!(16);

// Blanket implementations to cover more types
impl<T> AsGbnf for Vec<T>
where
    T: AsGbnf,
{
    fn to_gbnf() -> GbnfFieldType {
        use GbnfFieldType::*;
        match <T as AsGbnf>::to_gbnf() {
            Primitive(primitive_type) => PrimitiveList(primitive_type),
            OptionalPrimitive(primitive_type) => PrimitiveList(primitive_type),
            Complex(complex_type) => ComplexList(complex_type),
            OptionalComplex(complex_type) => ComplexList(complex_type),
            Limited(_) => panic!("limited values not yet supported"),
            ComplexList(_) | PrimitiveList(_) => panic!("nested lists not supported"),
        }
    }
}

impl<T> AsGbnf for Option<T>
where
    T: AsGbnf,
{
    fn to_gbnf() -> GbnfFieldType {
        use GbnfFieldType::*;
        match <T as AsGbnf>::to_gbnf() {
            Primitive(primitive_type) => OptionalPrimitive(primitive_type),
            Complex(complex_type) => OptionalComplex(complex_type),
            OptionalPrimitive(_) | OptionalComplex(_) => panic!("nested options are not allowed"),
            Limited(_) => panic!("limited values not yet supported"),
            _ => panic!("optional type cannot be a list"),
        }
    }
}

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

/// Tokens in the GBNF rule.
pub enum GbnfToken {
    Space,
}

impl GbnfToken {
    pub(self) const SPACE: &'static str = r#"[ \t\n]*"#;
}

impl AsGrammar for GbnfToken {
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

/// Represents a primitive value in the GBNF, the simplest possible
/// value a type can hold.
#[derive(Debug)]
pub enum GbnfPrimitive {
    String,
    Boolean,
    Number,
}

impl GbnfPrimitive {
    pub(self) const STRING: &'static str = r#""\""   ([^"]*)   "\"""#;
    pub(self) const BOOLEAN: &'static str = r#""true" | "false""#;
    pub(self) const NUMBER: &'static str = r#"[0-9]+   "."?   [0-9]*"#;
}

impl AsGrammar for GbnfPrimitive {
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

/// Categorize all types of fields that the generated grammar can
/// handle.
#[derive(Debug)]
pub enum GbnfFieldType {
    /// A single property on the type, e.g. myField: i32
    Primitive(GbnfPrimitive),

    /// Can be a value or null.
    OptionalPrimitive(GbnfPrimitive),

    /// A list/vec of primitive types.
    PrimitiveList(GbnfPrimitive),

    /// A complex type, with its own properties.
    Complex(GbnfComplex),

    /// Can be a value or null.
    OptionalComplex(GbnfComplex),

    /// A list/vec of complex types.
    ComplexList(GbnfComplex),

    /// A single property field, but with limited values allowed,
    /// constrained by the primitive type.
    Limited(GbnfPrimitive),
}

impl GbnfFieldType {
    pub fn as_complex(self) -> GbnfComplex {
        match self {
            GbnfFieldType::Complex(complex) => complex,
            _ => panic!("Not a GBNF complex type"),
        }
    }
}

/// Connect a property name and a field type to generate a GBNF rule.
#[derive(Debug)]
pub struct GbnfField {
    pub field_name: String,
    pub field_type: GbnfFieldType,
}

impl GbnfField {
    fn list_rule(field_type: &(impl AsGrammar + ?Sized)) -> String {
        r#""[]" | "["   {SPACE}   {TYPE_NAME}   (","   {SPACE}   {TYPE_NAME})*   "]""#
            .replace("{LIST_NAME}", "")
            .replace("{SPACE}", &GbnfToken::Space.token())
            .replace("{TYPE_NAME}", &field_type.token())
    }

    fn list_rules<T: AsGrammar>(&self, f: &T) -> Vec<GbnfRule> {
        // Create two rules: one for the list and on for its actual type.
        let list_rule = GbnfRule::new(self.token(), Self::list_rule(f));

        let mut rules = vec![list_rule];
        rules.append(&mut f.rules());
        rules
    }
}

impl AsGrammar for GbnfField {
    fn token(&self) -> String {
        match &self.field_type {
            GbnfFieldType::Primitive(f) => f.token(),
            GbnfFieldType::OptionalPrimitive(f) => f.token(),
            GbnfFieldType::PrimitiveList(f) => format!("{}List", f.token()),
            GbnfFieldType::Complex(f) => f.token(),
            GbnfFieldType::OptionalComplex(f) => f.token(),
            GbnfFieldType::ComplexList(f) => format!("{}List", f.token()),
            GbnfFieldType::Limited(f) => f.token(),
            _ => "".to_string(),
        }
    }

    // TODO need to implement optional rules, which probably involves
    // wrapping the primitive rule in parens, and then ORing to null.
    fn rules(&self) -> Vec<GbnfRule> {
        match &self.field_type {
            GbnfFieldType::Complex(f) => f.rules(),
            GbnfFieldType::OptionalComplex(f) => f.rules(),
            GbnfFieldType::ComplexList(f) => self.list_rules(f),
            GbnfFieldType::Primitive(f) => f.rules(),
            GbnfFieldType::OptionalPrimitive(f) => f.rules(),
            GbnfFieldType::PrimitiveList(f) => self.list_rules(f),
            GbnfFieldType::Limited(f) => f.rules(),
        }
    }
}

/// The complex type is a direct mapping from a supported Rust struct,
/// and also used to generate the root of a GBNF grammar.
#[derive(Debug)]
pub struct GbnfComplex {
    pub name: String,
    pub fields: Vec<GbnfField>,
}

impl GbnfComplex {
    pub fn to_grammar(&self) -> String {
        let mut rules = vec![GbnfRule::new("root".to_string(), self.name.clone())];

        rules.append(&mut self.rules());

        for field in &self.fields {
            rules.append(&mut field.rules());
        }

        rules
            .into_iter()
            .unique()
            .map(|rule| format!("{} ::= {}", rule.name, rule.text))
            .join("\n")
    }
}

impl AsGrammar for GbnfComplex {
    fn rules(&self) -> Vec<GbnfRule> {
        // This will output the full set of rules for the complex type.
        // Deduplication handled later.
        let mut rule = String::new();

        rule.push_str(r#""{"  "#);

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

        let mut rules = GbnfRule::single(self.token(), rule);
        rules.append(&mut GbnfToken::Space.rules());
        rules
    }

    fn token(&self) -> String {
        self.name.clone()
    }
}
