mod header;
mod subscriber;

pub use rosty_msg::Message;
pub use subscriber::{Subscriber, PublisherConnectError};
use tokio::io::AsyncRead;
use std::io;
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::Cursor;
use tokio::io::AsyncReadExt;

/// Read a packet from a stream. A packet firstly consists of a u32 little endian encoded length
/// followed by the rest of the packet. The returned vector also includes this initial length.
async fn read_packet<U: AsyncRead + Unpin>(stream: &mut U) -> Result<Vec<u8>, io::Error> {
    let length = read_packet_size(stream).await?;
    let u32_size = std::mem::size_of::<u32>();
    let num_bytes = length as usize + u32_size;

    // Create a vector of the size of the packet
    let mut out = Vec::<u8>::with_capacity(num_bytes);
    unsafe { out.set_len(num_bytes); }

    // Write the length from the stream back into the packet
    std::io::Cursor::new(&mut out)
        .write_u32::<LittleEndian>(length)?;

    // Read the data from the stream
    stream.read_exact(&mut out[u32_size..]).await?;

    // Return the new, now full and "safely" initialized Vec.
    Ok(out)
}

/// Read the size of packet from a stream.
async fn read_packet_size<U: AsyncRead + Unpin>(stream: &mut U) -> Result<u32, io::Error> {
    let mut data = [0u8; 4];
    stream.read_exact(&mut data).await?;
    Ok(byteorder::ReadBytesExt::read_u32::<LittleEndian>(&mut Cursor::new(&data))?)
}