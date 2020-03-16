use super::read_packet;
use rosty_msg::RosMsg;
use std::collections::HashMap;
use std::io;
use std::io::Cursor;
use tokio::io::AsyncWriteExt;

#[derive(Fail, Debug)]
pub enum InvalidHeaderError {
    #[fail(display = "missing field '{}'", 0)]
    MissingField(String),

    #[fail(display = "field mismatch, expected '{}', found '{}'", 0, 1)]
    FieldMismatch(String, String),
}

/// Reads from the given async `stream`, decodes the expected header and returns the result.
pub async fn read_and_decode<R: tokio::io::AsyncRead + Unpin>(
    stream: &mut R,
) -> Result<HashMap<String, String>, io::Error> {
    let package = read_packet(stream).await?;
    RosMsg::decode(&mut Cursor::new(&package))
}

/// Encode the specified `header` into bytes and async write it to the given `stream`.
pub async fn encode_and_write<W: tokio::io::AsyncWrite + Unpin>(
    stream: &mut W,
    header: &HashMap<String, String>,
) -> Result<(), io::Error> {
    let mut writer = io::Cursor::new(Vec::with_capacity(128));
    header.encode(&mut writer)?;
    let data = writer.into_inner();
    stream.write(&data).await?;
    Ok(())
}

/// Given a `header` ensure that one of its `field`s it set to a specific `expected` value.
pub fn match_field(
    header: &HashMap<String, String>,
    field: &str,
    expected: &str,
) -> Result<(), InvalidHeaderError> {
    let actual = match header.get(field) {
        Some(actual) => actual,
        None => return Err(InvalidHeaderError::MissingField(field.to_owned())),
    };
    if actual != expected {
        Err(InvalidHeaderError::FieldMismatch(
            expected.to_owned(),
            actual.to_owned(),
        ))
    } else {
        Ok(())
    }
}
