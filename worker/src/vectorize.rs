use std::collections::HashMap;

use crate::{send::SendFuture, EnvBinding, Result};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use worker_sys::types::Vectorize as VectorizeSys;

/// Supported distance metrics for an index.
/// Distance metrics determine how other "similar" vectors are determined.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VectorizeDistanceMetric {
    Euclidean,
    Cosine,
    DotProduct,
}

/// Information about the configuration of an index.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum VectorizeIndexConfig {
    Preset {
        preset: String,
    },
    Custom {
        dimensions: u16,
        metric: VectorizeDistanceMetric,
    },
}

/// Metadata about an existing index.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorizeIndexInfo {
    /// The number of records containing vectors within the index.
    pub vector_count: u64,
    /// Number of dimensions the index has been configured for.
    pub dimensions: u32,
    /// ISO 8601 datetime of the last processed mutation on in the index. All changes before this mutation will be reflected in the index state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processed_up_to_datetime: Option<String>,
    /// UUIDv4 of the last mutation processed by the index. All changes before this mutation will be reflected in the index state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processed_up_to_mutation: Option<String>,
}

/// Results of an operation that performed a mutation on a set of vectors.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorizeVectorAsyncMutation {
    /// The unique identifier for the async mutation operation containing the changeset.
    pub mutation_id: String,
}

/// Represents a single vector value set along with its associated metadata.
#[derive(Debug, Serialize)]
pub struct VectorizeVector<'a> {
    /// The ID for the vector. This can be user-defined, and must be unique. It should uniquely identify the object, and is best set based on the ID of what the vector represents.
    id: String,
    /// The vector values.
    values: &'a [f32],
    /// The namespace this vector belongs to.
    namespace: Option<String>,
    /// Metadata associated with the vector. Includes the values of other fields and potentially additional details.
    metadata: serde_json::Map<String, serde_json::Value>,
}

impl<'a> VectorizeVector<'a> {
    pub fn new(id: &str, values: &'a [f32]) -> Self {
        Self {
            id: id.to_owned(),
            values,
            namespace: None,
            metadata: serde_json::Map::new(),
        }
    }

    pub fn with_namespace(mut self, namespace: String) -> Self {
        self.namespace = Some(namespace);
        self
    }

    pub fn with_metadata_entry<V: Serialize>(mut self, key: &str, value: V) -> Result<Self> {
        self.metadata
            .insert(key.to_owned(), serde_json::to_value(value)?);
        Ok(self)
    }
}

/// Metadata return levels for a Vectorize query.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum VectorizeMetadataRetrievalLevel {
    /// Full metadata for the vector return set, including all fields (including those un-indexed) without truncation. This is a more expensive retrieval, as it requires additional fetching & reading of un-indexed data.
    All,
    /// Return all metadata fields configured for indexing in the vector return set. This level of retrieval is "free" in that no additional overhead is incurred returning this data. However, note that indexed metadata is subject to truncation (especially for larger strings).
    Indexed,
    /// No indexed metadata will be returned.
    #[default]
    None,
}

/// Options for metadata filtering.
#[derive(Debug, Serialize, Hash, PartialEq, Eq)]
pub enum VectorizeVectorMetadataFilterOp {
    #[serde(rename = "$eq")]
    Eq,
    #[serde(rename = "$ne")]
    Neq,
    #[serde(rename = "$in")]
    In,
    #[serde(rename = "$nin")]
    NotIn,
    #[serde(rename = "$lt")]
    Lt,
    #[serde(rename = "$lte")]
    Lte,
    #[serde(rename = "$gt")]
    Gt,
    #[serde(rename = "$gte")]
    Gte,
}

/// Filter criteria for vector metadata used to limit the retrieved query result set.
type VectorizeVectorMetadataFilter = HashMap<String, HashMap<String, serde_json::Value>>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorizeQueryOptions {
    /// Value between 1 and 100, default `5`
    top_k: u8,
    /// Return vectors from the specified namespace. Default `None`.
    namespace: Option<String>,
    /// Return vector values. Default `False`.
    return_values: bool,
    /// Return vector metadata. Default `None`.
    return_metadata: VectorizeMetadataRetrievalLevel,
    /// Default `None`.
    filter: Option<VectorizeVectorMetadataFilter>,
}

