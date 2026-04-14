use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct SilkInvokeArgs {
    pub command: String,
    pub args: Option<serde_json::Value>,
}
