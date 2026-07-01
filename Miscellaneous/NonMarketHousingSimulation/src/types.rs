//! Data types shared across modules.

use geo_types::Geometry;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum PropertyValue {
    String(String),
    Float(f64),
    Int(i64),
    Uint(u64),
    #[allow(dead_code)]
    Bool(bool),
}

impl PropertyValue {
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Self::Float(f) => Some(*f as f32),
            Self::Int(i) => Some(*i as f32),
            Self::Uint(u) => Some(*u as f32),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        if let Self::String(s) = self {
            Some(s)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedFeature {
    pub geometry: Geometry<f32>,
    pub properties: HashMap<String, PropertyValue>,
}

impl ParsedFeature {
    pub fn prop_f32(&self, key: &str) -> Option<f32> {
        self.properties.get(key)?.as_f32()
    }
    pub fn prop_str(&self, key: &str) -> Option<&str> {
        self.properties.get(key)?.as_str()
    }
}

/// Convert a raw `mvt_reader::feature::Value` to `PropertyValue`.
pub fn convert_mvt_value(v: &mvt_reader::feature::Value) -> PropertyValue {
    use mvt_reader::feature::Value as V;
    match v {
        V::String(s) => PropertyValue::String(s.clone()),
        V::Float(f) => PropertyValue::Float(*f as f64),
        V::Double(f) => PropertyValue::Float(*f),
        V::Int(i) => PropertyValue::Int(*i),
        V::UInt(u) => PropertyValue::Uint(*u),
        V::SInt(s) => PropertyValue::Int(*s),
        V::Bool(b) => PropertyValue::Bool(*b),
        V::Null => PropertyValue::Bool(false),
    }
}
