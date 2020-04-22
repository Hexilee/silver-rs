mod file;

use crate::task::spawn_blocking;
use std::fs;
use std::io;
use std::path::Path;

pub use file::File;

/// Read the entire contents of a file into a bytes vector.
///
/// This is a convenience function for using [`File::open`] and [`read_to_end`]
/// with fewer imports and without an intermediate variable. It pre-allocates a
/// buffer based on the file size when available, so it is generally faster than
/// reading into a vector created with `Vec::new()`.
///
/// [`File::open`]: struct.File.html#method.open
/// [`read_to_end`]: ../io/trait.Read.html#method.read_to_end
///
/// # Errors
///
/// This function will return an error if `path` does not already exist.
/// Other errors may also be returned according to [`OpenOptions::open`].
///
/// [`OpenOptions::open`]: struct.OpenOptions.html#method.open
///
/// It will also return an error if it encounters while reading an error
/// of a kind other than [`ErrorKind::Interrupted`].
///
/// [`ErrorKind::Interrupted`]: ../../std/io/enum.ErrorKind.html#variant.Interrupted
///
/// # Examples
///
/// ```no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error + 'static>> { tio::task::block_on( async {
/// #
/// use tio::fs;
/// use std::net::SocketAddr;
///
/// let contents = fs::read("address.txt").await?;
/// let foo: SocketAddr = String::from_utf8_lossy(&contents).parse()?;
/// # Ok(()) })
/// # }
/// ```
pub async fn read<P>(path: P) -> io::Result<Vec<u8>>
where
    P: 'static + Send + AsRef<Path>,
{
    spawn_blocking(move || fs::read(path)).await
}

/// Read the entire contents of a file into a string.
///
/// This is a convenience function for using [`File::open`] and [`read_to_string`]
/// with fewer imports and without an intermediate variable. It pre-allocates a
/// buffer based on the file size when available, so it is generally faster than
/// reading into a string created with `String::new()`.
///
/// [`File::open`]: struct.File.html#method.open
/// [`read_to_string`]: ../io/trait.Read.html#method.read_to_string
///
/// # Errors
///
/// This function will return an error if `path` does not already exist.
/// Other errors may also be returned according to [`OpenOptions::open`].
///
/// [`OpenOptions::open`]: struct.OpenOptions.html#method.open
///
/// It will also return an error if it encounters while reading an error
/// of a kind other than [`ErrorKind::Interrupted`],
/// or if the contents of the file are not valid UTF-8.
///
/// [`ErrorKind::Interrupted`]: ../../std/io/enum.ErrorKind.html#variant.Interrupted
///
/// # Examples
///
/// ```no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error + 'static>> { tio::task::block_on( async {
/// #
/// use tio::fs;
/// use std::net::SocketAddr;
///
/// let contents = fs::read_to_string("address.txt").await?;
/// let foo: SocketAddr = contents.parse()?;
/// # Ok(()) })
/// # }
/// ```
pub async fn read_to_string<P>(path: P) -> io::Result<String>
where
    P: 'static + Send + AsRef<Path>,
{
    spawn_blocking(move || fs::read_to_string(path)).await
}

/// Write a slice as the entire contents of a file.
///
/// This function will create a file if it does not exist,
/// and will entirely replace its contents if it does.
///
/// # Examples
///
/// ```no_run
///
/// # fn main() -> Result<(), Box<dyn std::error::Error + 'static>> { tio::task::block_on( async {
/// #
/// use tio::fs;
/// fs::write("foo.txt", b"Lorem ipsum").await?;
/// fs::write("bar.txt", "dolor sit").await?;
/// # Ok(()) })
/// # }
/// ```
pub async fn write<P, C>(path: P, contents: C) -> io::Result<()>
where
    P: 'static + Send + AsRef<Path>,
    C: 'static + Send + AsRef<[u8]>,
{
    spawn_blocking(move || fs::write(path, contents)).await
}
