use rosty_msg::RosMsg;
use std::collections::HashMap;
use std::io;
use tokio::io::AsyncWriteExt;

// pub async fn decode<R: std::io::Read>(data: &mut R) -> Result<HashMap<String, String>, io::Error> {
//     RosMsg::decode(data)
// }

pub async fn encode_and_write<W: tokio::io::AsyncWrite + Unpin>(
    writer: &mut W,
    data: &HashMap<String, String>,
) -> Result<(), io::Error> {
    let data = data.encode_vec()?;
    writer.write(&data).await?;
    Ok(())
}

pub fn match_field(
    fields: &HashMap<String, String>,
    field: &str,
    expected: &str,
) -> Result<(), failure::Error> {
    let actual = match fields.get(field) {
        Some(actual) => actual,
        None => failure::bail!("missing field {}", field),
    };
    if actual != expected {
        failure::bail!("header mismatch expected '{}', got '{}'", expected, actual);
    }
    Ok(())
}
