#![cfg(unix)]

use libloading::os;
use std::net::TcpListener;

#[allow(dead_code)]
const SD_LISTEN_FDS_START: i32 = 3;

#[derive(Debug)]
pub enum Error {
	#[cfg(feature = "dlopen")]
	LibLoading(libloading::Error),
	#[cfg(feature = "dlopen")]
	LibLoadingFailedToLoadSystemd(String),
	Systemd(std::io::Error),
}

#[cfg(feature = "dlopen")]
impl From<libloading::Error> for Error {
	fn from(e: libloading::Error) -> Self {
		Error::LibLoading(e)
	}
}

#[cfg(not(any(feature = "dlopen", feature = "dynlink")))]
pub fn systemd_socket_activation() -> Result<Vec<TcpListener>, Error> {
	unimplemented!(
		"Enable either \"dlopen\" or \"dynlink\" or \"env-var\" feature to use this crate"
	)
}

#[cfg(feature = "dlopen")]
pub fn systemd_socket_activation() -> Result<Vec<TcpListener>, Error> {
	use std::os::unix::prelude::FromRawFd;

	type SdListenFdsFunc = unsafe extern "C" fn(unset_environment: i32) -> i32;

	const SD_LISTEN_FDS_FUNC_NAME: &[u8] = b"sd_listen_fds\0";

	let nfds = unsafe {
		let systemd_lib = libloading::Library::new("libsystemd.so.0").map_err(|err| match err {
			dlopen_err @ libloading::Error::DlOpen { .. } => {
				Error::LibLoadingFailedToLoadSystemd(dlopen_err.to_string())
			}
			e => Error::LibLoading(e),
		})?;

		let sd_listen_fds: libloading::Symbol<SdListenFdsFunc> =
			systemd_lib.get(SD_LISTEN_FDS_FUNC_NAME)?;

		sd_listen_fds(false as i32)
	};

	if nfds < 0 {
		return Err(Error::Systemd(std::io::Error::from_raw_os_error(nfds)));
	}

	let listeners: Vec<TcpListener> = (SD_LISTEN_FDS_START..(SD_LISTEN_FDS_START + nfds))
		.map(|fd| unsafe { TcpListener::from_raw_fd(fd) })
		.collect();

	Ok(listeners)
}

#[cfg(feature = "dynlink")]
pub fn systemd_socket_activation() -> Result<Vec<TcpListener>, Error> {
	use std::os::unix::prelude::FromRawFd;

	#[link(name = "systemd")]
	extern "C" {
		fn sd_listen_fds(unset_environment: i32) -> i32;
	}

	let nfds = unsafe { sd_listen_fds(false as i32) };

	if nfds < 0 {
		return Err(Error::Systemd(std::io::Error::from_raw_os_error(nfds)));
	}

	let listeners: Vec<TcpListener> = (SD_LISTEN_FDS_START..(SD_LISTEN_FDS_START + nfds))
		.map(|fd| unsafe { TcpListener::from_raw_fd(fd) })
		.collect();

	Ok(listeners)
}

#[cfg(feature = "env-var")]
pub fn systemd_socket_activation() -> Result<Vec<TcpListener>, Error> {
	use std::os::unix::prelude::FromRawFd;

	let nfds = std::env::var("LISTEN_FDS")
		.unwrap_or_else(|| String::from("0"))
		.parse::<u64>()
		.unwrap_or(0);

	let listeners: Vec<TcpListener> = (SD_LISTEN_FDS_START..(SD_LISTEN_FDS_START + nfds))
		.map(|fd| unsafe { TcpListener::from_raw_fd(fd) })
		.collect();

	Ok(listeners)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn does_it_work() {
		assert!(systemd_socket_activation().is_ok());
	}
}