impl VectorizeQueryOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_top_k(mut self, top_k: u8) -> Self {
        self.top_k = top_k;
        self
    }

    pub fn with_namespace(mut self, namespace: &str) -> Self {
        self.namespace = Some(namespace.to_owned());
        self
    }

    pub fn with_return_values(mut self, return_values: bool) -> Self {
        self.return_values = return_values;
        self
    }

    pub fn with_return_metadata(
        mut self,
        return_metadata: VectorizeMetadataRetrievalLevel,
    ) -> Self {
        self.return_metadata = return_metadata;
        self
    }

    pub fn with_filter_entry<T: Serialize>(
        mut self,
        key: &str,
        op: VectorizeVectorMetadataFilterOp,
        value: T,
    ) -> Result<Self> {
        let mut filter = self.filter.unwrap_or_default();
        let inner = filter.entry(key.to_owned()).or_default();
        let op_str = serde_json::to_string(&op)?.trim_matches('"').to_string();
        inner.insert(op_str, serde_json::to_value(value)?);
        self.filter = Some(filter);
        Ok(self)
    }
}

impl Default for VectorizeQueryOptions {
    fn default() -> Self {
        Self {
            top_k: 5,
            namespace: None,
            return_values: false,
            return_metadata: VectorizeMetadataRetrievalLevel::None,
            filter: None,
        }
    }
}

/// Represents a single vector value set along with its associated metadata.
#[derive(Debug, Deserialize)]
pub struct VectorizeVectorResult {
    /// The ID for the vector. This can be user-defined, and must be unique. It should uniquely identify the object, and is best set based on the ID of what the vector represents.
    pub id: String,
    /// The vector values.
    pub values: Option<Vec<f32>>,
    /// Metadata associated with the vector. Includes the values of other fields and potentially additional details.
    pub metadata: Option<serde_json::Map<String, serde_json::Value>>,
    /// The namespace the vector belongs to.
    pub namespace: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VectorizeMatchVector {
    #[serde(flatten)]
    pub vector: VectorizeVectorResult,
    /// The score or rank for similarity, when returned as a result
    pub score: Option<f64>,
}

/// A set of matching [VectorizeMatchVector] for a particular query.
#[derive(Debug, Deserialize)]
pub struct VectorizeMatches {
    pub matches: Vec<VectorizeMatchVector>,
    pub count: u64,
}

pub struct Vectorize(VectorizeSys);

unsafe impl Send for Vectorize {}
unsafe impl Sync for Vectorize {}

impl EnvBinding for Vectorize {
    const TYPE_NAME: &'static str = "VectorizeIndexImpl";
}

impl Vectorize {
    /// Inserts vectors into an index.
    /// NOTE: This is asynchronous, and returns a mutation ID.
    pub async fn insert<'a>(
        &self,
        vectors: &[VectorizeVector<'a>],
    ) -> Result<VectorizeVectorAsyncMutation> {
        let vectors_js = to_js_value_safe(&vectors)?;
        let promise = self.0.insert(vectors_js.into())?;
        let fut = SendFuture::new(JsFuture::from(promise));
        let mutation = fut.await?;
        Ok(serde_wasm_bindgen::from_value(mutation)?)
    }

    /// Upserts vectors into an index.
    /// NOTE: This is asynchronous, and returns a mutation ID.
    pub async fn upsert<'a>(
        &self,
        vectors: &[VectorizeVector<'a>],
    ) -> Result<VectorizeVectorAsyncMutation> {
        let vectors_js = to_js_value_safe(&vectors)?;
        let promise = self.0.upsert(vectors_js.into())?;
        let fut = SendFuture::new(JsFuture::from(promise));
        let mutation = fut.await?;
        Ok(serde_wasm_bindgen::from_value(mutation)?)
    }

    /// Query an index with the provided vector, returning the score(s) of the closest vectors based on the configured distance metric.
    pub async fn query(
        &self,
        vector: JsValue,
        options: VectorizeQueryOptions,
    ) -> Result<VectorizeMatches> {
        let opts = to_js_value_safe(&options)?;
        let promise = self.0.query(vector, opts.into())?;
        let fut = SendFuture::new(JsFuture::from(promise));
        let matches = fut.await?;
        Ok(serde_wasm_bindgen::from_value(matches)?)
    }

