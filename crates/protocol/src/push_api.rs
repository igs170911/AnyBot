use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct PushServerKey {
    #[cfg_attr(
        feature = "schema",
        schema(
            example = "BNcRdreALRFXTkOOUHK1EtK2wtaz5Ry4YfYCA_0QTpQtUbVlUls0VJXg7A8u-Ts1XbjhazAkj7I99e8QcYP7DkM"
        )
    )]
    pub public_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct PushSubscriptionKeys {
    #[cfg_attr(
        feature = "schema",
        schema(
            example = "BLMbF9ffKBiWQLCKvTHb6LO8Nb6dcUh6TItC455vu2kElga6PQvUmaFyCdykxY2nOSSL3yKgfbmFLRTUaGv4yV8"
        )
    )]
    pub p256dh: String,
    #[cfg_attr(feature = "schema", schema(example = "xS03Fi5ErfTNH_l9WHE9Ig"))]
    pub auth: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct PushSubscribeRequest {
    #[cfg_attr(
        feature = "schema",
        schema(example = "https://push.example.org/send/abc123")
    )]
    pub endpoint: String,
    pub keys: PushSubscriptionKeys,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct PushSubscribeReply {
    #[cfg_attr(feature = "schema", schema(example = "018f-push-uuidv7"))]
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct PushMineReply {
    #[cfg_attr(feature = "schema", schema(example = true))]
    pub subscribed: bool,
}
