use rosty_msg::RosMsg;
use std::collections::HashMap;
use std::io;
use std::io::Cursor;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

pub async fn read_and_decode<R: tokio::io::AsyncRead + Unpin>(
    stream: &mut R,
) -> Result<HashMap<String, String>, io::Error> {
    let mut data = vec![0u8; 4];

    // Read the size of the message
    stream.read_exact(data.as_mut_slice()).await?;
    let mut cursor = Cursor::new(&data);
    let data_size = u32::decode(&mut cursor)?;

    // Read the rest of the data
    data.resize(4 + data_size as usize, 0);
    stream.read_exact(&mut data.as_mut_slice()[4..]).await?;
    let mut cursor = Cursor::new(&data);

    // Decode the entire message
    RosMsg::decode(&mut cursor)
}

pub async fn encode_and_write<W: tokio::io::AsyncWrite + Unpin>(
    stream: &mut W,
    data: &HashMap<String, String>,
) -> Result<(), io::Error> {
    let mut writer = io::Cursor::new(Vec::with_capacity(128));
    // skip the first 4 bytes that will contain the message length
    writer.set_position(4);

    data.encode(&mut writer)?;

    // write the message length to the start of the header
    let message_length = (writer.position() - 4) as u32;
    writer.set_position(0);
    message_length.encode(&mut writer)?;

    let data = writer.into_inner();
    stream.write(&data).await;
    Ok(())
}

#[derive(Fail, Debug)]
pub enum InvalidHeaderError {
    #[fail(display = "missing field '{}'", 0)]
    MissingField(String),

    #[fail(display = "field mismatch, expected '{}', found '{}'", 0, 1)]
    FieldMismatch(String, String),
}

pub fn match_field(
    fields: &HashMap<String, String>,
    field: &str,
    expected: &str,
) -> Result<(), InvalidHeaderError> {
    let actual = match fields.get(field) {
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