    /// Query an index using a vector that is already present in the index.
    pub async fn query_by_id<'a, T>(
        &self,
        id: T,
        options: VectorizeQueryOptions,
    ) -> Result<VectorizeMatches>
    where
        T: IntoIterator<Item = &'a str>,
    {
        let opts = to_js_value_safe(&options)?;
        let ids: Vec<String> = id.into_iter().map(|id| id.to_string()).collect();
        let arg = serde_wasm_bindgen::to_value(&ids)?;
        let promise = self.0.query_by_id(arg, opts.into())?;
        let fut = SendFuture::new(JsFuture::from(promise));
        let vectors = fut.await?;
        Ok(serde_wasm_bindgen::from_value(vectors)?)
    }

    /// Retrieves the specified vectors by their ID, including values and metadata.
    pub async fn get_by_ids<'a, T>(&self, ids: T) -> Result<Vec<VectorizeVectorResult>>
    where
        T: IntoIterator<Item = &'a str>,
    {
        let ids: Vec<String> = ids.into_iter().map(|id| id.to_string()).collect();
        let arg = serde_wasm_bindgen::to_value(&ids)?;
        let promise = self.0.get_by_ids(arg)?;
        let fut = SendFuture::new(JsFuture::from(promise));
        let vectors = fut.await?;
        Ok(serde_wasm_bindgen::from_value(vectors)?)
    }

    /// Deletes the vector IDs provided from the current index.
    pub async fn delete_by_ids<'a, T>(&self, ids: T) -> Result<VectorizeVectorAsyncMutation>
    where
        T: IntoIterator<Item = &'a str>,
    {
        // TODO: Can we avoid this allocation?
        let ids: Vec<String> = ids.into_iter().map(|id| id.to_string()).collect();
        let arg = serde_wasm_bindgen::to_value(&ids)?;
        let promise = self.0.delete_by_ids(arg)?;
        let fut = SendFuture::new(JsFuture::from(promise));
        let mutation = fut.await?;
        Ok(serde_wasm_bindgen::from_value(mutation)?)
    }

    /// Retrieves the configuration of a given index directly, including its configured `dimensions` and distance `metric`.
    pub async fn describe(&self) -> Result<VectorizeIndexInfo> {
        let promise = self.0.describe()?;
        let fut = SendFuture::new(JsFuture::from(promise));
        let details = fut.await?;
        Ok(serde_wasm_bindgen::from_value(details)?)
    }
}

impl JsCast for Vectorize {
    fn instanceof(val: &JsValue) -> bool {
        val.is_instance_of::<Vectorize>()
    }

    fn unchecked_from_js(val: JsValue) -> Self {
        Self(val.into())
    }

    fn unchecked_from_js_ref(val: &JsValue) -> &Self {
        unsafe { &*(val as *const JsValue as *const Self) }
    }
}

impl From<Vectorize> for JsValue {
    fn from(index: Vectorize) -> Self {
        JsValue::from(index.0)
    }
}

impl AsRef<JsValue> for Vectorize {
    fn as_ref(&self) -> &JsValue {
        &self.0
    }
}

/// Utility function to convert serde_json::Value to JsValue properly
fn json_value_to_js_value(value: &serde_json::Value) -> Result<JsValue> {
    use js_sys::{Array, Object};

    match value {
        serde_json::Value::Null => Ok(JsValue::NULL),
        serde_json::Value::Bool(b) => Ok((*b).into()),
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                Ok(f.into())
            } else {
                Ok(n.to_string().into())
            }
        }
        serde_json::Value::String(s) => Ok(s.into()),
        serde_json::Value::Array(arr) => {
            let js_array = Array::new();
            for (i, item) in arr.iter().enumerate() {
                let js_item = json_value_to_js_value(item)?;
                js_array.set(i as u32, js_item);
            }
            Ok(js_array.into())
        }
        serde_json::Value::Object(obj) => {
            let js_obj = Object::new();
            for (key, val) in obj {
                let js_val = json_value_to_js_value(val)?;
                js_sys::Reflect::set(&js_obj, &key.into(), &js_val)?;
            }
            Ok(js_obj.into())
        }
    }
}

/// Utility function to convert serde_json::Value to JsValue properly
/// NOTE: This is a workaround for the issue found at https://github.com/RReverser/serde-wasm-bindgen/issues/10
/// which is preventing us from sending something like `{"metadata": {"foo": "bar"}}` as a query option.
fn to_js_value_safe<T: Serialize>(value: &T) -> Result<JsValue> {
    let json_str = serde_json::to_string(value)?;
    let json_value: serde_json::Value = serde_json::from_str(&json_str)?;
    json_value_to_js_value(&json_value)
}
